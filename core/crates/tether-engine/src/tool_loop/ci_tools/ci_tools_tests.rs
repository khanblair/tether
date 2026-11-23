use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tether_ci::CiFailure;
use tether_providers::{ChatProvider, ChatRequest, ProviderError, StopReason, StreamEvent};
use tether_store::{BlobStore, Store};
use tether_types::{Executor, PermissionMode, ProjectId, SessionId, ToolCallId};
use tether_workspace::PathGuard;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::executors::{CiRunRequest, ExecutorError, RunReport, RunStatus};
use crate::permissions::{self, Effect, PermissionRequest};
use crate::{BoxFuture, ExecuteContext, PrepareContext, PreparedCall, ToolHandler, ToolOutput};
use crate::{ToolLoop, ToolLoopContext};

#[derive(Clone)]
struct FakeService {
    run_result: Arc<Mutex<Result<RunReport, String>>>,
    log_result: Arc<Mutex<Result<CiLogResult, String>>>,
    requests: Arc<Mutex<Vec<String>>>,
}

impl FakeService {
    fn success(text: &str) -> Arc<Self> {
        Arc::new(Self {
            run_result: Arc::new(Mutex::new(Ok(RunReport {
                status: RunStatus::Exited(0),
                output: text.into(),
                duration: std::time::Duration::from_secs(3),
                output_truncated: false,
            }))),
            log_result: Arc::new(Mutex::new(Ok(CiLogResult {
                text: "tail".into(),
                failures: Vec::new(),
            }))),
            requests: Arc::new(Mutex::new(Vec::new())),
        })
    }

    fn refusal(message: &str) -> Arc<Self> {
        let service = Self::success("");
        *service.run_result.lock().expect("run lock") = Err(message.into());
        service
    }
}

impl CiRunService for FakeService {
    fn run<'a>(
        &'a self,
        request: CiRunRequest,
        setup: Option<String>,
        _sink: Option<&'a dyn crate::EventSink>,
    ) -> BoxFuture<'a, Result<RunReport, ExecutorError>> {
        self.requests
            .lock()
            .expect("request lock")
            .push(format!("run:{}:{setup:?}", request.command));
        let result = self.run_result.lock().expect("run lock").clone();
        Box::pin(async move { result.map_err(ExecutorError::Failed) })
    }

    fn logs(&self, request: CiLogsRequest) -> BoxFuture<'_, Result<CiLogResult, String>> {
        self.requests
            .lock()
            .expect("request lock")
            .push(format!("logs:{:?}:{:?}", request.run_id, request.job));
        let result = self.log_result.lock().expect("log lock").clone();
        Box::pin(async move { result })
    }
}

fn prepared(handler: &CiTools, args: &str) -> PreparedCall {
    let root = tempfile::tempdir().expect("workspace");
    let guard = tether_workspace::PathGuard::new(root.path()).expect("guard");
    let call = handler
        .prepare("ci_run", args, &PrepareContext { guard: &guard })
        .expect("prepare");
    std::mem::forget(root);
    call
}

async fn execute(handler: &CiTools, call: PreparedCall) -> Result<ToolOutput, String> {
    let root = tempfile::tempdir().expect("workspace");
    let guard = tether_workspace::PathGuard::new(root.path()).expect("guard");
    let session = SessionId::new("session");
    let call_id = ToolCallId::new("call");
    let mut before_edit = |_paths: &[PathBuf]| Ok(());
    handler
        .execute(
            call,
            ExecuteContext {
                session_id: &session,
                tool_call_id: &call_id,
                cancel: CancellationToken::new(),
                sink: None,
                guard: &guard,
                before_edit: &mut before_edit,
            },
        )
        .await
}

#[test]
fn ci_run_prepares_exact_push_target_and_ci_executor() {
    let service = FakeService::success("done");
    let handler = CiTools::new(service, ProjectId::new("project"), "feature/api".into());
    let call = prepared(&handler, r#"{"command":"cargo test","setup":"rust"}"#);

    assert_eq!(call.tool, "ci_run");
    assert_eq!(call.effect, Effect::Push);
    assert_eq!(call.executor, Some(Executor::Ci));
    assert_eq!(
        call.target,
        "push feature/api to origin, then run \"cargo test\" on GitHub Actions"
    );
    assert_eq!(call.display_target(), call.target);
    assert_eq!(call.payload["command"], "cargo test");
    assert_eq!(call.payload["setup"], "rust");
}

#[tokio::test]
async fn ci_tools_return_exact_run_result_and_logs_requests() {
    let service = FakeService::success("run result");
    let service_dyn: Arc<dyn CiRunService> = service.clone();
    let handler = CiTools::new(service_dyn, ProjectId::new("project"), "main".into());
    let result = execute(&handler, prepared(&handler, r#"{"command":"cargo test"}"#))
        .await
        .expect("run");
    assert_eq!(result.text, "run result");

    let log_call = handler
        .prepare(
            "ci_logs",
            r#"{"run_id":42,"job":"build"}"#,
            &PrepareContext {
                guard: &tether_workspace::PathGuard::new(
                    tempfile::tempdir().expect("workspace").path(),
                )
                .expect("guard"),
            },
        )
        .expect("logs prepare");
    let logs = execute(&handler, log_call).await.expect("logs");
    assert_eq!(logs.text, "tail");
    assert_eq!(
        service.requests.lock().expect("request lock").as_slice(),
        ["run:cargo test:None", "logs:Some(42):Some(\"build\")"]
    );
    let default_call = handler
        .prepare(
            "ci_logs",
            "{}",
            &PrepareContext {
                guard: &tether_workspace::PathGuard::new(
                    tempfile::tempdir().expect("workspace").path(),
                )
                .expect("guard"),
            },
        )
        .expect("default logs prepare");
    execute(&handler, default_call).await.expect("default logs");
    assert_eq!(
        service.requests.lock().expect("request lock").as_slice(),
        [
            "run:cargo test:None",
            "logs:Some(42):Some(\"build\")",
            "logs:None:None"
        ]
    );
}

#[tokio::test]
async fn ci_tools_propagate_each_precondition_refusal() {
    for message in [
        "folder is not a git repo",
        "project does not have a GitHub remote",
        "GitHub is not signed in",
        "Commit your changes first",
    ] {
        let service = FakeService::refusal(message);
        let handler = CiTools::new(service, ProjectId::new("project"), "main".into());
        let error = execute(&handler, prepared(&handler, r#"{"command":"cargo test"}"#))
            .await
            .expect_err("refusal");
        assert_eq!(error, format!("executor error: {message}"));
    }
}

#[test]
fn ci_run_permission_modes_ask_and_block_without_rule_bypass() {
    let service = FakeService::success("");
    let handler = CiTools::new(service, ProjectId::new("project"), "main".into());
    let call = prepared(&handler, r#"{"command":"cargo test"}"#);
    let request = PermissionRequest {
        tool: &call.tool,
        target: &call.target,
        executor: call.executor,
    };
    assert_eq!(
        permissions::decide(PermissionMode::Ask, call.effect, &[], request),
        permissions::Decision::Ask
    );
    assert!(matches!(
        permissions::decide(PermissionMode::Plan, call.effect, &[], request),
        permissions::Decision::Block { .. }
    ));
    let rule = tether_types::PermissionRule {
        id: "stored".into(),
        tool: "ci_run".into(),
        pattern: "*".into(),
        executor: None,
    };
    assert_eq!(
        permissions::decide(PermissionMode::Ask, call.effect, &[rule], request),
        permissions::Decision::Ask
    );
}

#[test]
fn ci_result_types_can_render_failure_lines_without_secret_fields() {
    let failure = CiFailure {
        framework: "pytest".into(),
        test_name: Some("test_auth".into()),
        file: Some("tests/test_api.py".into()),
        line: Some(14),
        column: None,
        message: "expected 200".into(),
        excerpt: "expected 200".into(),
    };
    assert_eq!(failure.file.as_deref(), Some("tests/test_api.py"));
    assert!(!format!("{failure:?}").contains("authorization"));
}

#[derive(Default)]
struct CiScript(Mutex<usize>);

impl CiScript {
    /// Returns the zero-based step number and advances the script.
    fn next_step(&self) -> usize {
        let mut step = self.0.lock().expect("script lock");
        let current = *step;
        *step += 1;
        current
    }
}

impl ChatProvider for CiScript {
    fn stream_chat<'a>(
        &'a self,
        request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(script_turn(self, request, events))
    }
}

/// Plays one scripted turn: the tool call first, then the reply.
async fn script_turn(
    script: &CiScript,
    request: &ChatRequest,
    events: mpsc::Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    if script.next_step() == 0 {
        send_script(&events, tool_call_script()).await
    } else {
        assert_tool_result_delivered(request);
        send_script(&events, reply_script()).await
    }
}

/// The first provider turn: one `ci_run` tool call.
fn tool_call_script() -> Vec<StreamEvent> {
    vec![
        StreamEvent::ToolCallStarted {
            id: ToolCallId::new("ci-call"),
            name: "ci_run".into(),
        },
        StreamEvent::ToolCallArgsDelta {
            id: ToolCallId::new("ci-call"),
            json_fragment: r#"{"command":"cargo test"}"#.into(),
        },
        StreamEvent::ToolCallFinished {
            id: ToolCallId::new("ci-call"),
        },
        StreamEvent::Finished {
            stop_reason: StopReason::ToolUse,
        },
    ]
}

/// The second provider turn: the final text reply.
fn reply_script() -> Vec<StreamEvent> {
    vec![
        StreamEvent::TextDelta {
            text: "finished".into(),
        },
        StreamEvent::Finished {
            stop_reason: StopReason::EndTurn,
        },
    ]
}

fn assert_tool_result_delivered(request: &ChatRequest) {
    assert!(request.messages.iter().any(|message| {
        message.parts.iter().any(|part| {
            matches!(part, tether_types::MessagePart::ToolResult { text, .. } if text == "ci done")
        })
    }));
}

async fn send_script(
    events: &mpsc::Sender<StreamEvent>,
    script: Vec<StreamEvent>,
) -> Result<(), ProviderError> {
    for event in script {
        events
            .send(event)
            .await
            .map_err(|_| ProviderError::Cancelled)?;
    }
    Ok(())
}

#[tokio::test]
async fn ci_run_flows_through_tool_loop_and_approves_the_exact_target() {
    let root = tempfile::tempdir().expect("workspace");
    let database = root.path().join("store.db");
    seed_ci_session(&database);
    let mut store = Store::open(&database).expect("store reopen");
    let blobs = BlobStore::open(root.path().join("blobs")).expect("blobs");
    let loop_runner = ci_tool_loop(root.path());
    let session = SessionId::new("session");
    let (approve, approvals) = recording_approver();
    let mode = || PermissionMode::Ask;
    let mut context = ToolLoopContext {
        session_id: &session,
        store: &mut store,
        blobs: &blobs,
        cancel: CancellationToken::new(),
        approver: &approve,
        mode: &mode,
        sink: None,
    };
    let result = loop_runner
        .run(
            Arc::new(CiScript::default()),
            ChatRequest {
                model_id: "fixture".into(),
                system: String::new(),
                messages: Vec::new(),
                tools: Vec::new(),
                max_output_tokens: 100,
                reasoning: None,
            },
            &mut context,
        )
        .await
        .expect("tool loop");
    assert_eq!(result.final_text, "finished");
    assert_eq!(
        approvals.lock().expect("approval lock").as_slice(),
        &[(
            "ci_run".into(),
            "push feature/api to origin, then run \"cargo test\" on GitHub Actions".into()
        )]
    );
}

/// Creates the project and session rows the CI tool loop test needs.
fn seed_ci_session(database: &std::path::Path) {
    drop(Store::open(database).expect("store"));
    let connection = rusqlite::Connection::open(database).expect("connection");
    connection
        .execute_batch(
            "INSERT INTO projects (id, name, path, created_at) VALUES ('project', 'fixture', 'fixture', 1);
             INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
             VALUES ('session', 'project', 'openai', 'fixture', 'ask', 'idle', 1, 1);",
        )
        .expect("session rows");
}

fn ci_tool_loop(workspace: &std::path::Path) -> ToolLoop {
    let service = FakeService::success("ci done");
    let handler = Arc::new(CiTools::new(
        service,
        ProjectId::new("project"),
        "feature/api".into(),
    ));
    let guard = PathGuard::new(workspace).expect("guard");
    ToolLoop::with_handlers(guard, vec![handler]).expect("registry")
}

type Approvals = Arc<Mutex<Vec<(String, String)>>>;

/// An approver that always says yes and records each (tool, target) pair.
fn recording_approver() -> (impl Fn(&str, &str) -> bool + Send + Sync, Approvals) {
    let approvals = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&approvals);
    let approve = move |tool: &str, target: &str| {
        recorded
            .lock()
            .expect("approval lock")
            .push((tool.to_owned(), target.to_owned()));
        true
    };
    (approve, approvals)
}
