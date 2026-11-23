//! Runner lifecycle and stream tests.

mod fixtures;

use std::fs;
use std::time::Duration;

use tempfile::tempdir;
use tether_linux::{LinuxRunner, RunCommand, RunOutcome, RunRequest};
use tokio_util::sync::CancellationToken;

#[test]
fn runner_forwards_stdout_and_stderr_as_separate_streams() {
    let folder = tempdir().unwrap();
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let config = fixtures::config(
        &folder.path().join("linux"),
        Some(workspace),
        fixtures::fake_proot(folder.path(), folder.path().join("workspace").as_path()),
    );
    let runner = LinuxRunner::new(config);
    let request = RunRequest {
        command: RunCommand::Shell("printf out; printf err >&2; exit 9".into()),
        timeout: Some(Duration::from_secs(2)),
        ..RunRequest::default()
    };
    let mut output = fixtures::Collected::new();

    let report = runner
        .run(&request, &mut output, &CancellationToken::new())
        .unwrap();

    assert_eq!(report.outcome, RunOutcome::Exited(9));
    assert_eq!(output.stdout.concat(), b"out");
    assert_eq!(output.stderr.concat(), b"err");
}

#[test]
fn runner_reports_exact_cap_and_truncation() {
    let folder = tempdir().unwrap();
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let config = fixtures::config(
        &folder.path().join("linux"),
        Some(workspace.clone()),
        fixtures::fake_proot(folder.path(), &workspace),
    );
    let runner = LinuxRunner::new(config);
    let request = RunRequest {
        command: RunCommand::Shell("printf 12345; printf 67890 >&2".into()),
        output_cap_bytes: 7,
        timeout: Some(Duration::from_secs(2)),
        ..RunRequest::default()
    };
    let mut output = fixtures::Collected::new();

    let report = runner
        .run(&request, &mut output, &CancellationToken::new())
        .unwrap();

    assert_eq!(report.outcome, RunOutcome::Exited(0));
    assert_eq!(report.output_bytes, 7);
    assert!(report.output_truncated);
    assert_eq!(
        output.stdout.concat().len() + output.stderr.concat().len(),
        7
    );
}

#[test]
fn runner_cancelled_before_start_kills_child() {
    let folder = tempdir().unwrap();
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let config = fixtures::config(
        &folder.path().join("linux"),
        Some(workspace.clone()),
        fixtures::fake_proot(folder.path(), &workspace),
    );
    let runner = LinuxRunner::new(config);
    let request = RunRequest {
        command: RunCommand::Shell("sleep 10".into()),
        timeout: Some(Duration::from_secs(2)),
        ..RunRequest::default()
    };
    let token = CancellationToken::new();
    token.cancel();
    let mut output = fixtures::Collected::new();

    let report = runner.run(&request, &mut output, &token).unwrap();

    assert_eq!(report.outcome, RunOutcome::Cancelled);
}

#[test]
fn runner_maps_guest_workspace_to_host_workspace() {
    let folder = tempdir().unwrap();
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let config = fixtures::config(
        &folder.path().join("linux"),
        Some(workspace.clone()),
        fixtures::fake_proot(folder.path(), &workspace),
    );
    let runner = LinuxRunner::new(config);
    let request = RunRequest {
        command: RunCommand::Shell("pwd; printf data > created".into()),
        cwd: "/workspace".into(),
        timeout: Some(Duration::from_secs(2)),
        ..RunRequest::default()
    };
    let mut output = fixtures::Collected::new();

    runner
        .run(&request, &mut output, &CancellationToken::new())
        .unwrap();

    assert_eq!(
        output.stdout.concat(),
        format!("{}\n", workspace.display()).as_bytes()
    );
    assert_eq!(fs::read(workspace.join("created")).unwrap(), b"data");
}
