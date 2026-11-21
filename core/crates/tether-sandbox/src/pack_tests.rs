//! Runtime pack verification tests.

use std::collections::BTreeMap;
use std::io::Write;

use ed25519_dalek::{Signer, SigningKey};
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

use crate::{
    PackEntry, PackError, PackFetcher, PackFile, PackManifest, PackMount, install_pack,
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

#[test]
fn signed_manifest_rejects_changes_and_wrong_keys() {
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
    let mut changed = bytes.clone();
    changed[0] ^= 1;
    assert!(matches!(
        verify_manifest(
            &changed,
            &signature.to_bytes(),
            &key.verifying_key().to_bytes()
        ),
        Err(PackError::Signature)
    ));
    let mut changed_signature = signature.to_bytes();
    changed_signature[0] ^= 1;
    assert!(matches!(
        verify_manifest(&bytes, &changed_signature, &key.verifying_key().to_bytes()),
        Err(PackError::Signature)
    ));
    assert!(matches!(
        verify_manifest(&bytes, &signature.to_bytes(), &[9; 32]),
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
