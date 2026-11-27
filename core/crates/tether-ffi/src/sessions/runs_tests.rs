use std::sync::Arc;

use super::super::*;
use crate::{ExecutorView, RunOutcomeView, SecretsPlatform, SecretsPlatformFailure};
use tether_store::Store;
use tether_types::{
    MessagePart, ModelRef, PermissionMode, ProjectId, Provider, Role, SessionId, ToolCallId,
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

impl crate::EventSink for NoopSink {
    fn on_event(&self, _event: crate::CoreEventView) {}
}

fn fixture() -> (tempfile::TempDir, Arc<SessionRuntime>, ProjectId, SessionId) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let mut store = Store::open(&db).unwrap();
    let project = store.create_project("Project", 1).unwrap();
    let session = store
        .create_session(
            &project.id,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "model".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    drop(store);
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    (dir, runtime, project.id, session.id)
}

fn append_call(db: &std::path::Path, session: &SessionId, id: &str, args: &str, at: i64) {
    Store::open(db)
        .unwrap()
        .append_message(
            session,
            Role::Assistant,
            &[MessagePart::ToolCall {
                id: ToolCallId::new(id),
                tool: "run".into(),
                args_json: args.into(),
            }],
            at,
        )
        .unwrap();
}

fn append_result(db: &std::path::Path, session: &SessionId, id: &str, error: bool, text: &str) {
    Store::open(db)
        .unwrap()
        .append_message(
            session,
            Role::Tool,
            &[MessagePart::ToolResult {
                tool_call_id: ToolCallId::new(id),
                is_error: error,
                text: text.into(),
                result_ref: None,
            }],
            2,
        )
        .unwrap();
}

#[test]
fn unknown_project_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    Store::open(&db).unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();

    assert!(matches!(
        runtime.list_runs("missing".into(), 20),
        Err(SessionRuntimeError::NotFound)
    ));
}

#[test]
fn run_view_cuts_a_multibyte_command_at_500_characters() {
    let (dir, runtime, project, session) = fixture();
    let command = "é".repeat(501);
    let args = serde_json::json!({"command": command}).to_string();
    append_call(&dir.path().join("tether.db"), &session, "long", &args, 10);

    let view = runtime.list_runs(project.into(), 10).unwrap().remove(0);
    assert_eq!(view.command.chars().count(), 500);
    assert_eq!(view.command, "é".repeat(500));
}

#[test]
fn run_view_falls_back_to_the_call_executor() {
    let (dir, runtime, project, session) = fixture();
    let db = dir.path().join("tether.db");
    append_call(
        &db,
        &session,
        "cancelled",
        r#"{"command":"cargo test","executor":"linux"}"#,
        10,
    );
    append_result(
        &db,
        &session,
        "cancelled",
        true,
        "tool cancelled before execution",
    );

    let view = runtime.list_runs(project.into(), 10).unwrap().remove(0);
    assert_eq!(view.executor, Some(ExecutorView::Linux));
    assert_eq!(view.outcome, RunOutcomeView::Cancelled);
}

#[test]
fn run_view_marks_a_background_result_as_linux_background() {
    let (dir, runtime, project, session) = fixture();
    let db = dir.path().join("tether.db");
    append_call(
        &db,
        &session,
        "server",
        r#"{"command":"npm run dev","executor":"linux","background":true}"#,
        10,
    );
    append_result(
        &db,
        &session,
        "server",
        false,
        "process: p1\nport: 5173\nurl: http://127.0.0.1:5173/\noutput:\nready",
    );

    let view = runtime.list_runs(project.into(), 10).unwrap().remove(0);
    assert_eq!(view.executor, Some(ExecutorView::Linux));
    assert_eq!(view.outcome, RunOutcomeView::Background);
    assert!(view.background);
    assert_eq!(view.duration_ms, None);
}

#[test]
fn malformed_arguments_are_skipped_without_failing_the_list() {
    let (dir, runtime, project, session) = fixture();
    append_call(
        &dir.path().join("tether.db"),
        &session,
        "malformed",
        "not-json",
        10,
    );

    assert!(runtime.list_runs(project.into(), 10).unwrap().is_empty());
}
