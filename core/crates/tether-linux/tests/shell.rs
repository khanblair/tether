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
