//! Model provider adapters and the provider trait.
//!
//! An adapter turns a [`ChatRequest`] into one provider's API call and streams the answer
//! back as [`StreamEvent`] values. The engine owns retries, batching, and permissions.

pub mod anthropic;
mod client;
mod error;
mod event;
mod request;
pub mod sse;

use std::future::Future;
use std::pin::Pin;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub use anthropic::{AnthropicConfig, AnthropicProvider};
pub use client::{client_builder, http_client};
pub use error::ProviderError;
pub use event::{ResponseUsage, StopReason, StreamEvent};
pub use request::{ChatRequest, ReasoningEffort, ToolSpec};

/// A boxed future, so [`ChatProvider`] can be used as a trait object.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// One model provider.
///
/// The contract for [`stream_chat`](Self::stream_chat):
/// - Events go to `events` in the order the model produced them. On success the last event
///   is `StreamEvent::Finished`.
/// - On failure it returns the error. Partial events may already have been sent, and no
///   `Finished` follows.
/// - When `cancel` fires, or the receiver is dropped, it stops at once and returns
///   `ProviderError::Cancelled`.
/// - It does not retry. The engine retries when `CoreError::retryable` says so.
pub trait ChatProvider: Send + Sync {
    /// Streams one response for `request` into `events`.
    fn stream_chat<'a>(
        &'a self,
        request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed;

    async fn send_finished(events: mpsc::Sender<StreamEvent>) -> Result<(), ProviderError> {
        let finished = StreamEvent::Finished {
            stop_reason: StopReason::EndTurn,
        };
        events
            .send(finished)
            .await
            .map_err(|_| ProviderError::Cancelled)
    }

    impl ChatProvider for Fixed {
        fn stream_chat<'a>(
            &'a self,
            _request: &'a ChatRequest,
            events: mpsc::Sender<StreamEvent>,
            _cancel: CancellationToken,
        ) -> BoxFuture<'a, Result<(), ProviderError>> {
            Box::pin(send_finished(events))
        }
    }

    fn request() -> ChatRequest {
        ChatRequest {
            model_id: "m".into(),
            system: String::new(),
            messages: Vec::new(),
            tools: Vec::new(),
            max_output_tokens: 16,
            reasoning: None,
        }
    }

    #[tokio::test]
    async fn a_provider_works_as_a_trait_object() {
        let provider: Box<dyn ChatProvider> = Box::new(Fixed);
        let (tx, mut rx) = mpsc::channel(4);
        provider
            .stream_chat(&request(), tx, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(
            rx.recv().await,
            Some(StreamEvent::Finished {
                stop_reason: StopReason::EndTurn
            })
        );
    }

    #[tokio::test]
    async fn a_dropped_receiver_stops_the_stream() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let result = Fixed
            .stream_chat(&request(), tx, CancellationToken::new())
            .await;
        assert_eq!(result, Err(ProviderError::Cancelled));
    }
}
