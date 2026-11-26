//! Tests that the saved reasoning effort reaches the provider only for reasoning models.

use std::sync::{Arc, Mutex};

use tether_providers::{
    BoxFuture, ChatProvider, ChatRequest, ProviderError, StopReason, StreamEvent,
};
use tether_types::{
    CoreEvent, ModelCaps, ModelInfo, Provider, ReasoningEffort, SessionId, Support,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::SessionActor;
use crate::session_tests::{Recorder, next_event, seeded_store};

/// Records each request and ends the turn at once.
#[derive(Default)]
struct Recording {
    requests: Mutex<Vec<ChatRequest>>,
}

impl ChatProvider for Recording {
    fn stream_chat<'a>(
        &'a self,
        request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            self.requests.lock().unwrap().push(request.clone());
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

fn model_with_reasoning(reasoning: Support) -> ModelInfo {
    ModelInfo {
        provider: Provider::Anthropic,
        id: "test-model".into(),
        display_name: "Test model".into(),
        context_window: None,
        max_output_tokens: None,
        price: None,
        caps: ModelCaps {
            tools: Support::Yes,
            images: Support::No,
            reasoning,
        },
    }
}

/// Saves `effort` on the session, seeds the catalog with `catalog` (when given), runs one
/// turn, and returns the reasoning the provider received.
async fn reasoning_sent(
    effort: Option<ReasoningEffort>,
    catalog: Option<ModelInfo>,
) -> Option<ReasoningEffort> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.db");
    let (mut store, id): (_, SessionId) = seeded_store(&path);
    store.set_session_reasoning(&id, effort, 2).unwrap();
    if let Some(model) = catalog {
        store
            .replace_models(Provider::Anthropic, &[model], 1)
            .unwrap();
    }
    let provider = Arc::new(Recording::default());
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (actor, handle) =
        SessionActor::new(store, &id, provider.clone(), Arc::new(Recorder(tx))).unwrap();
    let task = tokio::spawn(actor.run());
    handle.send_message("Hi".into()).await.unwrap();
    loop {
        let event = next_event(&mut rx).await;
        if matches!(
            event,
            CoreEvent::TurnFinished { .. } | CoreEvent::TurnFailed { .. }
        ) {
            break;
        }
    }
    drop(handle);
    task.await.unwrap();
    let requests = provider.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    requests[0].reasoning
}

#[tokio::test]
async fn a_saved_effort_is_sent_to_a_model_that_reasons() {
    let model = model_with_reasoning(Support::Yes);
    assert_eq!(
        reasoning_sent(Some(ReasoningEffort::High), Some(model)).await,
        Some(ReasoningEffort::High)
    );
}

#[tokio::test]
async fn a_model_that_does_not_reason_gets_no_effort() {
    let model = model_with_reasoning(Support::No);
    assert_eq!(
        reasoning_sent(Some(ReasoningEffort::Medium), Some(model)).await,
        None
    );
}

#[tokio::test]
async fn a_model_with_unknown_reasoning_support_gets_no_effort() {
    let model = model_with_reasoning(Support::Unknown);
    assert_eq!(
        reasoning_sent(Some(ReasoningEffort::Low), Some(model)).await,
        None
    );
}

#[tokio::test]
async fn a_model_missing_from_the_catalog_gets_no_effort() {
    assert_eq!(
        reasoning_sent(Some(ReasoningEffort::High), None).await,
        None
    );
}

#[tokio::test]
async fn an_unset_effort_sends_nothing_even_to_a_reasoning_model() {
    let model = model_with_reasoning(Support::Yes);
    assert_eq!(reasoning_sent(None, Some(model)).await, None);
}
