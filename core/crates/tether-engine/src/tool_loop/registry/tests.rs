use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::json;
use tether_providers::{BoxFuture, ToolSpec};
use tether_types::{CoreEvent, Executor, SessionId, ToolCallId};
use tokio_util::sync::CancellationToken;

use super::super::ToolLoop;
use super::super::registry::{
    ExecuteContext, PrepareContext, PreparedCall, ToolHandler, ToolOutput, ToolRegistry,
};
use crate::EventSink;
use crate::permissions::Effect;

struct Events(Arc<Mutex<Vec<String>>>);

impl EventSink for Events {
    fn on_event(&self, event: CoreEvent) {
        if let CoreEvent::ToolOutput { chunk, .. } = event {
            self.0.lock().expect("lock").push(chunk);
        }
    }
}

#[test]
fn every_default_spec_prepares_with_a_known_effect() {
    let root = tempfile::tempdir().unwrap();
    let loop_runner = ToolLoop::new(tether_workspace::PathGuard::new(root.path()).unwrap());
    let guard = tether_workspace::PathGuard::new(root.path()).unwrap();
    let args = [
        ("read_file", json!({"path":"a.txt"})),
        ("list_dir", json!({})),
        ("glob", json!({"pattern":"*.rs"})),
        ("grep", json!({"pattern":"x"})),
        ("write_file", json!({"path":"a.txt","text":"x"})),
        ("edit_file", json!({"path":"a.txt","old":"x","new":"y"})),
        (
            "multi_edit",
            json!({"path":"a.txt","edits":[{"old":"x","new":"y"}]}),
        ),
        ("move_file", json!({"from":"a.txt","to":"b.txt"})),
        ("delete_file", json!({"path":"a.txt"})),
    ];
    for spec in loop_runner.specs() {
        let (_, value) = args.iter().find(|(name, _)| *name == spec.name).unwrap();
        let call = loop_runner
            .registry
            .prepare(
                &spec.name,
                &value.to_string(),
                &PrepareContext { guard: &guard },
            )
            .unwrap();
        assert_ne!(call.effect, Effect::Unknown, "{}", spec.name);
    }
    assert_eq!(
        loop_runner
            .registry
            .prepare("later_tool", "{}", &PrepareContext { guard: &guard })
            .unwrap()
            .effect,
        Effect::Unknown
    );
}

struct Fake;

impl ToolHandler for Fake {
    fn specs(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            name: "fake".into(),
            description: "fake".into(),
            input_schema_json: "{}".into(),
        }]
    }

    fn prepare(
        &self,
        name: &str,
        _args_json: &str,
        _context: &PrepareContext<'_>,
    ) -> Result<PreparedCall, String> {
        Ok(PreparedCall {
            tool: name.into(),
            effect: Effect::Internal,
            target: "fake".into(),
            executor: Some(Executor::Sandbox),
            payload: json!({}),
        })
    }

    fn execute<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> BoxFuture<'a, Result<ToolOutput, String>> {
        Box::pin(async move {
            emit_chunks(&context);
            Ok(ToolOutput { text: call.target })
        })
    }
}

fn emit_chunks(context: &ExecuteContext<'_>) {
    for chunk in ["one", "two"] {
        emit_chunk(context, chunk);
    }
}

fn emit_chunk(context: &ExecuteContext<'_>, chunk: &str) {
    if let Some(sink) = context.sink {
        sink.on_event(CoreEvent::ToolOutput {
            session_id: context.session_id.clone(),
            tool_call_id: context.tool_call_id.clone(),
            chunk: chunk.into(),
        });
    }
}

#[tokio::test]
async fn a_handler_can_stream_output_and_return_a_result() {
    let registry = ToolRegistry::new(vec![Arc::new(Fake)]).unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Events(Arc::clone(&events));
    let guard_root = tempfile::tempdir().unwrap();
    let guard = tether_workspace::PathGuard::new(guard_root.path()).unwrap();
    let call = registry
        .prepare("fake", "{}", &PrepareContext { guard: &guard })
        .unwrap();
    let session = SessionId::new("session");
    let call_id = ToolCallId::new("call");
    let mut before_edit = |_: &[PathBuf]| Ok(());
    let output = registry
        .execute(
            call,
            ExecuteContext {
                session_id: &session,
                tool_call_id: &call_id,
                cancel: CancellationToken::new(),
                sink: Some(&sink),
                guard: &guard,
                before_edit: &mut before_edit,
            },
        )
        .await
        .unwrap();
    assert_eq!(output.text, "fake");
    assert_eq!(events.lock().unwrap().as_slice(), ["one", "two"]);
}
