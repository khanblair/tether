use std::sync::Arc;
use std::time::Duration;

use super::super::{BackgroundProcesses, LinuxRunnerLike};
use tether_linux::{LinuxError, LinuxRunReport, OutputSink, RunOutcome, RunRequest};
use tether_workspace::PathGuard;
use tokio_util::sync::CancellationToken;

use super::RunTools;
use super::run_support::{append_user_args, format_result, parse_run_result, trim_output};
use crate::executors::{CiExecutor, CiRunRequest, ExecutorError, RunReport, RunStatus};
use crate::permissions::Effect;
use crate::permissions::rule_for;
use crate::tool_loop::{ExecuteContext, ToolOutput};
use crate::tool_loop::{PrepareContext, ToolHandler};
use tether_providers::BoxFuture;

struct TestCi;

impl CiExecutor for TestCi {
    fn run<'a>(
        &'a self,
        _request: CiRunRequest,
        _sink: Option<&'a dyn crate::EventSink>,
    ) -> BoxFuture<'a, Result<RunReport, ExecutorError>> {
        Box::pin(std::future::ready(Err(ExecutorError::NotConfigured)))
    }
}

struct TestSink;

impl crate::EventSink for TestSink {
    fn on_event(&self, _event: tether_types::CoreEvent) {}
}

struct BackgroundRunner;

impl LinuxRunnerLike for BackgroundRunner {
    fn run(
        &self,
        _request: &RunRequest,
        sink: &mut dyn OutputSink,
        cancel: &CancellationToken,
    ) -> Result<LinuxRunReport, LinuxError> {
        sink.stdout(b"Local: http://localhost:5173/\n");
        while !cancel.is_cancelled() {
            std::thread::park_timeout(Duration::from_millis(1));
        }
        Ok(LinuxRunReport {
            outcome: RunOutcome::Cancelled,
            elapsed: Duration::from_millis(1),
            output_bytes: 32,
            output_truncated: false,
        })
    }
}

#[test]
fn output_cut_keeps_head_tail_and_exact_omitted_count() {
    let output = format!(
        "{}{}{}",
        "h".repeat(4_000),
        "x".repeat(100_000),
        "t".repeat(12_000)
    );
    let (trimmed, omitted) = trim_output(&output);

    assert_eq!(omitted, 100_000);
    assert!(trimmed.starts_with(&"h".repeat(4_000)));
    assert!(trimmed.contains("[... 100000 characters omitted ...]"));
    assert!(trimmed.ends_with(&"t".repeat(12_000)));
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn user_arguments_follow_the_pack_prefix_without_losing_any() {
    let prefix = strings(&["python", "-I"]);
    let typed = strings(&["python", "-m", "pytest", "-q"]);
    assert_eq!(
        append_user_args(prefix, &typed),
        strings(&["python", "-I", "-m", "pytest", "-q"])
    );
}

#[test]
fn a_pack_without_a_prefix_keeps_the_command_as_typed() {
    let typed = strings(&["ls", "-la"]);
    assert_eq!(append_user_args(strings(&["ls"]), &typed), typed);
    assert_eq!(
        append_user_args(strings(&["ls", "--color"]), &strings(&["ls"])),
        strings(&["ls", "--color"])
    );
}

#[test]
fn a_run_target_is_the_bare_command_so_a_saved_rule_names_the_program() {
    let root = std::env::temp_dir().join(format!("tether-run-prepare-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let guard = PathGuard::new(&root).unwrap();
    let tools = RunTools::new(root.join("packs")).unwrap().with_executors(
        None,
        Arc::new(TestCi),
        None,
        Arc::new(|_| false),
    );
    let call = tools
        .prepare(
            "run",
            r#"{"command":"echo hello","executor":"ci"}"#,
            &PrepareContext { guard: &guard },
        )
        .unwrap();
    assert_eq!(call.target, "echo hello");
    assert_eq!(call.display_target(), "[ci] echo hello");
    let rule = rule_for(call.effect, &call.tool, &call.target, call.executor).unwrap();
    assert_eq!(rule.pattern, "echo");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[expect(
    clippy::too_many_lines,
    clippy::excessive_nesting,
    reason = "The status matrix keeps every formatter and parser case together."
)]
fn formatted_run_statuses_round_trip_for_each_executor() {
    let cases = [
        (
            RunStatus::Exited(0),
            serde_json::json!({"_timeout": 30}),
            false,
        ),
        (
            RunStatus::Exited(1),
            serde_json::json!({"_timeout": 30}),
            false,
        ),
        (
            RunStatus::TimedOut,
            serde_json::json!({"_timeout": 30}),
            false,
        ),
        (RunStatus::TimedOut, serde_json::json!({}), false),
        (RunStatus::Cancelled, serde_json::json!({}), false),
        (RunStatus::OutOfMemory, serde_json::json!({}), false),
        (RunStatus::Exited(0), serde_json::json!({}), true),
    ];
    for executor in [
        tether_types::Executor::Sandbox,
        tether_types::Executor::Linux,
        tether_types::Executor::Ci,
    ] {
        for (status, extra, truncated) in &cases {
            let mut payload = serde_json::json!({"command": "echo hello"});
            payload
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            let call = crate::tool_loop::PreparedCall {
                tool: "run".into(),
                effect: if executor == tether_types::Executor::Sandbox {
                    Effect::RunSandbox
                } else {
                    Effect::RunHost
                },
                target: "echo hello".into(),
                executor: Some(executor),
                payload,
            };
            let text = format_result(
                executor,
                &call,
                &RunReport {
                    status: *status,
                    output: "output".into(),
                    duration: Duration::from_millis(123),
                    output_truncated: *truncated,
                },
            );
            let parsed = parse_run_result(&text, false);
            assert_eq!(parsed.executor, Some(executor));
            assert_eq!(parsed.duration_ms, Some(123));
            assert_eq!(
                parsed.outcome,
                match *status {
                    RunStatus::Exited(code) => super::run_support::RunOutcome::Exited(code),
                    RunStatus::TimedOut => super::run_support::RunOutcome::TimedOut,
                    RunStatus::Cancelled => super::run_support::RunOutcome::Cancelled,
                    RunStatus::OutOfMemory => super::run_support::RunOutcome::OutOfMemory,
                }
            );
        }
    }
}

#[test]
fn parser_handles_background_cancelled_denied_and_unknown_results() {
    let background = parse_run_result(
        "process: p1\nport: 5173\nurl: http://127.0.0.1:5173/\noutput:\nready",
        false,
    );
    assert_eq!(background.executor, Some(tether_types::Executor::Linux));
    assert_eq!(
        background.outcome,
        super::run_support::RunOutcome::Background
    );
    assert_eq!(background.duration_ms, None);
    assert_eq!(
        parse_run_result("tool cancelled before execution", true).outcome,
        super::run_support::RunOutcome::Cancelled
    );
    assert_eq!(
        parse_run_result("denied by the user: needs approval", true).outcome,
        super::run_support::RunOutcome::Denied
    );
    assert_eq!(
        parse_run_result("garbage", false).outcome,
        super::run_support::RunOutcome::Unknown
    );
}

fn run_tools(root: &std::path::Path) -> (RunTools, PathGuard) {
    let guard = PathGuard::new(root).unwrap();
    let tools = RunTools::new(root.join("packs")).unwrap().with_executors(
        None,
        Arc::new(TestCi),
        None,
        Arc::new(|_| false),
    );
    (tools, guard)
}

#[test]
fn run_cwd_normalizes_the_root_and_clean_relative_folders() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("web")).unwrap();
    let (tools, guard) = run_tools(root.path());
    for (cwd, expected) in [(".", ""), ("", ""), ("./web", "web"), ("web/", "web")] {
        let args = serde_json::json!({"command": "echo hello", "executor": "ci", "cwd": cwd});
        let call = tools
            .prepare("run", &args.to_string(), &PrepareContext { guard: &guard })
            .unwrap();
        assert_eq!(call.payload["_cwd"].as_str(), Some(expected), "cwd {cwd:?}");
    }
}

#[test]
fn run_cwd_rejects_escapes_and_absolute_paths() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("web")).unwrap();
    let (tools, guard) = run_tools(root.path());
    for cwd in ["..", "/etc", "web/../..", "../web"] {
        let args = serde_json::json!({"command": "echo hello", "executor": "ci", "cwd": cwd});
        let error = tools
            .prepare("run", &args.to_string(), &PrepareContext { guard: &guard })
            .unwrap_err();
        assert_eq!(error, "cwd must stay inside the workspace", "cwd {cwd:?}");
    }
}

#[test]
fn background_runs_refuse_sandbox_and_ci_with_the_same_error() {
    let root = tempfile::tempdir().unwrap();
    let (tools, guard) = run_tools(root.path());
    for executor in ["sandbox", "ci"] {
        let args = serde_json::json!({
            "command": "npm run dev",
            "executor": executor,
            "background": true
        });
        assert_eq!(
            tools
                .prepare("run", &args.to_string(), &PrepareContext { guard: &guard })
                .unwrap_err(),
            "background runs need Linux mode"
        );
    }
}

#[tokio::test]
async fn background_run_returns_process_id_and_loopback_url() {
    let root = tempfile::tempdir().unwrap();
    let guard = PathGuard::new(root.path()).unwrap();
    let sink: Arc<dyn crate::EventSink> = Arc::new(TestSink);
    let manager = Arc::new(BackgroundProcesses::new(
        tether_types::SessionId::new("session"),
        sink,
    ));
    let mut tools = RunTools::new(root.path().join("packs")).unwrap();
    tools.linux_off = Some(Arc::new(BackgroundRunner));
    tools.linux_on = tools.linux_off.clone();
    tools.background = Some(Arc::clone(&manager));
    let call = tools
        .prepare(
            "run",
            r#"{"command":"npm run dev","executor":"linux","background":true}"#,
            &PrepareContext { guard: &guard },
        )
        .unwrap();
    let mut before_edit = |_paths: &[std::path::PathBuf]| Ok(());
    let output: ToolOutput = tools
        .execute(
            call,
            ExecuteContext {
                session_id: &tether_types::SessionId::new("session"),
                tool_call_id: &tether_types::ToolCallId::new("call"),
                cancel: CancellationToken::new(),
                sink: None,
                guard: &guard,
                before_edit: &mut before_edit,
            },
        )
        .await
        .unwrap();
    assert!(output.text.contains("process: p1"));
    assert!(output.text.contains("url: http://127.0.0.1:5173/"));
    manager.shutdown();
}
