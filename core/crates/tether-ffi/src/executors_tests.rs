use std::path::{Path, PathBuf};
use std::sync::Arc;

use tempfile::TempDir;

use super::*;
use crate::{CoreEventView, EventSink, SecretVault, SecretsPlatform, SecretsPlatformFailure};

struct PlainPlatform;

impl SecretsPlatform for PlainPlatform {
    fn wrap(
        &self,
        plaintext: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, SecretsPlatformFailure> {
        Ok(plaintext)
    }

    fn unwrap(
        &self,
        wrapped: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, SecretsPlatformFailure> {
        Ok(wrapped)
    }
}

struct NoopSink;

impl EventSink for NoopSink {
    fn on_event(&self, _event: CoreEventView) {}
}

fn runtime_fixture() -> (TempDir, Arc<SessionRuntime>) {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("tether.db");
    let vault = SecretVault::new(
        directory
            .path()
            .join("secrets")
            .to_string_lossy()
            .into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime =
        SessionRuntime::new(database.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    (directory, runtime)
}

fn paths(
    root: &Path,
    proot_path: Option<PathBuf>,
    loader_path: Option<PathBuf>,
) -> ExecutorPathsView {
    ExecutorPathsView {
        packs_root: root.join("packs").to_string_lossy().into_owned(),
        linux_root: root.join("linux").to_string_lossy().into_owned(),
        proot_path: proot_path.map(|path| path.to_string_lossy().into_owned()),
        proot_loader_path: loader_path.map(|path| path.to_string_lossy().into_owned()),
        pack_verifying_key_hex: None,
        rootfs_verifying_key_hex: None,
    }
}

fn ready_rootfs(linux_root: &Path) {
    std::fs::create_dir_all(linux_root.join("rootfs/alpine")).unwrap();
    let state = serde_json::json!({
        "alpine_version": "3.20",
        "image": {
            "arch": "aarch64",
            "url": "https://example.invalid/alpine.tar.gz",
            "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "size_bytes": 1,
            "max_extracted_bytes": 1
        },
        "files": [],
        "ready": true,
        "skipped_special": 0
    });
    std::fs::write(
        linux_root.join("rootfs/alpine.json"),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
}

#[test]
fn configure_rejects_proot_without_a_loader() {
    let (directory, runtime) = runtime_fixture();
    let result = runtime.configure_executors(paths(
        directory.path(),
        Some(directory.path().join("proot")),
        None,
    ));

    assert!(matches!(
        result,
        Err(ExecutorRuntimeError::InvalidConfiguration)
    ));
}

#[test]
fn configure_rejects_relative_proot_or_loader_paths() {
    let (directory, runtime) = runtime_fixture();
    let loader = directory.path().join("loader");
    let relative_proot = ExecutorPathsView {
        proot_path: Some("relative/proot".into()),
        ..paths(
            directory.path(),
            Some(directory.path().join("proot")),
            Some(loader.clone()),
        )
    };
    assert!(matches!(
        runtime.configure_executors(relative_proot),
        Err(ExecutorRuntimeError::InvalidConfiguration)
    ));

    let relative_loader = ExecutorPathsView {
        proot_loader_path: Some("relative/loader".into()),
        ..paths(
            directory.path(),
            Some(directory.path().join("proot")),
            Some(loader),
        )
    };
    assert!(matches!(
        runtime.configure_executors(relative_loader),
        Err(ExecutorRuntimeError::InvalidConfiguration)
    ));
}

#[test]
fn configure_replaces_saved_paths_on_the_next_call() {
    let (directory, runtime) = runtime_fixture();
    let first_root = directory.path().join("first");
    let second_root = directory.path().join("second");
    runtime
        .configure_executors(paths(
            &first_root,
            Some(first_root.join("proot")),
            Some(first_root.join("loader")),
        ))
        .unwrap();
    runtime
        .configure_executors(paths(
            &second_root,
            Some(second_root.join("proot")),
            Some(second_root.join("loader")),
        ))
        .unwrap();

    let saved = std::fs::read_to_string(directory.path().join("executors/paths.conf")).unwrap();
    assert!(saved.contains(&format!(
        "packs_root={}",
        second_root.join("packs").display()
    )));
    assert!(saved.contains(&format!(
        "proot_path={}",
        second_root.join("proot").display()
    )));
    assert!(saved.contains(&format!(
        "loader_path={}",
        second_root.join("loader").display()
    )));
    assert!(!saved.contains(&first_root.to_string_lossy().to_string()));
}

#[test]
fn linux_readiness_requires_the_loader_file() {
    let (directory, runtime) = runtime_fixture();
    let proot = directory.path().join("native/libproot.so");
    let loader = directory.path().join("native/libproot-loader.so");
    let linux_root = directory.path().join("linux");
    std::fs::create_dir_all(proot.parent().unwrap()).unwrap();
    std::fs::write(&proot, b"proot").unwrap();
    ready_rootfs(&linux_root);
    runtime
        .configure_executors(paths(directory.path(), Some(proot), Some(loader.clone())))
        .unwrap();

    assert!(!runtime.executor_availability().unwrap().linux_ready);
    std::fs::write(loader, b"loader").unwrap();
    assert!(runtime.executor_availability().unwrap().linux_ready);
}
