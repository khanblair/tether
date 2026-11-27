use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use tempfile::TempDir;

use super::*;
use crate::{
    CoreEventView, EventSink, LinuxShellSession, LinuxShellSink, SecretVault, SecretsPlatform,
    SecretsPlatformFailure,
};

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

enum ShellEvent {
    Output(Vec<u8>),
    Exit(i32),
}

struct ShellSink(Sender<ShellEvent>);

impl LinuxShellSink for ShellSink {
    fn on_output(&self, bytes: Vec<u8>) {
        self.0.send(ShellEvent::Output(bytes)).unwrap();
    }

    fn on_exit(&self, code: i32) {
        self.0.send(ShellEvent::Exit(code)).unwrap();
    }
}

fn shell_fixture(ready: bool) -> (TempDir, Arc<SessionRuntime>) {
    let (directory, runtime) = runtime_fixture();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let proot = fake_shell_proot(directory.path(), &workspace);
    let loader = directory.path().join("loader");
    std::fs::write(&loader, b"loader").unwrap();
    let linux_root = directory.path().join("linux");
    std::fs::create_dir_all(linux_root.join("tmp")).unwrap();
    std::fs::create_dir_all(linux_root.join("etc")).unwrap();
    std::fs::write(linux_root.join("etc/resolv.empty"), b"").unwrap();
    std::fs::write(
        linux_root.join("etc/resolv.conf"),
        b"nameserver 127.0.0.1\n",
    )
    .unwrap();
    if ready {
        ready_rootfs(&linux_root);
    }
    runtime
        .configure_executors(paths(directory.path(), Some(proot), Some(loader)))
        .unwrap();
    (directory, runtime)
}

fn fake_shell_proot(root: &Path, workspace: &Path) -> PathBuf {
    let path = root.join("proot");
    let script = format!(
        "#!/bin/sh\nwhile [ \"$#\" -gt 0 ]; do\n  case \"$1\" in\n    -r|-w) shift 2 ;;\n    -b) bind=\"$2\"; case \"$bind\" in *:/workspace) cd \"${{bind%:/workspace}}\" ;; esac; shift 2 ;;\n    -0|--link2symlink|--kill-on-exit) shift ;;\n    *) break ;;\n  esac\ndone\ncd \"{}\"\nexec \"$@\"\n",
        workspace.display()
    );
    std::fs::write(&path, script).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn open_shell(runtime: &SessionRuntime) -> (Arc<LinuxShellSession>, Receiver<ShellEvent>) {
    let (sender, receiver) = mpsc::channel();
    let sink = Arc::new(ShellSink(sender));
    let session = runtime.open_linux_shell(None, 24, 80, sink).unwrap();
    (session, receiver)
}

fn collect_until_exit(receiver: &Receiver<ShellEvent>) -> (Vec<Vec<u8>>, Vec<i32>) {
    let mut output = Vec::new();
    let mut exits = Vec::new();
    loop {
        match receiver.recv_timeout(Duration::from_secs(2)).unwrap() {
            ShellEvent::Output(bytes) => output.push(bytes),
            ShellEvent::Exit(code) => {
                exits.push(code);
                break;
            }
        }
    }
    (output, exits)
}

#[test]
fn linux_shell_forwards_output_in_order_and_caps_chunks() {
    let (_directory, runtime) = shell_fixture(true);
    let (session, receiver) = open_shell(&runtime);

    session
        .write(b"stty -echo; printf first; printf second; exit\n".to_vec())
        .unwrap();
    let (chunks, exits) = collect_until_exit(&receiver);
    let output: Vec<u8> = chunks.iter().flatten().copied().collect();
    let first = output.windows(5).position(|part| part == b"first");
    let second = output.windows(6).position(|part| part == b"second");

    assert!(first.is_some());
    assert!(second.is_some());
    assert!(first.unwrap() < second.unwrap());
    assert!(chunks.iter().all(|chunk| chunk.len() <= 8 * 1024));
    assert_eq!(exits, vec![0]);
}

#[test]
fn linux_shell_write_reaches_the_guest() {
    let (_directory, runtime) = shell_fixture(true);
    let (session, receiver) = open_shell(&runtime);

    session.write(b"printf reached; exit\n".to_vec()).unwrap();
    let (chunks, _exits) = collect_until_exit(&receiver);
    let output: Vec<u8> = chunks.iter().flatten().copied().collect();

    assert!(output.windows(7).any(|part| part == b"reached"));
}

#[test]
fn linux_shell_exit_callback_fires_once_with_the_code() {
    let (_directory, runtime) = shell_fixture(true);
    let (session, receiver) = open_shell(&runtime);

    session.write(b"exit 7\n".to_vec()).unwrap();
    let (_chunks, exits) = collect_until_exit(&receiver);

    assert_eq!(exits, vec![7]);
    assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
}

#[test]
fn linux_shell_close_is_safe_to_call_twice() {
    let (_directory, runtime) = shell_fixture(true);
    let (session, receiver) = open_shell(&runtime);
    session.write(b"sleep 10\n".to_vec()).unwrap();

    session.close().unwrap();
    session.close().unwrap();
    let (_chunks, exits) = collect_until_exit(&receiver);

    assert_eq!(exits.len(), 1);
}

#[test]
fn linux_shell_rejects_a_second_open_until_the_first_exits() {
    let (_directory, runtime) = shell_fixture(true);
    let (first, receiver) = open_shell(&runtime);
    let (sender, _second_receiver) = mpsc::channel();
    let second_sink = Arc::new(ShellSink(sender));

    let result = runtime.open_linux_shell(None, 24, 80, second_sink);

    assert!(matches!(
        result,
        Err(ExecutorRuntimeError::LinuxShellAlreadyOpen)
    ));
    first.close().unwrap();
    let (_chunks, _exits) = collect_until_exit(&receiver);
}

#[test]
fn linux_shell_reports_not_ready_as_a_typed_error() {
    let (_directory, runtime) = shell_fixture(false);
    let (sender, _receiver) = mpsc::channel();
    let sink = Arc::new(ShellSink(sender));

    assert!(matches!(
        runtime.open_linux_shell(None, 24, 80, sink),
        Err(ExecutorRuntimeError::LinuxNotReady)
    ));
}

#[test]
fn dropping_linux_shell_kills_the_guest() {
    let (_directory, runtime) = shell_fixture(true);
    let (session, receiver) = open_shell(&runtime);
    session.write(b"sleep 10\n".to_vec()).unwrap();

    drop(session);
    let (_chunks, exits) = collect_until_exit(&receiver);

    assert_eq!(exits.len(), 1);
}
