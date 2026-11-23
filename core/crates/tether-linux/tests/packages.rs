//! Base package setup tests.

mod fixtures;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, atomic::AtomicUsize};

use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use sha2::{Digest, Sha256};
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
        sha256: format!("{:x}", Sha256::digest(bytes)),
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
    let path = root.join("rootfs/alpine/usr/bin/apk");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let body = if failing {
        "#!/bin/sh\nprintf 'head\\n' >&2\ni=0\nwhile [ \"$i\" -lt 5000 ]; do printf x >&2; i=$((i + 1)); done\nprintf 'TAIL-FAILURE\\n' >&2\nexit 7\n"
    } else {
        "#!/bin/sh\nif [ -n \"$http_proxy\" ] || [ -n \"$https_proxy\" ]; then exit 10; fi\nprintf network-on > \"$PWD/apk-network\"\nexit 0\n"
    };
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
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
