use std::fs;
use std::sync::{Arc, Mutex};

use tether_providers::{BoxFuture, ChatProvider, ChatRequest, ProviderError, StopReason, StreamEvent};
use tether_types::{MessagePart, Role, SessionId, ToolCallId};
use tether_workspace::PathGuard;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::ToolLoop;

#[derive(Default)]
struct Script(Mutex<usize>);

impl ChatProvider for Script {
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
            let calls = [
                ("list_dir", r#"{}"#),
                ("read_file", r#"{"path":"note.txt"}"#),
                ("grep", r#"{"needle":"old"}"#),
                ("edit_file", r#"{"path":"note.txt","old":"old","new":"new"}"#),
                ("read_file", r#"{"path":"note.txt"}"#),
            ];
            if let Some((name, args)) = calls.get(step) {
                let id = ToolCallId::new(format!("call-{step}"));
                send(&events, StreamEvent::ToolCallStarted { id: id.clone(), name: (*name).into() }).await?;
                for fragment in args.as_bytes().chunks(3) {
                    send(&events, StreamEvent::ToolCallArgsDelta {
                        id: id.clone(),
                        json_fragment: String::from_utf8(fragment.to_vec()).unwrap(),
                    }).await?;
                }
                send(&events, StreamEvent::ToolCallFinished { id }).await?;
                send(&events, StreamEvent::Finished { stop_reason: StopReason::ToolUse }).await?;
            } else {
                assert_eq!(step, 5);
                assert!(request.messages.last().unwrap().parts.iter().any(|part| {
                    matches!(part, MessagePart::ToolResult { text, .. } if text.contains("new"))
                }));
                send(&events, StreamEvent::TextDelta { text: "Done".into() }).await?;
                send(&events, StreamEvent::Finished { stop_reason: StopReason::EndTurn }).await?;
            }
            Ok(())
        })
    }
}

async fn send(events: &mpsc::Sender<StreamEvent>, event: StreamEvent) -> Result<(), ProviderError> {
    events.send(event).await.map_err(|_| ProviderError::Cancelled)
}

#[tokio::test]
async fn five_step_read_and_edit_fixture_completes() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("note.txt"), "old\n").unwrap();
    let loop_runner = ToolLoop::new(PathGuard::new(root.path()).unwrap());
    let approvals = Arc::new(Mutex::new(Vec::new()));
    let approved = Arc::clone(&approvals);
    let result = loop_runner.run(
        Arc::new(Script::default()),
        ChatRequest {
            model_id: "fixture".into(), system: String::new(), messages: Vec::new(),
            tools: Vec::new(), max_output_tokens: 100, reasoning: None,
        },
        SessionId::new("fixture"),
        CancellationToken::new(),
        move |name, target| {
            approved.lock().unwrap().push((name.to_owned(), target.to_owned()));
            true
        },
    ).await.unwrap();
    assert_eq!(result.exchanges, 6);
    assert_eq!(result.final_text, "Done");
    assert_eq!(fs::read_to_string(root.path().join("note.txt")).unwrap(), "new\n");
    assert_eq!(approvals.lock().unwrap().as_slice(), &[("edit_file".into(), "note.txt".into())]);
    assert_eq!(result.messages.len(), 11);
    assert_eq!(result.messages[1].role, Role::Tool);
}

#[test]
fn invalid_arguments_and_denial_do_not_mutate() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("note.txt"), "old").unwrap();
    let loop_runner = ToolLoop::new(PathGuard::new(root.path()).unwrap());
    let denied = super::dispatch::run(&loop_runner, "write_file", r#"{"path":"note.txt","text":"new"}"#, &|_, _| false);
    assert_eq!(denied.unwrap_err(), "tool approval denied");
    let malformed = super::dispatch::run(&loop_runner, "read_file", r#"{"path":12}"#, &|_, _| true);
    assert!(malformed.unwrap_err().contains("path must be a string"));
    assert_eq!(fs::read_to_string(root.path().join("note.txt")).unwrap(), "old");
}
