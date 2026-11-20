//! Regression fixtures for malformed calls and usage after failures.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tether_providers::{
    BoxFuture, ChatProvider, ChatRequest, ProviderError, ResponseUsage, StopReason, StreamEvent,
};
use tether_store::{BlobStore, Store};
use tether_types::{CoreEvent, MessagePart, SessionId, ToolCallId};
use tether_workspace::PathGuard;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{ToolLoop, ToolLoopContext};

async fn send(events: &mpsc::Sender<StreamEvent>, event: StreamEvent) -> Result<(), ProviderError> {
    events
        .send(event)
        .await
        .map_err(|_| ProviderError::Cancelled)
}

struct Malformed;

impl ChatProvider for Malformed {
    fn stream_chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            let id = ToolCallId::new("bad-call");
            send(
                &events,
                StreamEvent::ToolCallStarted {
                    id: id.clone(),
                    name: "read_file".into(),
                },
            )
            .await?;
            send(
                &events,
                StreamEvent::ToolCallArgsDelta {
                    id: id.clone(),
                    json_fragment: "{".into(),
                },
            )
            .await?;
            send(&events, StreamEvent::ToolCallFinished { id }).await?;
            send(
                &events,
                StreamEvent::Finished {
                    stop_reason: StopReason::ToolUse,
                },
            )
            .await
        })
    }
}

#[tokio::test]
async fn malformed_tool_json_never_enters_saved_history() {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::open_in_memory().unwrap();
    let session_id = SessionId::new("missing");
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let mut context = ToolLoopContext {
        session_id: &session_id,
        store: &mut store,
        blobs: &blobs,
        cancel: CancellationToken::new(),
        approve: &|_, _| false,
        sink: None,
    };
    let result = ToolLoop::new(PathGuard::new(root.path()).unwrap())
        .run(
            Arc::new(Malformed),
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
        .await;
    assert!(result.is_err());
    assert_eq!(store.messages(&session_id).unwrap().len(), 0);
}

struct UsageThenFail(AtomicUsize);

impl ChatProvider for UsageThenFail {
    #[expect(
        clippy::excessive_nesting,
        reason = "the fixture sends a paid response before a provider failure"
    )]
    fn stream_chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            if self.0.fetch_add(1, Ordering::SeqCst) > 0 {
                return Err(ProviderError::Network {
                    message: "offline".into(),
                });
            }
            send(
                &events,
                StreamEvent::Usage(ResponseUsage {
                    input_tokens: 7,
                    output_tokens: 3,
                    ..ResponseUsage::default()
                }),
            )
            .await?;
            let id = ToolCallId::new("list-1");
            send(
                &events,
                StreamEvent::ToolCallStarted {
                    id: id.clone(),
                    name: "list_dir".into(),
                },
            )
            .await?;
            send(
                &events,
                StreamEvent::ToolCallArgsDelta {
                    id: id.clone(),
                    json_fragment: "{}".into(),
                },
            )
            .await?;
            send(&events, StreamEvent::ToolCallFinished { id }).await?;
            send(
                &events,
                StreamEvent::Finished {
                    stop_reason: StopReason::ToolUse,
                },
            )
            .await
        })
    }
}

#[tokio::test]
async fn completed_response_usage_survives_a_later_provider_failure() {
    let root = tempfile::tempdir().unwrap();
    let database = root.path().join("store.db");
    drop(Store::open(&database).unwrap());
    rusqlite::Connection::open(&database).unwrap().execute_batch(
        "INSERT INTO projects (id, name, path, created_at) VALUES ('project', 'fixture', 'fixture', 1);
         INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
         VALUES ('fixture', 'project', 'openai', 'fixture', 'ask', 'idle', 1, 1);",
    ).unwrap();
    let mut store = Store::open(&database).unwrap();
    let session_id = SessionId::new("fixture");
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let mut context = ToolLoopContext {
        session_id: &session_id,
        store: &mut store,
        blobs: &blobs,
        cancel: CancellationToken::new(),
        approve: &|_, _| false,
        sink: None,
    };
    let result = ToolLoop::new(PathGuard::new(root.path()).unwrap())
        .run(
            Arc::new(UsageThenFail(AtomicUsize::new(0))),
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
        .await;
    assert!(result.is_err());
    assert_eq!(store.session_usage(&session_id).unwrap().tokens_in, 7);
    assert_eq!(store.session_usage(&session_id).unwrap().tokens_out, 3);
}

struct ThreeReads;

async fn send_read_call(
    events: &mpsc::Sender<StreamEvent>,
    number: u8,
) -> Result<(), ProviderError> {
    let id = ToolCallId::new(format!("read-{number}"));
    send(
        events,
        StreamEvent::ToolCallStarted {
            id: id.clone(),
            name: "read_file".into(),
        },
    )
    .await?;
    send(
        events,
        StreamEvent::ToolCallArgsDelta {
            id: id.clone(),
            json_fragment: r#"{"path":"note.txt"}"#.into(),
        },
    )
    .await?;
    send(events, StreamEvent::ToolCallFinished { id }).await
}

impl ChatProvider for ThreeReads {
    fn stream_chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            send_read_call(&events, 1).await?;
            send_read_call(&events, 2).await?;
            send_read_call(&events, 3).await?;
            send(
                &events,
                StreamEvent::Finished {
                    stop_reason: StopReason::ToolUse,
                },
            )
            .await
        })
    }
}

struct CancelOnSecond {
    token: CancellationToken,
    proposals: AtomicUsize,
}

impl crate::EventSink for CancelOnSecond {
    fn on_event(&self, event: CoreEvent) {
        if matches!(event, CoreEvent::ToolProposed { .. })
            && self.proposals.fetch_add(1, Ordering::SeqCst) == 1
        {
            self.token.cancel();
        }
    }
}

#[tokio::test]
async fn cancellation_saves_results_for_every_pending_tool_call() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("note.txt"), "hello").unwrap();
    let database = root.path().join("store.db");
    drop(Store::open(&database).unwrap());
    rusqlite::Connection::open(&database).unwrap().execute_batch(
        "INSERT INTO projects (id, name, path, created_at) VALUES ('project', 'fixture', 'fixture', 1);
         INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
         VALUES ('fixture', 'project', 'openai', 'fixture', 'ask', 'idle', 1, 1);",
    ).unwrap();
    let mut store = Store::open(&database).unwrap();
    let session_id = SessionId::new("fixture");
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let cancel = CancellationToken::new();
    let sink = CancelOnSecond {
        token: cancel.clone(),
        proposals: AtomicUsize::new(0),
    };
    let mut context = ToolLoopContext {
        session_id: &session_id,
        store: &mut store,
        blobs: &blobs,
        cancel,
        approve: &|_, _| false,
        sink: Some(&sink),
    };
    let result = ToolLoop::new(PathGuard::new(root.path()).unwrap())
        .run(
            Arc::new(ThreeReads),
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
        .await;
    assert!(result.is_err());
    let messages = store.messages(&session_id).unwrap();
    assert_eq!(messages.len(), 4);
    for (number, message) in messages.iter().skip(1).enumerate() {
        assert!(matches!(
            &message.parts[0],
            MessagePart::ToolResult { tool_call_id, is_error, .. }
                if tool_call_id.as_str() == format!("read-{}", number + 1)
                    && *is_error == (number > 0)
        ));
    }
}
