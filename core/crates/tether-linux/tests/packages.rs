//! Base package setup tests.

mod fixtures;

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, atomic::AtomicUsize};

use ed25519_dalek::{Signer, SigningKey};
use flate2::{Compression, write::GzEncoder};
use serde_json::json;
use sha2::{Digest, Sha256};
use tar::{Builder, EntryType, Header};
use tempfile::tempdir;
use tether_linux::{
    LinuxError, NoopProgress, ProotConfig, RootfsFetcher, RootfsImage, RootfsManager, RootfsStatus,
    SessionKind, SetupRequest, setup,
};
use tokio_util::sync::CancellationToken;

fn image(bytes: &[u8]) -> RootfsImage {
    RootfsImage {
        arch: "x86_64".into(),
        url: "https://example.invalid/image".into(),
        sha256: hex(&Sha256::digest(bytes)),
        size_bytes: bytes.len() as u64,
        max_extracted_bytes: 1024,
    }
}

fn signed_manifest(image: &RootfsImage) -> (Vec<u8>, [u8; 64], [u8; 32]) {
    let key = SigningKey::from_bytes(&[8_u8; 32]);
    let bytes = serde_json::to_vec(&json!({
        "format": 1,
        "alpine_version": "3.20",
        "images": [{
            "arch": image.arch,
            "url": image.url,
            "sha256": image.sha256,
            "size_bytes": image.size_bytes,
            "max_extracted_bytes": image.max_extracted_bytes
        }],
        "base_packages": ["busybox"]
    }))
    .unwrap();
    (
        bytes.clone(),
        key.sign(&bytes).to_bytes(),
        key.verifying_key().to_bytes(),
    )
}

fn install_rootfs(manager: &RootfsManager, image: &RootfsImage, bytes: Vec<u8>) {
    let fetcher = fixtures::BytesFetcher {
        bytes: Arc::new(bytes),
        calls: Arc::new(AtomicUsize::new(0)),
    };
    manager
        .install(
            image,
            &fetcher,
            &mut NoopProgress,
            &CancellationToken::new(),
        )
        .unwrap();
}

fn apk_script(root: &std::path::Path, failing: bool) {
    let body = if failing {
        "#!/bin/sh\nprintf 'head\\n' >&2\ni=0\nwhile [ \"$i\" -lt 5000 ]; do printf x >&2; i=$((i + 1)); done\nprintf 'TAIL-FAILURE\\n' >&2\nexit 7\n"
    } else {
        "#!/bin/sh\nif [ -n \"$http_proxy\" ] || [ -n \"$https_proxy\" ]; then exit 10; fi\nprintf network-on > \"$PWD/apk-network\"\nexit 0\n"
    };
    write_apk(root, body);
}

fn write_apk(root: &std::path::Path, body: &str) {
    let path = root.join("rootfs/alpine/usr/bin/apk");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

/// Writes a fake `apk` that edits an indexed file and adds a new file, as a real install does.
fn apk_rewriting_packages(root: &std::path::Path) {
    let rootfs = root.join("rootfs/alpine");
    let body = format!(
        "#!/bin/sh\nprintf 'ssh\\n' >> '{shells}'\nprintf bash > '{bash}'\nexit 0\n",
        shells = rootfs.join("etc/shells").display(),
        bash = rootfs.join("bin/bash").display(),
    );
    write_apk(root, &body);
}

/// Archive with two indexed files: `bin/busybox` and `etc/shells`.
fn two_file_archive() -> Vec<u8> {
    let mut compressed = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut builder = Builder::new(&mut compressed);
        append_entry(&mut builder, "bin", EntryType::dir(), &[]);
        append_entry(&mut builder, "bin/busybox", EntryType::file(), b"busybox");
        append_entry(&mut builder, "etc", EntryType::dir(), &[]);
        append_entry(&mut builder, "etc/shells", EntryType::file(), b"/bin/sh\n");
        builder.finish().unwrap();
    }
    compressed.finish().unwrap()
}

fn append_entry<W: Write>(builder: &mut Builder<W>, path: &str, kind: EntryType, bytes: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_path(path).unwrap();
    header.set_entry_type(kind);
    header.set_size(bytes.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    builder.append(&header, bytes).unwrap();
}

fn empty_fetcher() -> fixtures::BytesFetcher {
    fixtures::BytesFetcher {
        bytes: Arc::new(Vec::new()),
        calls: Arc::new(AtomicUsize::new(0)),
    }
}

/// Runs `setup` with the signed manifest for `image` and the test fetcher.
fn run_setup(
    manager: &RootfsManager,
    image: &RootfsImage,
    config: &ProotConfig,
) -> Result<(), LinuxError> {
    let (manifest, signature, key) = signed_manifest(image);
    let fetcher = empty_fetcher();
    let token = CancellationToken::new();
    let mut progress = NoopProgress;
    let mut parts = RequestParts {
        manifest: &manifest,
        signature: &signature,
        key: &key,
        image,
        config,
        fetcher: &fetcher,
        progress: &mut progress,
        cancel: &token,
    };
    let mut request = request(&mut parts);
    setup(manager, &mut request).map(|_manifest| ())
}

#[test]
fn setup_is_ready_after_packages_rewrite_indexed_files() {
    let folder = tempdir().unwrap();
    let bytes = two_file_archive();
    let image = image(&bytes);
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let linux = folder.path().join("linux");
    let proot = fixtures::fake_proot(folder.path(), &workspace);
    let config = fixtures::config(&linux, Some(workspace.clone()), proot);
    let manager = RootfsManager::new(&linux);
    install_rootfs(&manager, &image, bytes);
    apk_rewriting_packages(&linux);

    run_setup(&manager, &image, &config).unwrap();

    assert_eq!(
        manager.status().unwrap(),
        RootfsStatus::Ready {
            alpine_version: "3.20".into()
        }
    );
    assert_eq!(
        fs::read(linux.join("rootfs/alpine/etc/shells")).unwrap(),
        b"/bin/sh\nssh\n"
    );
    assert!(linux.join("rootfs/alpine/bin/bash").is_file());
}

#[test]
fn a_deleted_original_file_is_still_reported_after_setup() {
    let folder = tempdir().unwrap();
    let bytes = two_file_archive();
    let image = image(&bytes);
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let linux = folder.path().join("linux");
    let proot = fixtures::fake_proot(folder.path(), &workspace);
    let config = fixtures::config(&linux, Some(workspace.clone()), proot);
    let manager = RootfsManager::new(&linux);
    install_rootfs(&manager, &image, bytes);
    apk_script(&linux, false);
    run_setup(&manager, &image, &config).unwrap();
    fs::remove_file(linux.join("rootfs/alpine/bin/busybox")).unwrap();

    assert_eq!(
        manager.status().unwrap(),
        RootfsStatus::NeedsRepair {
            missing: 1,
            changed: 0
        }
    );
    manager
        .repair(
            &empty_fetcher(),
            &mut NoopProgress,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(
        fs::read(linux.join("rootfs/alpine/bin/busybox")).unwrap(),
        b"busybox"
    );
}

#[test]
fn repair_does_not_revert_a_package_managed_file() {
    let folder = tempdir().unwrap();
    let bytes = two_file_archive();
    let image = image(&bytes);
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let linux = folder.path().join("linux");
    let proot = fixtures::fake_proot(folder.path(), &workspace);
    let config = fixtures::config(&linux, Some(workspace.clone()), proot);
    let manager = RootfsManager::new(&linux);
    install_rootfs(&manager, &image, bytes);
    apk_rewriting_packages(&linux);
    run_setup(&manager, &image, &config).unwrap();

    manager
        .repair(
            &empty_fetcher(),
            &mut NoopProgress,
            &CancellationToken::new(),
        )
        .unwrap();

    assert_eq!(
        fs::read(linux.join("rootfs/alpine/etc/shells")).unwrap(),
        b"/bin/sh\nssh\n"
    );
    assert_eq!(
        manager.status().unwrap(),
        RootfsStatus::Ready {
            alpine_version: "3.20".into()
        }
    );
}

struct RequestParts<'a> {
    manifest: &'a [u8],
    signature: &'a [u8; 64],
    key: &'a [u8; 32],
    image: &'a RootfsImage,
    config: &'a ProotConfig,
    fetcher: &'a dyn RootfsFetcher,
    progress: &'a mut dyn tether_linux::RootfsProgress,
    cancel: &'a CancellationToken,
}

fn request<'a>(parts: &'a mut RequestParts<'_>) -> SetupRequest<'a> {
    SetupRequest {
        manifest_bytes: parts.manifest,
        signature: parts.signature,
        key: parts.key,
        image: parts.image,
        config: parts.config,
        fetcher: parts.fetcher,
        progress: &mut *parts.progress,
        cancel: parts.cancel,
    }
}

#[test]
fn successful_base_package_setup_marks_ready_and_enables_network_temporarily() {
    let folder = tempdir().unwrap();
    let bytes = fixtures::simple_archive();
    let image = image(&bytes);
    let (manifest, signature, key) = signed_manifest(&image);
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let linux = folder.path().join("linux");
    let proot = fixtures::fake_proot(folder.path(), &workspace);
    let config = fixtures::config(&linux, Some(workspace.clone()), proot);
    let manager = RootfsManager::new(&linux);
    install_rootfs(&manager, &image, bytes);
    apk_script(&linux, false);
    let fetcher = fixtures::BytesFetcher {
        bytes: Arc::new(Vec::new()),
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let token = CancellationToken::new();
    let mut progress = NoopProgress;
    let mut parts = RequestParts {
        manifest: &manifest,
        signature: &signature,
        key: &key,
        image: &image,
        config: &config,
        fetcher: &fetcher,
        progress: &mut progress,
        cancel: &token,
    };
    let mut request = request(&mut parts);

    setup(&manager, &mut request).unwrap();

    assert_eq!(
        manager.status().unwrap(),
        RootfsStatus::Ready {
            alpine_version: "3.20".into()
        }
    );
    assert_eq!(
        fs::read(workspace.join("apk-network")).unwrap(),
        b"network-on"
    );
    let command = config
        .command(&["true".into()], &[], "/", SessionKind::Command)
        .unwrap();
    assert!(command.get_envs().any(|(name, value)| {
        name.to_string_lossy() == "http_proxy"
            && value.is_some_and(|value| value.to_string_lossy() == "http://127.0.0.1:9")
    }));
}

#[test]
fn failing_base_package_setup_returns_last_four_kib_and_stays_installed() {
    let folder = tempdir().unwrap();
    let bytes = fixtures::simple_archive();
    let image = image(&bytes);
    let (manifest, signature, key) = signed_manifest(&image);
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let linux = folder.path().join("linux");
    let config = fixtures::config(
        &linux,
        Some(workspace),
        fixtures::fake_proot(folder.path(), folder.path()),
    );
    let manager = RootfsManager::new(&linux);
    install_rootfs(&manager, &image, bytes);
    apk_script(&linux, true);
    let fetcher = fixtures::BytesFetcher {
        bytes: Arc::new(Vec::new()),
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let token = CancellationToken::new();
    let mut progress = NoopProgress;
    let mut parts = RequestParts {
        manifest: &manifest,
        signature: &signature,
        key: &key,
        image: &image,
        config: &config,
        fetcher: &fetcher,
        progress: &mut progress,
        cancel: &token,
    };
    let mut request = request(&mut parts);

    let error = setup(&manager, &mut request).unwrap_err();

    let LinuxError::Packages(output) = error else {
        panic!("expected package failure");
    };
    assert!(output.contains("TAIL-FAILURE"));
    assert!(!output.contains("head"));
    assert_eq!(
        manager.status().unwrap(),
        RootfsStatus::Installed {
            alpine_version: "3.20".into()
        }
    );
}

#[test]
fn base_package_failure_after_more_than_one_mib_of_output_keeps_the_final_error() {
    let folder = tempdir().unwrap();
    let bytes = fixtures::simple_archive();
    let image = image(&bytes);
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let linux = folder.path().join("linux");
    let config = fixtures::config(
        &linux,
        Some(workspace.clone()),
        fixtures::fake_proot(folder.path(), &workspace),
    );
    let manager = RootfsManager::new(&linux);
    install_rootfs(&manager, &image, bytes);
    write_apk(
        &linux,
        "#!/bin/sh\nhead -c 1572864 /dev/zero | tr '\\0' x >&2\nprintf 'FINAL-APK-ERROR\\n' >&2\nexit 7\n",
    );

    let error = run_setup(&manager, &image, &config).unwrap_err();

    let LinuxError::Packages(output) = error else {
        panic!("expected package failure");
    };
    assert!(output.contains("FINAL-APK-ERROR"));
    assert!(output.contains("exited with 7"));
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}").expect("writing to a String cannot fail");
        output
    })
}
