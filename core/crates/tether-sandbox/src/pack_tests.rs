//! Runtime pack verification tests.

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::Arc;

use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

use crate::test_support::{HELLO, fixture_bytes, run_lock};
use crate::{
    ExtraMount, ModuleCache, OutputSink, PackEntry, PackError, PackFetcher, PackFile, PackManifest,
    PackMount, RunLimits, RunOutcome, RunSpec, SandboxRunner, WasiContextSpec, install_pack,
    program_for, verify_manifest,
};

const MODULE: &[u8] = b"module bytes";
const MODULE_HASH: &str = "5c073b6de205a83e4df87dcd07111be224ad4435a8c08ca0ba10e68dcf38fe92";

struct FakeFetcher {
    files: BTreeMap<String, Vec<u8>>,
    mutate: bool,
    too_many: bool,
}

impl PackFetcher for FakeFetcher {
    fn fetch(
        &self,
        _pack: &PackEntry,
        file: &PackFile,
        out: &mut dyn Write,
        _cancel: &CancellationToken,
    ) -> Result<(), PackError> {
        let mut bytes = self.files.get(&file.path).cloned().unwrap_or_default();
        if self.mutate && !bytes.is_empty() {
            bytes[0] ^= 1;
        }
        if self.too_many {
            bytes.extend_from_slice(b"overflow");
        }
        drop(out.write(&bytes));
        Ok(())
    }
}

fn manifest() -> PackManifest {
    PackManifest {
        format: 1,
        packs: vec![PackEntry {
            id: "python".into(),
            version: "1.0.0".into(),
            programs: vec!["python".into(), "python3".into()],
            entry: "runtime.wasm".into(),
            files: vec![PackFile {
                path: "runtime.wasm".into(),
                size_bytes: MODULE.len() as u64,
                sha256: MODULE_HASH.into(),
            }],
            mounts: Vec::new(),
            env: BTreeMap::new(),
            args_prefix: Vec::new(),
        }],
    }
}

fn manifest_with_module(module: &[u8]) -> PackManifest {
    let hash = format!("{:x}", Sha256::digest(module));
    PackManifest {
        format: 1,
        packs: vec![PackEntry {
            id: "fixture".into(),
            version: "1.0.0".into(),
            programs: vec!["hello".into()],
            entry: "runtime.wasm".into(),
            files: vec![PackFile {
                path: "runtime.wasm".into(),
                size_bytes: module.len() as u64,
                sha256: hash,
            }],
            mounts: Vec::new(),
            env: BTreeMap::new(),
            args_prefix: Vec::new(),
        }],
    }
}

#[test]
fn signed_manifest_verifies_with_the_fixed_test_key() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let manifest = manifest();
    let bytes = serde_json::to_vec(&manifest).unwrap();
    let signature = key.sign(&bytes);
    assert!(
        verify_manifest(
            &bytes,
            &signature.to_bytes(),
            &key.verifying_key().to_bytes()
        )
        .is_ok()
    );
    assert!(
        verify_manifest(
            &bytes,
            &signature.to_bytes(),
            &key.verifying_key().to_bytes()
        )
        .is_ok()
    );
}

#[test]
fn flipped_manifest_bytes_are_rejected() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let bytes = serde_json::to_vec(&manifest()).unwrap();
    let signature = key.sign(&bytes);
    let mut changed = bytes;
    changed[0] ^= 1;
    assert!(matches!(
        verify_manifest(
            &changed,
            &signature.to_bytes(),
            &key.verifying_key().to_bytes()
        ),
        Err(PackError::Signature)
    ));
}

#[test]
fn flipped_signature_bytes_are_rejected() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let bytes = serde_json::to_vec(&manifest()).unwrap();
    let signature = key.sign(&bytes);
    let mut changed_signature = signature.to_bytes();
    changed_signature[0] ^= 1;
    assert!(matches!(
        verify_manifest(&bytes, &changed_signature, &key.verifying_key().to_bytes()),
        Err(PackError::Signature)
    ));
}

#[test]
fn wrong_manifest_key_is_rejected() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let bytes = serde_json::to_vec(&manifest()).unwrap();
    let signature = key.sign(&bytes);
    assert!(matches!(
        verify_manifest(&bytes, &signature.to_bytes(), &[9; 32]),
        Err(PackError::Signature)
    ));
}

#[test]
fn missing_manifest_signature_is_rejected() {
    let key = SigningKey::from_bytes(&[7; 32]);
    let bytes = serde_json::to_vec(&manifest()).unwrap();
    assert!(matches!(
        verify_manifest(&bytes, &[], &key.verifying_key().to_bytes()),
        Err(PackError::Signature)
    ));
}

#[test]
fn install_verify_program_lookup_and_tamper_detection() {
    let root = tempdir().unwrap();
    let mut files = BTreeMap::new();
    files.insert("runtime.wasm".into(), MODULE.to_vec());
    let fetcher = FakeFetcher {
        files,
        mutate: false,
        too_many: false,
    };
    let installed = install_pack(
        &manifest(),
        "python",
        &fetcher,
        root.path(),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(installed.entry().programs, ["python", "python3"]);
    assert_eq!(installed.module_bytes().unwrap(), MODULE);
    assert!(program_for(root.path(), "python3").is_ok());
    std::fs::write(root.path().join("python/1.0.0/runtime.wasm"), b"changed").unwrap();
    assert!(matches!(
        installed.verify(),
        Err(PackError::Size { .. } | PackError::HashMismatch { .. })
    ));
}

#[test]
fn install_writes_verified_files_to_disk() {
    let root = tempdir().unwrap();
    let mut files = BTreeMap::new();
    files.insert("runtime.wasm".into(), MODULE.to_vec());
    let installed = install_pack(
        &manifest(),
        "python",
        &FakeFetcher {
            files,
            mutate: false,
            too_many: false,
        },
        root.path(),
        &CancellationToken::new(),
    )
    .unwrap();

    assert_eq!(
        std::fs::read(installed.root().join("runtime.wasm")).unwrap(),
        MODULE
    );
    assert!(installed.root().join("manifest.json").is_file());
}

#[test]
fn oversized_fetch_is_rejected_before_install() {
    let root = tempdir().unwrap();
    let mut files = BTreeMap::new();
    files.insert("runtime.wasm".into(), MODULE.to_vec());
    let fetcher = FakeFetcher {
        files,
        mutate: false,
        too_many: true,
    };
    assert!(matches!(
        install_pack(
            &manifest(),
            "python",
            &fetcher,
            root.path(),
            &CancellationToken::new()
        ),
        Err(PackError::Size { .. })
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn changed_fetch_bytes_leave_no_pack_files() {
    let root = tempdir().unwrap();
    let mut files = BTreeMap::new();
    files.insert("runtime.wasm".into(), MODULE.to_vec());
    let result = install_pack(
        &manifest(),
        "python",
        &FakeFetcher {
            files,
            mutate: true,
            too_many: false,
        },
        root.path(),
        &CancellationToken::new(),
    );

    assert!(matches!(result, Err(PackError::HashMismatch { .. })));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

struct CancelFetcher;

impl PackFetcher for CancelFetcher {
    fn fetch(
        &self,
        _pack: &PackEntry,
        _file: &PackFile,
        out: &mut dyn Write,
        cancel: &CancellationToken,
    ) -> Result<(), PackError> {
        out.write_all(MODULE).map_err(|_| PackError::Io {
            id: "python".into(),
            path: "runtime.wasm".into(),
        })?;
        cancel.cancel();
        Ok(())
    }
}

#[test]
fn cancellation_mid_fetch_leaves_no_pack_files() {
    let root = tempdir().unwrap();
    let result = install_pack(
        &manifest(),
        "python",
        &CancelFetcher,
        root.path(),
        &CancellationToken::new(),
    );

    assert!(matches!(result, Err(PackError::Cancelled { .. })));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn old_manifest_without_optional_fields_still_verifies() {
    let value = serde_json::json!({
        "format": 1,
        "packs": [{
            "id": "python",
            "version": "1.0.0",
            "programs": ["python"],
            "entry": "runtime.wasm",
            "files": [{"path":"runtime.wasm","size_bytes":12,"sha256":MODULE_HASH}]
        }]
    });
    let parsed: PackManifest = serde_json::from_value(value).unwrap();
    assert_eq!(parsed.packs[0].mounts.len(), 0);
    assert_eq!(parsed.packs[0].env.len(), 0);
    assert_eq!(parsed.packs[0].args_prefix.len(), 0);
}

#[test]
fn invalid_mount_and_environment_rules_are_rejected() {
    let mut manifest = manifest();
    manifest.packs[0].mounts = vec![PackMount {
        path: "support".into(),
        guest: "/workspace/support".into(),
    }];
    assert!(matches!(
        crate::pack::validate_manifest(&manifest),
        Err(PackError::Manifest(_))
    ));

    manifest.packs[0].mounts[0].guest = "/opt/support/../bad".into();
    assert!(matches!(
        crate::pack::validate_manifest(&manifest),
        Err(PackError::Manifest(_))
    ));

    manifest.packs[0].env.insert("bad-name".into(), "x".into());
    assert!(matches!(
        crate::pack::validate_manifest(&manifest),
        Err(PackError::Manifest(_))
    ));
}

#[test]
fn every_pack_path_rule_is_rejected() {
    for path in ["/absolute", "../parent", "dir\\file", "nul\0file"] {
        let mut invalid = manifest();
        invalid.packs[0].files[0].path = path.into();
        assert!(matches!(
            crate::pack::validate_manifest(&invalid),
            Err(PackError::Path { .. })
        ));
    }

    let mut duplicate = manifest();
    let file = duplicate.packs[0].files[0].clone();
    duplicate.packs[0].files.push(file);
    assert!(matches!(
        crate::pack::validate_manifest(&duplicate),
        Err(PackError::Manifest(_))
    ));
}

#[test]
fn oversized_pack_file_is_rejected_before_disk_creation() {
    let root = tempdir().unwrap();
    let mut invalid = manifest();
    invalid.packs[0].files[0].size_bytes = 512 * 1024 * 1024 + 1;

    assert!(matches!(
        install_pack(
            &invalid,
            "python",
            &FakeFetcher {
                files: BTreeMap::new(),
                mutate: false,
                too_many: false,
            },
            root.path(),
            &CancellationToken::new()
        ),
        Err(PackError::Manifest(_))
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn file_used_as_folder_is_rejected_without_leftovers() {
    let root = tempdir().unwrap();
    let mut invalid = manifest();
    let empty_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    invalid.packs[0].files.push(PackFile {
        path: "dir".into(),
        size_bytes: 0,
        sha256: empty_hash.into(),
    });
    invalid.packs[0].files.push(PackFile {
        path: "dir/file".into(),
        size_bytes: 0,
        sha256: empty_hash.into(),
    });

    let result = install_pack(
        &invalid,
        "python",
        &FakeFetcher {
            files: BTreeMap::new(),
            mutate: false,
            too_many: false,
        },
        root.path(),
        &CancellationToken::new(),
    );

    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn installed_symlink_is_rejected() {
    use std::os::unix::fs::symlink;

    let root = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let installed = install_pack(
        &manifest(),
        "python",
        &FakeFetcher {
            files: BTreeMap::from([(String::from("runtime.wasm"), MODULE.to_vec())]),
            mutate: false,
            too_many: false,
        },
        root.path(),
        &CancellationToken::new(),
    )
    .unwrap();
    std::fs::remove_file(installed.root().join("runtime.wasm")).unwrap();
    symlink(
        outside.path().join("secret"),
        installed.root().join("runtime.wasm"),
    )
    .unwrap();

    assert!(matches!(installed.verify(), Err(PackError::Path { .. })));
}

#[test]
fn verify_after_changing_file_fails() {
    let root = tempdir().unwrap();
    let installed = installed_pack(root.path());
    std::fs::write(installed.root().join("runtime.wasm"), b"changed").unwrap();

    assert!(matches!(installed.verify(), Err(PackError::Size { .. })));
}

#[test]
fn verify_after_deleting_file_fails() {
    let root = tempdir().unwrap();
    let installed = installed_pack(root.path());
    std::fs::remove_file(installed.root().join("runtime.wasm")).unwrap();

    assert!(matches!(
        installed.verify(),
        Err(PackError::NotFound { .. })
    ));
}

#[test]
fn verify_after_truncating_file_fails() {
    let root = tempdir().unwrap();
    let installed = installed_pack(root.path());
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(installed.root().join("runtime.wasm"))
        .unwrap();
    file.set_len(1).unwrap();

    assert!(matches!(installed.verify(), Err(PackError::Size { .. })));
}

fn installed_pack(root: &std::path::Path) -> crate::InstalledPack {
    install_pack(
        &manifest(),
        "python",
        &FakeFetcher {
            files: BTreeMap::from([(String::from("runtime.wasm"), MODULE.to_vec())]),
            mutate: false,
            too_many: false,
        },
        root,
        &CancellationToken::new(),
    )
    .unwrap()
}

#[test]
fn installed_fixture_runs_end_to_end() {
    let _guard = run_lock();
    let module_bytes = fixture_bytes(HELLO).unwrap();
    let manifest = manifest_with_module(&module_bytes);
    let root = tempdir().unwrap();
    let mut files = BTreeMap::new();
    files.insert(String::from("runtime.wasm"), module_bytes);
    let installed = install_pack(
        &manifest,
        "fixture",
        &FakeFetcher {
            files,
            mutate: false,
            too_many: false,
        },
        root.path(),
        &CancellationToken::new(),
    )
    .unwrap();
    let (bytes, argv, mounts, env) = installed.run_spec_parts("hello").unwrap();
    let mounts = mounts
        .into_iter()
        .map(|mount| ExtraMount {
            host: installed.root().join(mount.path),
            guest: mount.guest,
        })
        .collect();
    let cache = Arc::new(ModuleCache::new().unwrap());
    let workspace = tempdir().unwrap();
    let spec = RunSpec {
        module: cache.load(&bytes).unwrap(),
        argv,
        workspace: WasiContextSpec::new(workspace.path()).unwrap(),
        cwd: "/workspace".into(),
        pack_root: installed.root().to_path_buf(),
        extra_mounts: mounts,
        extra_env: env,
        limits: RunLimits::default(),
    };
    let mut sink = PackSink::default();
    let report = SandboxRunner::new(cache)
        .run(&spec, &mut sink, &CancellationToken::new())
        .unwrap();

    assert_eq!(report.outcome, RunOutcome::Exited(0));
    assert_eq!(sink.stdout, b"hello\n");
}

#[derive(Default)]
struct PackSink {
    stdout: Vec<u8>,
}

impl OutputSink for PackSink {
    fn stdout(&mut self, bytes: &[u8]) {
        self.stdout.extend_from_slice(bytes);
    }

    fn stderr(&mut self, _bytes: &[u8]) {}
}
