//! User-initiated command runs and reruns.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use tether_engine::{ExecuteContext, PrepareContext, ToolHandler};
use tether_types::{CoreEvent, Executor, MessagePart, Role, SessionId, ToolCallId, ToolCallStatus};
use tether_workspace::PathGuard;

use super::{SessionRuntime, SessionRuntimeError, now_ms};
use crate::ExecutorView;

static NEXT_USER_RUN: AtomicU64 = AtomicU64::new(0);

#[uniffi::export]
impl SessionRuntime {
    /// Runs a command typed by the user without requesting approval.
    ///
    /// # Errors
    /// Returns an error when the session, workspace, or executor is unavailable.
    pub fn run_user_command(
        &self,
        session_id: String,
        command: &str,
        executor: Option<ExecutorView>,
    ) -> Result<String, SessionRuntimeError> {
        if command.trim().is_empty() {
            return Err(SessionRuntimeError::EmptyInput);
        }
        let id = SessionId::new(session_id);
        self.runtime
            .block_on(run_command(
                self,
                &id,
                command,
                RunOptions {
                    executor: executor.map(executor_from_view),
                    cwd: None,
                    timeout: None,
                },
            ))
            .map_err(|error| match error {
                RunError::NotFound => SessionRuntimeError::NotFound,
                RunError::Busy => SessionRuntimeError::Busy,
                RunError::Unavailable => SessionRuntimeError::Unavailable,
            })
    }

    /// Reruns a saved command with a different executor and no approval.
    ///
    /// # Errors
    /// Returns an error when the saved call or target executor is unavailable.
    pub fn rerun_elsewhere(
        &self,
        session_id: String,
        tool_call_id: String,
        executor: ExecutorView,
    ) -> Result<String, SessionRuntimeError> {
        let id = SessionId::new(session_id);
        let store = self.store()?;
        let call_id = ToolCallId::new(tool_call_id);
        let saved = store
            .messages(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .into_iter()
            .flat_map(|message| message.parts.into_iter())
            .find_map(|part| match part {
                MessagePart::ToolCall {
                    id,
                    tool,
                    args_json,
                } if id == call_id && tool == "run" => Some(args_json),
                _ => None,
            })
            .ok_or(SessionRuntimeError::NotFound)?;
        let value: Value =
            serde_json::from_str(&saved).map_err(|_| SessionRuntimeError::Unavailable)?;
        let command = value
            .get("command")
            .and_then(Value::as_str)
            .ok_or(SessionRuntimeError::Unavailable)?;
        let cwd = value.get("cwd").and_then(Value::as_str).map(str::to_owned);
        let timeout = value.get("timeout_seconds").and_then(Value::as_u64);
        self.runtime
            .block_on(run_command(
                self,
                &id,
                command,
                RunOptions {
                    executor: Some(executor_from_view(executor)),
                    cwd,
                    timeout,
                },
            ))
            .map_err(|error| match error {
                RunError::NotFound => SessionRuntimeError::NotFound,
                RunError::Busy => SessionRuntimeError::Busy,
                RunError::Unavailable => SessionRuntimeError::Unavailable,
            })
    }
}

enum RunError {
    NotFound,
    Busy,
    Unavailable,
}

struct RunOptions {
    executor: Option<Executor>,
    cwd: Option<String>,
    timeout: Option<u64>,
}

#[expect(
    clippy::too_many_lines,
    reason = "A user run keeps transcript and event ordering together."
)]
async fn run_command(
    runtime: &SessionRuntime,
    session_id: &SessionId,
    command: &str,
    options: RunOptions,
) -> Result<String, RunError> {
    let mut store = runtime.store().map_err(|_| RunError::Unavailable)?;
    let session = store
        .load_session(session_id)
        .map_err(|_| RunError::Unavailable)?
        .ok_or(RunError::NotFound)?;
    if session.status != tether_types::SessionStatus::Idle {
        return Err(RunError::Busy);
    }
    let project_root = runtime
        .project_roots
        .lock()
        .map_err(|_| RunError::Unavailable)?
        .get(session_id)
        .cloned()
        .ok_or(RunError::Unavailable)?;
    let guard = PathGuard::new(&project_root).map_err(|_| RunError::Unavailable)?;
    let handler = crate::executors::session_run_tools(
        Path::new(&runtime.database_path),
        &project_root,
        &session.project_id,
    )
    .map_err(|_| RunError::Unavailable)?;
    let args = run_args(command, &options);
    let prepared = handler
        .prepare("run", &args, &PrepareContext { guard: &guard })
        .map_err(|_| RunError::Unavailable)?;
    let call_id = next_call_id();
    store
        .append_message(
            session_id,
            Role::Assistant,
            &[MessagePart::ToolCall {
                id: call_id.clone(),
                tool: "run".into(),
                args_json: args,
            }],
            now_ms(),
        )
        .map_err(|_| RunError::Unavailable)?;
    let sink = runtime.session_sink();
    sink.on_event(CoreEvent::ToolProposed {
        session_id: session_id.clone(),
        tool_call_id: call_id.clone(),
        tool: "run".into(),
        target: prepared.display_target(),
    });
    sink.on_event(CoreEvent::ToolStarted {
        session_id: session_id.clone(),
        tool_call_id: call_id.clone(),
    });
    store
        .set_session_status(
            session_id,
            tether_types::SessionStatus::RunningTool,
            now_ms(),
        )
        .map_err(|_| RunError::Unavailable)?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut before_edit = |_paths: &[std::path::PathBuf]| Ok(());
    let result = handler
        .execute(
            prepared,
            ExecuteContext {
                session_id,
                tool_call_id: &call_id,
                cancel,
                sink: Some(sink.as_ref()),
                guard: &guard,
                before_edit: &mut before_edit,
            },
        )
        .await;
    let (is_error, text, status) = match result {
        Ok(output) => (false, output.text, ToolCallStatus::Succeeded),
        Err(error) => (true, error, ToolCallStatus::Failed),
    };
    store
        .append_message(
            session_id,
            Role::Tool,
            &[MessagePart::ToolResult {
                tool_call_id: call_id.clone(),
                is_error,
                text,
                result_ref: None,
            }],
            now_ms(),
        )
        .map_err(|_| RunError::Unavailable)?;
    store
        .set_session_status(session_id, tether_types::SessionStatus::Idle, now_ms())
        .map_err(|_| RunError::Unavailable)?;
    sink.on_event(CoreEvent::ToolFinished {
        session_id: session_id.clone(),
        tool_call_id: call_id.clone(),
        status,
    });
    Ok(call_id.to_string())
}

fn run_args(command: &str, options: &RunOptions) -> String {
    let mut value = json!({"command": command, "user": true});
    if let Some(executor) = options.executor {
        value["executor"] = Value::String(executor.as_str().into());
    }
    if let Some(cwd) = &options.cwd {
        value["cwd"] = Value::String(cwd.clone());
    }
    if let Some(timeout) = options.timeout {
        value["timeout_seconds"] = json!(timeout);
    }
    value.to_string()
}

fn next_call_id() -> ToolCallId {
    let sequence = NEXT_USER_RUN.fetch_add(1, Ordering::Relaxed);
    ToolCallId::new(format!("user-run-{}-{sequence}", now_ms()))
}

fn executor_from_view(view: ExecutorView) -> Executor {
    match view {
        ExecutorView::Sandbox => Executor::Sandbox,
        ExecutorView::Linux => Executor::Linux,
        ExecutorView::Ci => Executor::Ci,
    }
}
