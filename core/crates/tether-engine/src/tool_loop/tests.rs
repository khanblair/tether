use std::fs;
use std::sync::{Arc, Mutex};

use tether_providers::{
    BoxFuture, ChatProvider, ChatRequest, ProviderError, StopReason, StreamEvent,
};
use tether_store::{BlobStore, Store};
use tether_types::{CoreEvent, MessagePart, Role, SessionId, ToolCallId};
use tether_workspace::PathGuard;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{ToolLoop, ToolLoopContext};

struct ProposalSink(Arc<Mutex<Vec<&'static str>>>);

impl crate::EventSink for ProposalSink {
    fn on_event(&self, event: CoreEvent) {
        if matches!(event, CoreEvent::ToolProposed { tool, target, .. } if tool == "edit_file" && target == "note.txt")
        {
            self.0.lock().unwrap().push("proposed");
        }
    }
}

#[derive(Default)]
struct Script(Mutex<usize>);

impl ChatProvider for Script {
    #[expect(
        clippy::too_many_lines,
        clippy::excessive_nesting,
        reason = "the scripted provider checks each exchange in order"
    )]
    fn stream_chat<'a>(
        &'a self,
        request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            let step = {
                let mut count = self.0.lock().unwrap();
                let step = *count;
                *count += 1;
                step
            };
            assert_eq!(request.messages.len(), step * 2);
            assert_eq!(request.tools.len(), 9);
            if step > 0 {
                assert!(matches!(
                    &request.messages[0].parts[0],
                    MessagePart::Thinking { text } if text == "reasoning"
                ));
            }
            let calls = [
                ("list_dir", "{}"),
                ("read_file", r#"{"path":"note.txt"}"#),
                ("grep", r#"{"pattern":"old"}"#),
                (
                    "edit_file",
                    r#"{"path":"note.txt","old":"old","new":"new"}"#,
                ),
                ("read_file", r#"{"path":"note.txt"}"#),
            ];
            if let Some((name, args)) = calls.get(step) {
                if step == 0 {
                    send(
                        &events,
                        StreamEvent::ThinkingDelta {
                            text: "reasoning".into(),
                        },
                    )
                    .await?;
                }
                let id = ToolCallId::new(format!("call-{step}"));
                send(
                    &events,
                    StreamEvent::ToolCallStarted {
                        id: id.clone(),
                        name: (*name).into(),
                    },
                )
                .await?;
                for fragment in args.as_bytes().chunks(3) {
                    send(
                        &events,
                        StreamEvent::ToolCallArgsDelta {
                            id: id.clone(),
                            json_fragment: String::from_utf8(fragment.to_vec()).unwrap(),
                        },
                    )
                    .await?;
                }
                send(&events, StreamEvent::ToolCallFinished { id }).await?;
                send(
                    &events,
                    StreamEvent::Finished {
                        stop_reason: StopReason::ToolUse,
                    },
                )
                .await?;
            } else {
                assert_eq!(step, 5);
                assert!(request.messages.last().unwrap().parts.iter().any(|part| {
                    matches!(part, MessagePart::ToolResult { text, .. } if text.contains("new"))
                }));
                send(
                    &events,
                    StreamEvent::TextDelta {
                        text: "Done".into(),
                    },
                )
                .await?;
                send(
                    &events,
                    StreamEvent::Finished {
                        stop_reason: StopReason::EndTurn,
                    },
                )
                .await?;
            }
            Ok(())
        })
    }
}

async fn send(events: &mpsc::Sender<StreamEvent>, event: StreamEvent) -> Result<(), ProviderError> {
    events
        .send(event)
        .await
        .map_err(|_| ProviderError::Cancelled)
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "the five-step fixture asserts persistence and the prior edit snapshot"
)]
async fn five_step_read_and_edit_fixture_completes() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    fs::write(workspace.join("note.txt"), "old\n").unwrap();
    let database = root.path().join("store.db");
    drop(Store::open(&database).unwrap());
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection.execute_batch("INSERT INTO projects (id, name, path, created_at) VALUES ('project', 'fixture', 'fixture', 1);
        INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
        VALUES ('fixture', 'project', 'openai', 'fixture', 'ask', 'idle', 1, 1);").unwrap();
    drop(connection);
    let mut store = Store::open(&database).unwrap();
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let id = SessionId::new("fixture");
    let loop_runner = ToolLoop::new(PathGuard::new(&workspace).unwrap());
    let approvals = Arc::new(Mutex::new(Vec::new()));
    let order = Arc::new(Mutex::new(Vec::new()));
    let sink = ProposalSink(Arc::clone(&order));
    let approved = Arc::clone(&approvals);
    let approved_order = Arc::clone(&order);
    let approve = move |name: &str, target: &str| {
        approved_order.lock().unwrap().push("approved");
        approved
            .lock()
            .unwrap()
            .push((name.to_owned(), target.to_owned()));
        true
    };
    let mut context = ToolLoopContext {
        session_id: &id,
        store: &mut store,
        blobs: &blobs,
        cancel: CancellationToken::new(),
        approve: &approve,
        sink: Some(&sink),
    };
    let result = loop_runner
        .run(
            Arc::new(Script::default()),
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
        .unwrap();
    drop(context);
    assert_eq!(result.exchanges, 6);
    assert_eq!(result.final_text, "Done");
    assert_eq!(
        fs::read_to_string(workspace.join("note.txt")).unwrap(),
        "new\n"
    );
    assert_eq!(
        approvals.lock().unwrap().as_slice(),
        &[("edit_file".into(), "note.txt".into())]
    );
    assert_eq!(order.lock().unwrap().as_slice(), &["proposed", "approved"]);
    assert_eq!(result.messages.len(), 11);
    assert_eq!(result.messages[1].role, Role::Tool);
    assert_eq!(store.messages(&id).unwrap(), result.messages);
    let connection = rusqlite::Connection::open(&database).unwrap();
    let checkpoint_id: String = connection
        .query_row("SELECT id FROM checkpoints", [], |row| row.get(0))
        .unwrap();
    let checkpoint = store.load_checkpoint(&checkpoint_id).unwrap().unwrap();
    assert_eq!(checkpoint.files.len(), 1);
    assert_eq!(
        blobs
            .get(checkpoint.files[0].blob_hash.as_deref().unwrap())
            .unwrap(),
        b"old\n"
    );
}

#[test]
fn invalid_arguments_and_denial_do_not_mutate() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("note.txt"), "old").unwrap();
    let loop_runner = ToolLoop::new(PathGuard::new(root.path()).unwrap());
    let denied = super::dispatch::run(
        &loop_runner,
        "write_file",
        r#"{"path":"note.txt","text":"new"}"#,
        &|_, _| false,
        &mut |_| Ok(()),
    );
    assert_eq!(denied.unwrap_err(), "tool approval denied");
    let malformed = super::dispatch::run(
        &loop_runner,
        "read_file",
        r#"{"path":12}"#,
        &|_, _| true,
        &mut |_| Ok(()),
    );
    assert!(malformed.unwrap_err().contains("path must be a string"));
    assert_eq!(
        fs::read_to_string(root.path().join("note.txt")).unwrap(),
        "old"
    );
}
