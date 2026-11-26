//! Tests for rewinding a session to one of its messages.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tether_providers::{
    BoxFuture, ChatProvider, ChatRequest, ProviderError, StopReason, StreamEvent,
};
use tether_store::{BlobStore, SnapshotInput, Store};
use tether_types::{CoreEvent, MessagePart, Role, SessionId, SessionStatus};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::SessionActor;
use crate::session_tests::{Recorder, next_event, seeded_store};

/// Waits for the turn until the session is cancelled.
struct UntilCancelled;

impl ChatProvider for UntilCancelled {
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

/// Records the text of each request and answers at once.
#[derive(Default)]
struct Recording(Mutex<Vec<Vec<String>>>);

impl ChatProvider for Recording {
    fn stream_chat<'a>(
        &'a self,
        request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            let texts = request
                .messages
                .iter()
                .flat_map(|message| message.parts.iter())
                .filter_map(|part| match part {
                    MessagePart::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect();
            self.0.lock().unwrap().push(texts);
            events
                .send(StreamEvent::TextDelta { text: "ok".into() })
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

/// Seeds a user message, an assistant reply with a checkpoint, and a tool message.
/// Returns the message ids as strings: first, reply, later.
fn fixture() -> (tempfile::TempDir, PathBuf, SessionId, [String; 3]) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    let (mut store, session) = seeded_store(&path);
    let first = store.begin_turn(&session, "first", 2).unwrap();
    let reply = store
        .append_message(
            &session,
            Role::Assistant,
            &[MessagePart::Text {
                text: "reply".into(),
            }],
            3,
        )
        .unwrap();
    let later = store
        .append_message(
            &session,
            Role::Tool,
            &[MessagePart::Text {
                text: "later".into(),
            }],
            4,
        )
        .unwrap();
    let blobs = BlobStore::open(dir.path().join("blobs")).unwrap();
    store
        .save_checkpoint(
            &session,
            Some(&reply.id),
            &blobs,
            &[SnapshotInput {
                path: "note.txt".into(),
                content: None,
            }],
            5,
        )
        .unwrap();
    let ids = [
        first.as_str().to_owned(),
        reply.id.as_str().to_owned(),
        later.id.as_str().to_owned(),
    ];
    (dir, path, session, ids)
}

fn rewound_flags(path: &Path, session: &SessionId) -> Vec<(String, bool)> {
    Store::open(path)
        .unwrap()
        .messages(session)
        .unwrap()
        .into_iter()
        .map(|message| (message.id.as_str().to_owned(), message.rewound))
        .collect()
}

#[tokio::test]
async fn rewind_to_message_stops_a_running_turn_and_hides_the_message_onward() {
    let (_dir, path, session, [first, reply, _later]) = fixture();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (actor, handle) = SessionActor::new(
        Store::open(&path).unwrap(),
        &session,
        Arc::new(UntilCancelled),
        Arc::new(Recorder(tx)),
    )
    .unwrap();
    let task = tokio::spawn(actor.run());
    handle.send_message("running".into()).await.unwrap();
    while !matches!(next_event(&mut rx).await, CoreEvent::SessionUpdated { .. }) {}

    handle.rewind_to_message(reply.clone()).await.unwrap();

    let mut saw_rewound = false;
    while let Ok(event) = rx.try_recv() {
        saw_rewound |= matches!(event, CoreEvent::SessionRewound { .. });
    }
    assert!(saw_rewound);
    drop(handle);
    task.await.unwrap();

    let flags = rewound_flags(&path, &session);
    assert_eq!(flags[0], (first, false));
    assert!(flags.iter().skip(1).all(|(_, hidden)| *hidden));
    assert_eq!(flags[1].0, reply);
    let store = Store::open(&path).unwrap();
    assert_eq!(
        store.load_session(&session).unwrap().unwrap().status,
        SessionStatus::Idle
    );
    assert_eq!(store.list_checkpoints(&session).unwrap().len(), 0);
}

#[tokio::test]
async fn later_turn_request_leaves_out_the_hidden_messages() {
    let (_dir, path, session, [_first, reply, _later]) = fixture();
    let provider = Arc::new(Recording::default());
    let (tx, _rx) = mpsc::unbounded_channel();
    let (actor, handle) = SessionActor::new(
        Store::open(&path).unwrap(),
        &session,
        provider.clone(),
        Arc::new(Recorder(tx)),
    )
    .unwrap();
    let task = tokio::spawn(actor.run());
    handle.rewind_to_message(reply).await.unwrap();
    handle.send_message("new request".into()).await.unwrap();
    drop(handle);
    task.await.unwrap();

    let requests = provider.0.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains(&"first".to_owned()));
    assert!(requests[0].contains(&"new request".to_owned()));
    assert!(
        !requests[0]
            .iter()
            .any(|text| text == "reply" || text == "later")
    );
}

#[tokio::test]
async fn unknown_message_id_errors_and_changes_nothing() {
    let (_dir, path, session, _ids) = fixture();
    let (tx, _rx) = mpsc::unbounded_channel();
    let (actor, handle) = SessionActor::new(
        Store::open(&path).unwrap(),
        &session,
        Arc::new(UntilCancelled),
        Arc::new(Recorder(tx)),
    )
    .unwrap();
    let task = tokio::spawn(actor.run());
    assert!(handle.rewind_to_message("missing".into()).await.is_err());
    drop(handle);
    task.await.unwrap();

    assert!(
        rewound_flags(&path, &session)
            .iter()
            .all(|(_, hidden)| !hidden)
    );
    assert_eq!(
        Store::open(&path)
            .unwrap()
            .list_checkpoints(&session)
            .unwrap()
            .len(),
        1
    );
}
