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

struct CountingSummaryProvider {
    calls: Arc<std::sync::atomic::AtomicUsize>,
}

impl ChatProvider for CountingSummaryProvider {
    fn stream_chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            events
                .send(StreamEvent::TextDelta { text: "ok".into() })
                .await
                .unwrap();
            events
                .send(StreamEvent::Usage(ResponseUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_read_tokens: 0,
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

/// Runs five finished turns so the history is long enough to compact.
async fn five_turns(
    path: &std::path::Path,
    calls: &Arc<std::sync::atomic::AtomicUsize>,
) -> (
    crate::SessionHandle,
    mpsc::UnboundedReceiver<CoreEvent>,
    tether_types::SessionId,
    tokio::task::JoinHandle<()>,
) {
    let (store, id) = seeded_store(path);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (actor, handle) = SessionActor::new(
        store,
        &id,
        Arc::new(CountingSummaryProvider {
            calls: Arc::clone(calls),
        }),
        Arc::new(Recorder(tx)),
    )
    .unwrap();
    let task = tokio::spawn(actor.run());
    for turn in 0..5 {
        handle.send_message(format!("turn {turn}")).await.unwrap();
        while !matches!(next_event(&mut rx).await, CoreEvent::TurnFinished { .. }) {}
    }
    (handle, rx, id, task)
}

#[tokio::test]
async fn a_compaction_summary_counts_toward_session_usage() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (handle, mut rx, id, task) = five_turns(&path, &calls).await;
    let before = Store::open(&path).unwrap().session_usage(&id).unwrap();
    assert_eq!((before.tokens_in, before.tokens_out), (500, 250));

    handle.compact(None).await.unwrap();
    let mut updated = None;
    while let Ok(Some(event)) =
        tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await
    {
        if let CoreEvent::UsageUpdated { usage, .. } = event {
            updated = Some(usage);
        }
    }
    drop(handle);
    task.await.unwrap();
    let after = Store::open(&path).unwrap().session_usage(&id).unwrap();
    assert_eq!((after.tokens_in, after.tokens_out), (600, 300));
    assert_eq!(updated.map(|usage| usage.tokens_in), Some(600));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 6);
}

/// Sets a daily limit and records more spend than it allows.
fn exhaust_daily_budget(path: &std::path::Path, id: &tether_types::SessionId) {
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let mut store = Store::open(path).unwrap();
    store
        .set_budgets(
            tether_store::BudgetSettings {
                daily_limit_micros: Some(10_000),
                session_limit_micros: None,
            },
            now,
        )
        .unwrap();
    store
        .record_response_usage(&tether_store::ResponseUsageRecord {
            session_id: id.clone(),
            provider: tether_types::Provider::Anthropic,
            model_id: "test-model".into(),
            project_id: "01TESTPROJECT00000000000000".into(),
            day: crate::usage::utc_day_from_ms(now),
            usage: tether_types::UsageSnapshot {
                tokens_in: 0,
                tokens_out: 0,
                cost_micros: 20_000,
            },
            price_known: true,
            now_ms: now,
        })
        .unwrap();
}

#[tokio::test]
async fn compaction_waits_at_an_exhausted_budget_before_calling_the_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (handle, mut rx, id, task) = five_turns(&path, &calls).await;
    exhaust_daily_budget(&path, &id);
    let compacting = {
        let handle = handle.clone();
        tokio::spawn(async move { handle.compact(None).await })
    };
    loop {
        if matches!(next_event(&mut rx).await, CoreEvent::BudgetReached { .. }) {
            break;
        }
    }
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 5);
    handle.continue_past_budget();
    compacting.await.unwrap().unwrap();
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 6);
    drop(handle);
    task.await.unwrap();
}
