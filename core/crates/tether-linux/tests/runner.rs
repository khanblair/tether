//! Streaming runner integration tests.

mod fixtures;

use std::fs;
use std::time::Duration;

use tempfile::tempdir;
use tether_linux::{LinuxRunner, RunCommand, RunOutcome, RunRequest};
use tokio_util::sync::CancellationToken;

#[test]
fn fake_proot_streams_output_caps_bytes_and_maps_workspace() {
    let directory = tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let proot = fixtures::fake_proot(directory.path(), &workspace);
    let config = fixtures::config(
        &directory.path().join("linux"),
        Some(workspace.clone()),
        proot,
    );
    let runner = LinuxRunner::new(config);
    let request = RunRequest {
        command: RunCommand::Shell("printf stdout; printf stderr >&2; printf file > result".into()),
        cwd: "/workspace".into(),
        extra_env: Vec::new(),
        timeout: Some(Duration::from_secs(2)),
        output_cap_bytes: 8,
    };
    let mut output = fixtures::Collected::new();

    let report = runner
        .run(&request, &mut output, &CancellationToken::new())
        .unwrap();

    assert_eq!(report.outcome, RunOutcome::Exited(0));
    assert_eq!(report.output_bytes, 8);
    assert!(report.output_truncated);
    assert!(workspace.join("result").is_file());
    assert!(!output.stdout.is_empty() || !output.stderr.is_empty());
}

#[test]
fn fake_proot_timeout_kills_the_child() {
    let directory = tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let proot = fixtures::fake_proot(directory.path(), &workspace);
    let config = fixtures::config(&directory.path().join("linux"), Some(workspace), proot);
    let runner = LinuxRunner::new(config);
    let request = RunRequest {
        command: RunCommand::Shell("sleep 2".into()),
        timeout: Some(Duration::from_millis(40)),
        ..RunRequest::default()
    };
    let mut output = fixtures::Collected::new();

    let report = runner
        .run(&request, &mut output, &CancellationToken::new())
        .unwrap();

    assert_eq!(report.outcome, RunOutcome::TimedOut);
}

#[test]
fn missing_proot_is_typed_not_ready_error() {
    let directory = tempdir().unwrap();
    let config = fixtures::config(
        &directory.path().join("linux"),
        None,
        directory.path().join("missing-proot"),
    );
    let runner = LinuxRunner::new(config);
    let mut output = fixtures::Collected::new();

    let error = runner
        .run(
            &RunRequest::default(),
            &mut output,
            &CancellationToken::new(),
        )
        .unwrap_err();

    assert!(matches!(error, tether_linux::LinuxError::NotReady(_)));
}
