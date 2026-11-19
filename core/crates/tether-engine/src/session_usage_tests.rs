#![expect(
    clippy::excessive_nesting,
    reason = "test scripts and event loops keep their expected order visible"
)]

use std::sync::{Arc, Mutex};

use tether_providers::{
    BoxFuture, ChatProvider, ChatRequest, ProviderError, ResponseUsage, StopReason, StreamEvent,
};
use tether_store::Store;
use tether_types::{CoreEvent, MessagePart};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::SessionActor;
use crate::session_tests::{Recorder, next_event, seeded_store};

struct UsageThenErrorProvider {
    reported: Arc<tokio::sync::Notify>,
    wait_for_cancel: bool,
}

struct NamedProvider(&'static str);

impl ChatProvider for NamedProvider {
    fn stream_chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            events
                .send(StreamEvent::TextDelta {
                    text: self.0.into(),
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

impl ChatProvider for UsageThenErrorProvider {
    fn stream_chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            events
                .send(StreamEvent::Usage(ResponseUsage {
                    input_tokens: 7,
                    output_tokens: 2,
                    cache_read_tokens: 3,
                    cache_write_tokens: 0,
                }))
                .await
                .unwrap();
            self.reported.notify_one();
            if self.wait_for_cancel {
                cancel.cancelled().await;
                Err(ProviderError::Cancelled)
            } else {
                Err(ProviderError::Network {
                    message: "disconnected".into(),
                })
            }
        })
    }
}

#[tokio::test]
async fn reported_usage_survives_failure_and_cancellation() {
    for wait_for_cancel in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.db");
        let (store, id) = seeded_store(&path);
        let reported = Arc::new(tokio::sync::Notify::new());
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (actor, handle) = SessionActor::new(
            store,
            &id,
            Arc::new(UsageThenErrorProvider {
                reported: Arc::clone(&reported),
                wait_for_cancel,
            }),
            Arc::new(Recorder(tx)),
        )
        .unwrap();
        let task = tokio::spawn(actor.run());
        handle.send_message("Hi".into()).await.unwrap();
        reported.notified().await;
        if wait_for_cancel {
            handle.interrupt();
        }
        loop {
            let event = next_event(&mut rx).await;
            if matches!(
                event,
                CoreEvent::TurnFailed { .. } | CoreEvent::TurnInterrupted { .. }
            ) {
                break;
            }
        }
        drop(handle);
        task.await.unwrap();
        let usage = Store::open(&path).unwrap().session_usage(&id).unwrap();
        assert_eq!(usage.tokens_in, 10);
        assert_eq!(usage.tokens_out, 2);
    }
}

#[tokio::test]
async fn a_new_turn_uses_the_current_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    let (store, id) = seeded_store(&path);
    let name = Arc::new(Mutex::new("old"));
    let current = Arc::clone(&name);
    let factory = Arc::new(move || {
        let value = *current.lock().unwrap();
        Ok(Arc::new(NamedProvider(value)) as Arc<dyn ChatProvider>)
    });
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (actor, handle) =
        SessionActor::new_with_provider_factory(store, &id, factory, Arc::new(Recorder(tx)))
            .unwrap();
    let task = tokio::spawn(actor.run());
    handle.send_message("First".into()).await.unwrap();
    while !matches!(next_event(&mut rx).await, CoreEvent::TurnFinished { .. }) {}
    *name.lock().unwrap() = "new";
    handle.send_message("Second".into()).await.unwrap();
    while !matches!(next_event(&mut rx).await, CoreEvent::TurnFinished { .. }) {}
    drop(handle);
    task.await.unwrap();
    let messages = Store::open(&path).unwrap().messages(&id).unwrap();
    assert_eq!(
        messages[1].parts,
        vec![MessagePart::Text { text: "old".into() }]
    );
    assert_eq!(
        messages[3].parts,
        vec![MessagePart::Text { text: "new".into() }]
    );
}
