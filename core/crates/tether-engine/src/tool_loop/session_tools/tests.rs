use std::sync::Arc;
use std::sync::Mutex;

use tether_store::Store;
use tether_types::{CoreEvent, ModelRef, PermissionMode, Provider, SessionId};

use super::*;
use crate::EventSink;

struct NoopAsker;

impl Asker for NoopAsker {
    fn interactive(&self) -> bool {
        false
    }

    fn ask(
        &self,
        _request: QuestionRequest,
        _cancel: tokio_util::sync::CancellationToken,
    ) -> crate::BoxFuture<'_, Answer> {
        Box::pin(std::future::ready(Answer::Cancelled))
    }
}

struct FixedAsker {
    answer: Answer,
    interactive: bool,
}

impl Asker for FixedAsker {
    fn interactive(&self) -> bool {
        self.interactive
    }

    fn ask(
        &self,
        _request: QuestionRequest,
        _cancel: tokio_util::sync::CancellationToken,
    ) -> crate::BoxFuture<'_, Answer> {
        let answer = self.answer.clone();
        Box::pin(std::future::ready(answer))
    }
}

struct Events(Mutex<Vec<CoreEvent>>);

impl EventSink for Events {
    fn on_event(&self, event: CoreEvent) {
        self.0.lock().unwrap().push(event);
    }
}

fn handler() -> SessionTools {
    SessionTools::new(
        Arc::new(NoopAsker),
        Arc::new(std::sync::Mutex::new(
            tether_store::Store::open_in_memory().unwrap(),
        )),
    )
}

fn context(root: &std::path::Path) -> PrepareContext<'_> {
    PrepareContext {
        guard: Box::leak(Box::new(tether_workspace::PathGuard::new(root).unwrap())),
    }
}

fn session_store() -> (Arc<Mutex<Store>>, SessionId) {
    let mut store = Store::open_in_memory().unwrap();
    let project = store.create_project("Tools", 1).unwrap();
    let session = store
        .create_session(
            &project.id,
            &ModelRef {
                provider: Provider::OpenAi,
                model_id: "model".into(),
            },
            PermissionMode::Plan,
            1,
        )
        .unwrap();
    (Arc::new(Mutex::new(store)), session.id)
}

fn execute_context<'a>(
    session_id: &'a SessionId,
    tool_call_id: &'a tether_types::ToolCallId,
    sink: Option<&'a Events>,
    root: &'a tether_workspace::PathGuard,
    before_edit: &'a mut (dyn FnMut(&[std::path::PathBuf]) -> Result<(), String> + Send),
) -> ExecuteContext<'a> {
    ExecuteContext {
        session_id,
        tool_call_id,
        cancel: tokio_util::sync::CancellationToken::new(),
        sink: sink.map(|value| value as &dyn EventSink),
        guard: root,
        before_edit,
    }
}

#[test]
fn validates_todo_limits_and_unique_active_item() {
    let root = tempfile::tempdir().unwrap();
    let handler = handler();
    let context = context(root.path());
    let empty = handler.prepare("todo_write", r#"{"todos":[]}"#, &context);
    assert_eq!(empty.unwrap_err(), "todos must contain 1 to 30 items");
    let too_long = format!(
        r#"{{"todos":[{{"text":"{}","status":"pending"}}]}}"#,
        "a".repeat(201)
    );
    assert_eq!(
        handler
            .prepare("todo_write", &too_long, &context)
            .unwrap_err(),
        "todo text must contain 1 to 200 characters"
    );
    let two_active =
        r#"{"todos":[{"text":"a","status":"in_progress"},{"text":"b","status":"in_progress"}]}"#;
    assert_eq!(
        handler
            .prepare("todo_write", two_active, &context)
            .unwrap_err(),
        "todos may contain at most one in_progress item"
    );
    assert_eq!(
        handler
            .prepare(
                "todo_write",
                r#"{"todos":[{"text":"a","status":"pending"}],"extra":1}"#,
                &context
            )
            .unwrap_err(),
        "unknown tool argument: extra"
    );
}

#[test]
fn validates_question_defaults_and_types() {
    let root = tempfile::tempdir().unwrap();
    let handler = handler();
    let context = context(root.path());
    let prepared = handler
        .prepare("ask_user", r#"{"question":"Choose"}"#, &context)
        .unwrap();
    assert_eq!(prepared.target, "Choose");
    assert_eq!(prepared.payload["allow_free_text"], true);
    assert_eq!(
        handler
            .prepare("ask_user", r#"{"question":12}"#, &context)
            .unwrap_err(),
        "question must be a string"
    );
    assert_eq!(
        handler
            .prepare("ask_user", r#"{"question":"q","options":[1]}"#, &context)
            .unwrap_err(),
        "each options item must be a string"
    );
}

#[test]
fn validates_plan_paths_and_advertising() {
    let root = tempfile::tempdir().unwrap();
    let handler = handler();
    let context = context(root.path());
    let prepared = handler
        .prepare(
            "submit_plan",
            r#"{"summary":"s","steps":["a"],"files":["new.txt"]}"#,
            &context,
        )
        .unwrap();
    assert_eq!(prepared.target, "plan");
    assert_eq!(
        handler
            .prepare(
                "submit_plan",
                r#"{"summary":"s","steps":["a"],"files":["../escape"]}"#,
                &context
            )
            .unwrap_err(),
        "files must contain workspace-relative paths"
    );
    let plan_spec = SESSION_TOOL_SPECS
        .iter()
        .find(|spec| spec.name == "submit_plan")
        .unwrap();
    assert!(plan_spec.advertised_in(PermissionMode::Plan));
    assert!(!plan_spec.advertised_in(PermissionMode::Ask));
}

#[tokio::test]
async fn plan_execution_emits_exact_plan_and_short_result() {
    let root = tempfile::tempdir().unwrap();
    let guard = tether_workspace::PathGuard::new(root.path()).unwrap();
    let (store, session_id) = session_store();
    let handler = SessionTools::new(Arc::new(NoopAsker), store);
    let call_id = tether_types::ToolCallId::new("plan-call");
    let call = handler
        .prepare(
            "submit_plan",
            r#"{"summary":"Ship","steps":["test"],"files":["src/lib.rs"],"commands":[],"open_questions":[]}"#,
            &PrepareContext { guard: &guard },
        )
        .unwrap();
    let events = Events(Mutex::new(Vec::new()));
    let mut before_edit = |_: &[std::path::PathBuf]| Ok(());
    let output = handler
        .execute(
            call,
            execute_context(
                &session_id,
                &call_id,
                Some(&events),
                &guard,
                &mut before_edit,
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        output.text,
        "Plan shown to the user. Stop and wait for their decision."
    );
    assert!(matches!(
        events.0.lock().unwrap().as_slice(),
        [CoreEvent::PlanReady { plan, .. }] if plan.summary == "Ship" && plan.files.len() == 1 && plan.files[0] == "src/lib.rs"
    ));
}

#[tokio::test]
async fn todo_execution_replaces_store_and_emits_exact_items() {
    let root = tempfile::tempdir().unwrap();
    let guard = tether_workspace::PathGuard::new(root.path()).unwrap();
    let (store, session_id) = session_store();
    let handler = SessionTools::new(Arc::new(NoopAsker), Arc::clone(&store));
    let call_id = tether_types::ToolCallId::new("todo-call");
    let call = handler
        .prepare(
            "todo_write",
            r#"{"todos":[{"text":"done","status":"completed"},{"text":"next","status":"pending"}]}"#,
            &PrepareContext { guard: &guard },
        )
        .unwrap();
    let events = Events(Mutex::new(Vec::new()));
    let mut before_edit = |_: &[std::path::PathBuf]| Ok(());
    let output = handler
        .execute(
            call,
            execute_context(
                &session_id,
                &call_id,
                Some(&events),
                &guard,
                &mut before_edit,
            ),
        )
        .await
        .unwrap();
    assert_eq!(output.text, "checklist updated: 1 of 2 done");
    assert!(matches!(
        events.0.lock().unwrap().as_slice(),
        [CoreEvent::TodoUpdated { items, .. }] if items.len() == 2
    ));
    assert_eq!(store.lock().unwrap().todos(&session_id).unwrap().len(), 2);
}

#[tokio::test]
async fn question_execution_returns_choice_text_and_event() {
    let root = tempfile::tempdir().unwrap();
    let guard = tether_workspace::PathGuard::new(root.path()).unwrap();
    let (store, session_id) = session_store();
    let handler = SessionTools::new(
        Arc::new(FixedAsker {
            answer: Answer::Text("A".into()),
            interactive: true,
        }),
        Arc::clone(&store),
    );
    let call_id = tether_types::ToolCallId::new("question-call");
    let call = handler
        .prepare(
            "ask_user",
            r#"{"question":"Pick","options":["A","B"],"allow_free_text":false}"#,
            &PrepareContext { guard: &guard },
        )
        .unwrap();
    let events = Events(Mutex::new(Vec::new()));
    let mut before_edit = |_: &[std::path::PathBuf]| Ok(());
    let output = handler
        .execute(
            call,
            execute_context(
                &session_id,
                &call_id,
                Some(&events),
                &guard,
                &mut before_edit,
            ),
        )
        .await
        .unwrap();
    assert_eq!(output.text, "The user chose: A");
    assert!(events.0.lock().unwrap().iter().any(
        |event| matches!(event, CoreEvent::QuestionAsked { question, .. } if question == "Pick")
    ));
}

#[tokio::test]
async fn question_execution_returns_cancel_text() {
    let root = tempfile::tempdir().unwrap();
    let guard = tether_workspace::PathGuard::new(root.path()).unwrap();
    let (store, session_id) = session_store();
    let cancelled = SessionTools::new(
        Arc::new(FixedAsker {
            answer: Answer::Cancelled,
            interactive: true,
        }),
        store,
    );
    let call_id = tether_types::ToolCallId::new("cancel-call");
    let call = cancelled
        .prepare(
            "ask_user",
            r#"{"question":"Again"}"#,
            &PrepareContext { guard: &guard },
        )
        .unwrap();
    let mut before_edit = |_: &[std::path::PathBuf]| Ok(());
    let output = cancelled
        .execute(
            call,
            execute_context(&session_id, &call_id, None, &guard, &mut before_edit),
        )
        .await
        .unwrap();
    assert_eq!(output.text, "The user did not answer");
}

#[tokio::test]
async fn question_interactive_off_returns_without_an_event() {
    let root = tempfile::tempdir().unwrap();
    let guard = tether_workspace::PathGuard::new(root.path()).unwrap();
    let (store, session_id) = session_store();
    let handler = SessionTools::new(
        Arc::new(FixedAsker {
            answer: Answer::Text("unused".into()),
            interactive: false,
        }),
        store,
    );
    let call_id = tether_types::ToolCallId::new("off-call");
    let call = handler
        .prepare(
            "ask_user",
            r#"{"question":"Unavailable"}"#,
            &PrepareContext { guard: &guard },
        )
        .unwrap();
    let events = Events(Mutex::new(Vec::new()));
    let mut before_edit = |_: &[std::path::PathBuf]| Ok(());
    let output = handler
        .execute(
            call,
            execute_context(
                &session_id,
                &call_id,
                Some(&events),
                &guard,
                &mut before_edit,
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        output.text,
        "No one is available to answer. Continue with your best assumption and say which assumption you made."
    );
    assert!(events.0.lock().unwrap().is_empty());
}
