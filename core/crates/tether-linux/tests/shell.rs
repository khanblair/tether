//! Interactive shell integration tests.

mod fixtures;

use std::time::Duration;

use tempfile::tempdir;
use tether_linux::{LinuxShell, PtySize};

#[test]
fn shell_writes_reads_resizes_and_kills() {
    let directory = tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let config = fixtures::config(
        &directory.path().join("linux"),
        Some(workspace),
        fixtures::fake_proot(directory.path(), &directory.path().join("workspace")),
    );
    let mut shell = LinuxShell::spawn(
        &config,
        PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        },
    )
    .unwrap();
    let output = shell.output();
    shell.resize(30, 100).unwrap();
    shell.write(b"echo hi\n").unwrap();
    let bytes = output.recv_timeout(Duration::from_secs(2)).unwrap();
    shell.kill().unwrap();

    assert!(String::from_utf8_lossy(&bytes).contains("hi"));
    assert!(shell.wait().is_ok());
}

#[test]
fn dropping_shell_kills_the_guest_process() {
    let directory = tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let config = fixtures::config(
        &directory.path().join("linux"),
        Some(workspace),
        fixtures::fake_proot(directory.path(), &directory.path().join("workspace")),
    );
    let mut shell = LinuxShell::spawn(
        &config,
        PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        },
    )
    .unwrap();
    let output = shell.output();
    shell
        .write(b"trap 'printf stopped; exit' INT; sleep 10\n")
        .unwrap();

    drop(shell);

    let mut received = Vec::new();
    let mut terminated = false;
    while !terminated && !received.windows(7).any(|bytes| bytes == b"stopped") {
        match output.recv_timeout(Duration::from_secs(2)) {
            Ok(bytes) => received.extend(bytes),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => terminated = true,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
        }
    }
    assert!(terminated || String::from_utf8_lossy(&received).contains("stopped"));
}
