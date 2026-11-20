#![expect(
    clippy::excessive_nesting,
    reason = "test scripts and event loops keep their expected order visible"
)]

use std::sync::{Arc, Mutex};

use tether_providers::{
    BoxFuture, ChatProvider, ChatRequest, ProviderError, ResponseUsage, StopReason, StreamEvent,
};
use tether_store::Store;
use tether_types::{
    CoreEvent, MessagePart, ModelRef, PermissionMode, ProjectId, Provider, Role, SessionStatus,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{EventSink, SessionActor};

struct ScriptedProvider {
    requests: Arc<Mutex<Vec<ChatRequest>>>,
}

impl ChatProvider for ScriptedProvider {
    fn stream_chat<'a>(
        &'a self,
        request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            self.requests.lock().unwrap().push(request.clone());
            for text in ["Hel", "lo"] {
                events
                    .send(StreamEvent::TextDelta { text: text.into() })
                    .await
                    .unwrap();
            }
            events
                .send(StreamEvent::Usage(ResponseUsage {
                    input_tokens: 5,
                    output_tokens: 2,
                    cache_read_tokens: 3,
                    cache_write_tokens: 0,
                }))
                .await
                .unwrap();
            events
                .send(StreamEvent::Finished {
                    stop_reason: StopReason::EndTurn,
                })
                .await
                .unwrap();
            Ok(())
        })
    }
}

struct WaitingProvider;

impl ChatProvider for WaitingProvider {
    fn stream_chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
        _events: mpsc::Sender<StreamEvent>,
        cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            cancel.cancelled().await;
            Err(ProviderError::Cancelled)
        })
    }
}

struct DelayedProvider(Arc<tokio::sync::Notify>);

impl ChatProvider for DelayedProvider {
    fn stream_chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            events
                .send(StreamEvent::TextDelta {
                    text: "first".into(),
                })
                .await
                .unwrap();
            self.0.notified().await;
            events
                .send(StreamEvent::TextDelta {
                    text: " second".into(),
                })
                .await
                .unwrap();
            events
                .send(StreamEvent::Finished {
                    stop_reason: StopReason::EndTurn,
                })
                .await
                .unwrap();
            Ok(())
        })
    }
}

pub(crate) struct Recorder(pub(crate) mpsc::UnboundedSender<CoreEvent>);

impl EventSink for Recorder {
    fn on_event(&self, event: CoreEvent) {
        self.0.send(event).unwrap();
    }
}

pub(crate) fn seeded_store(path: &std::path::Path) -> (Store, tether_types::SessionId) {
    let mut store = Store::open(path).unwrap();
    let project = ProjectId::new("01TESTPROJECT00000000000000");
    rusqlite::Connection::open(path)
        .unwrap()
        .execute(
            "INSERT INTO projects (id, name, path, created_at) VALUES (?1, 'Test', 'test', 1)",
            [project.as_str()],
        )
        .unwrap();
    let session = store
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "test-model".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    (store, session.id)
}

pub(crate) async fn next_event(rx: &mut mpsc::UnboundedReceiver<CoreEvent>) -> CoreEvent {
    tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .unwrap()
        .unwrap()
}

#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end turn assertion covers streaming and reopening"
)]
#[tokio::test]
async fn a_plain_turn_streams_and_reopens_with_its_transcript() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    let (store, id) = seeded_store(&path);
    let requests = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(ScriptedProvider {
        requests: Arc::clone(&requests),
    });
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (actor, handle) = SessionActor::new(store, &id, provider, Arc::new(Recorder(tx))).unwrap();
    let task = tokio::spawn(actor.run());
    handle.send_message("Hi".into()).await.unwrap();

    let mut events = Vec::new();
    loop {
        let event = next_event(&mut rx).await;
        let finished = matches!(event, CoreEvent::TurnFinished { .. });
        events.push(event);
        if finished {
            break;
        }
    }
    assert!(
        events
            .iter()
            .any(|event| matches!(event, CoreEvent::TextDelta { text, .. } if text == "Hello"))
    );
    assert!(events.iter().any(|event| matches!(
        event,
        CoreEvent::SessionUpdated {
            status: SessionStatus::Streaming,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(event, CoreEvent::UsageUpdated { usage, .. } if usage.tokens_in == 8 && usage.tokens_out == 2)));
    assert_eq!(requests.lock().unwrap()[0].tools.len(), 0);
    assert_eq!(
        requests.lock().unwrap()[0].messages[0].parts,
        vec![MessagePart::Text { text: "Hi".into() }]
    );

    drop(handle);
    task.await.unwrap();
    let reopened = Store::open(&path).unwrap();
    let messages = reopened.messages(&id).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, Role::User);
    assert_eq!(
        messages[1].parts,
        vec![MessagePart::Text {
            text: "Hello".into()
        }]
    );
    assert_eq!(
        reopened.load_session(&id).unwrap().unwrap().status,
        SessionStatus::Idle
    );
}

#[tokio::test]
async fn interrupt_stops_the_provider_and_keeps_the_user_message() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    let (store, id) = seeded_store(&path);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (actor, handle) = SessionActor::new(
        store,
        &id,
        Arc::new(WaitingProvider),
        Arc::new(Recorder(tx)),
    )
    .unwrap();
    let task = tokio::spawn(actor.run());
    handle.send_message("Hi".into()).await.unwrap();
    assert!(matches!(
        next_event(&mut rx).await,
        CoreEvent::SessionUpdated {
            status: SessionStatus::Requesting,
            ..
        }
    ));
    handle.interrupt();
    let mut interrupted = false;
    for _ in 0..3 {
        if matches!(next_event(&mut rx).await, CoreEvent::TurnInterrupted { .. }) {
            interrupted = true;
            break;
        }
    }
    assert!(interrupted);
    drop(handle);
    task.await.unwrap();
    let reopened = Store::open(&path).unwrap();
    assert_eq!(reopened.messages(&id).unwrap().len(), 1);
    assert_eq!(
        reopened.load_session(&id).unwrap().unwrap().status,
        SessionStatus::Idle
    );
}

#[tokio::test]
async fn text_reaches_the_sink_before_the_provider_finishes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    let (store, id) = seeded_store(&path);
    let gate = Arc::new(tokio::sync::Notify::new());
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (actor, handle) = SessionActor::new(
        store,
        &id,
        Arc::new(DelayedProvider(Arc::clone(&gate))),
        Arc::new(Recorder(tx)),
    )
    .unwrap();
    let task = tokio::spawn(actor.run());
    handle.send_message("Hi".into()).await.unwrap();

    loop {
        if let CoreEvent::TextDelta { text, .. } = next_event(&mut rx).await {
            assert_eq!(text, "first");
            break;
        }
    }
    assert!(
        rx.try_recv().is_err(),
        "turn finished before the second chunk was sent"
    );
    gate.notify_one();
    loop {
        if matches!(next_event(&mut rx).await, CoreEvent::TurnFinished { .. }) {
            break;
        }
    }
    drop(handle);
    task.await.unwrap();
    assert_eq!(
        Store::open(&path).unwrap().messages(&id).unwrap()[1].parts,
        vec![MessagePart::Text {
            text: "first second".into()
        }]
    );
}

#[tokio::test]
async fn later_turn_omits_rewound_messages_from_provider_context() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    let (mut store, id) = seeded_store(&path);
    store.begin_turn(&id, "old request", 2).unwrap();
    let old_assistant = store
        .append_message(
            &id,
            Role::Assistant,
            &[MessagePart::Text {
                text: "old reply".into(),
            }],
            3,
        )
        .unwrap();
    store.rewind_conversation(&id, &old_assistant.id).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(ScriptedProvider {
        requests: Arc::clone(&requests),
    });
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (actor, handle) = SessionActor::new(store, &id, provider, Arc::new(Recorder(tx))).unwrap();
    let task = tokio::spawn(actor.run());
    handle.send_message("new request".into()).await.unwrap();
    loop {
        if matches!(next_event(&mut rx).await, CoreEvent::TurnFinished { .. }) {
            break;
        }
    }
    {
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].messages.len(), 2);
        assert_eq!(
            requests[0].messages[0].parts,
            vec![MessagePart::Text {
                text: "old request".into()
            }]
        );
        assert_eq!(
            requests[0].messages[1].parts,
            vec![MessagePart::Text {
                text: "new request".into()
            }]
        );
    }
    drop(handle);
    task.await.unwrap();
}
