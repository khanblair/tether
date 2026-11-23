//! Collects provider events, batches text, and saves each batch before it is emitted.

use std::sync::Arc;
use std::time::Duration;

use tether_providers::{ChatProvider, ChatRequest, ResponseUsage, StopReason, StreamEvent};
use tether_types::{CoreError, CoreEvent, MessageId, SessionStatus, UsageSnapshot};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};
use tokio_util::sync::CancellationToken;

use super::{SessionActor, TEXT_BATCH_CHARS, TEXT_BATCH_MS, now_ms, require_finished, store_error};

impl SessionActor {
    pub(super) async fn stream(
        &mut self,
        provider: Arc<dyn ChatProvider>,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> (UsageSnapshot, bool, Result<(), CoreError>) {
        let model_id = request.model_id.clone();
        let mut output = Output::default();
        let result = self
            .stream_attempts(&provider, &request, &cancel, &mut output)
            .await;
        self.finish_usage(model_id.as_str(), output.usage, result)
    }

    /// Sends the request until it finishes, waiting out network drops on the way.
    async fn stream_attempts(
        &mut self,
        provider: &Arc<dyn ChatProvider>,
        request: &ChatRequest,
        cancel: &CancellationToken,
        output: &mut Output,
    ) -> Result<(), CoreError> {
        self.wait_if_offline(cancel).await?;
        while self.attempt(provider, request, cancel, output).await? {
            self.wait_if_offline(cancel).await?;
        }
        Ok(())
    }

    /// Runs one provider request. Returns `Ok(true)` when a network drop was waited out
    /// and the request must be sent again.
    async fn attempt(
        &mut self,
        provider: &Arc<dyn ChatProvider>,
        request: &ChatRequest,
        cancel: &CancellationToken,
        output: &mut Output,
    ) -> Result<bool, CoreError> {
        let (tx, mut rx) = mpsc::channel(32);
        let attempt = request.clone();
        let attempt_cancel = cancel.clone();
        let attempt_provider = Arc::clone(provider);
        let provider_task = tokio::spawn(async move {
            attempt_provider
                .stream_chat(&attempt, tx, attempt_cancel)
                .await
        });
        if let Err(error) = self.collect(&mut rx, output).await {
            provider_task.abort();
            return Err(error);
        }
        let result = provider_task
            .await
            .map_err(|_| CoreError::Internal {
                message: "provider task stopped".into(),
            })
            .and_then(|provider_result| provider_result.map_err(CoreError::from))
            .and_then(|()| require_finished(output.finished));
        let Err(error) = result else {
            return Ok(false);
        };
        if !self.wait_after_drop(&error, cancel).await? {
            return Err(error);
        }
        output.finished = false;
        Ok(true)
    }

    /// Waits for the network when the gate is on and the device is offline.
    async fn wait_if_offline(&mut self, cancel: &CancellationToken) -> Result<(), CoreError> {
        let Some(network) = self.network.as_ref() else {
            return Ok(());
        };
        if !network.enabled() || network.is_online() {
            return Ok(());
        }
        // The status write is best effort: the wait and the events still run without it.
        drop(
            self.store
                .set_session_status(&self.session.id, SessionStatus::Paused, now_ms()),
        );
        self.sink.on_event(CoreEvent::NetworkPaused {
            session_id: self.session.id.clone(),
        });
        Self::wait_for_network(network, cancel).await?;
        // The status write is best effort, as above.
        drop(
            self.store
                .set_session_status(&self.session.id, SessionStatus::Requesting, now_ms()),
        );
        self.sink.on_event(CoreEvent::NetworkResumed {
            session_id: self.session.id.clone(),
        });
        Ok(())
    }

    /// Waits for a network drop to end when the error is a network error and the gate is on.
    /// Returns `Ok(false)` when there is nothing to wait for.
    async fn wait_after_drop(
        &mut self,
        error: &CoreError,
        cancel: &CancellationToken,
    ) -> Result<bool, CoreError> {
        let Some(network) = self.network.as_ref() else {
            return Ok(false);
        };
        if !matches!(error, CoreError::Network { .. }) || !network.enabled() {
            return Ok(false);
        }
        network.mark_offline();
        drop(
            self.store
                .set_session_status(&self.session.id, SessionStatus::Paused, now_ms()),
        );
        self.sink.on_event(CoreEvent::NetworkPaused {
            session_id: self.session.id.clone(),
        });
        Self::wait_for_network(network, cancel).await?;
        drop(
            self.store
                .set_session_status(&self.session.id, SessionStatus::Requesting, now_ms()),
        );
        self.sink.on_event(CoreEvent::NetworkResumed {
            session_id: self.session.id.clone(),
        });
        Ok(true)
    }

    async fn wait_for_network(
        network: &crate::network::NetworkControl,
        cancel: &CancellationToken,
    ) -> Result<(), CoreError> {
        match network.wait_until_online(cancel).await {
            crate::network::WaitResult::Online => Ok(()),
            crate::network::WaitResult::Cancelled => Err(CoreError::Cancelled {
                message: "turn interrupted".into(),
            }),
            crate::network::WaitResult::TimedOut => Err(CoreError::Network {
                message: "the network never came back".into(),
            }),
        }
    }

    fn finish_usage(
        &self,
        model_id: &str,
        usage: ResponseUsage,
        result: Result<(), CoreError>,
    ) -> (UsageSnapshot, bool, Result<(), CoreError>) {
        match crate::usage::price_response(&self.store, &self.session.id, model_id, usage) {
            Ok(priced) => (priced.snapshot, priced.price_known, result),
            Err(error) => (UsageSnapshot::default(), false, Err(store_error(error))),
        }
    }

    #[expect(
        clippy::excessive_nesting,
        reason = "the stream loop selects between events and the text batch deadline"
    )]
    async fn collect(
        &mut self,
        rx: &mut mpsc::Receiver<StreamEvent>,
        output: &mut Output,
    ) -> Result<(), CoreError> {
        loop {
            let event = if let Some(deadline) = output.deadline {
                tokio::select! {
                    item = rx.recv() => item,
                    () = sleep_until(deadline) => { self.flush(output)?; continue; }
                }
            } else {
                rx.recv().await
            };
            let Some(event) = event else { break };
            if let Err(error) = Self::accept_event(event, output) {
                self.flush(output)?;
                return Err(error);
            }
            if output.pending_chars >= TEXT_BATCH_CHARS {
                self.flush(output)?;
            }
        }
        self.flush(output)
    }

    #[expect(
        clippy::excessive_nesting,
        reason = "each provider event variant updates one output state"
    )]
    fn accept_event(event: StreamEvent, output: &mut Output) -> Result<(), CoreError> {
        match event {
            StreamEvent::TextDelta { text } => {
                output.text.push_str(&text);
                output.pending_text.push_str(&text);
                output.pending_chars += text.chars().count();
                output
                    .deadline
                    .get_or_insert_with(|| Instant::now() + Duration::from_millis(TEXT_BATCH_MS));
            }
            StreamEvent::ThinkingDelta { text } => {
                output.thinking.push_str(&text);
                output.pending_thinking.push_str(&text);
                output.pending_chars += text.chars().count();
                output
                    .deadline
                    .get_or_insert_with(|| Instant::now() + Duration::from_millis(TEXT_BATCH_MS));
            }
            StreamEvent::Usage(usage) => output.usage = usage,
            StreamEvent::Finished { stop_reason } => {
                if stop_reason == StopReason::ToolUse {
                    return Err(CoreError::InvalidInput {
                        message: "this plain chat turn cannot run tools".into(),
                    });
                }
                output.finished = true;
            }
            StreamEvent::ToolCallStarted { .. }
            | StreamEvent::ToolCallArgsDelta { .. }
            | StreamEvent::ToolCallFinished { .. } => {
                return Err(CoreError::InvalidInput {
                    message: "this plain chat turn cannot run tools".into(),
                });
            }
        }
        Ok(())
    }

    fn flush(&mut self, output: &mut Output) -> Result<(), CoreError> {
        if output.pending_text.is_empty() && output.pending_thinking.is_empty() {
            output.deadline = None;
            return Ok(());
        }
        let id = &self.session.id;
        let message_id = if let Some(id) = &output.message_id {
            id.clone()
        } else {
            let created = self
                .store
                .create_assistant_message(id, now_ms())
                .map_err(store_error)?;
            output.message_id = Some(created.clone());
            self.store
                .set_session_status(id, SessionStatus::Streaming, now_ms())
                .map_err(store_error)?;
            self.emit_status(SessionStatus::Streaming);
            created
        };
        self.store
            .save_assistant_parts(&message_id, &output.text, &output.thinking)
            .map_err(store_error)?;
        if !output.pending_thinking.is_empty() {
            self.sink.on_event(CoreEvent::ThinkingDelta {
                session_id: id.clone(),
                message_id: message_id.clone(),
                text: std::mem::take(&mut output.pending_thinking),
            });
        }
        if !output.pending_text.is_empty() {
            self.sink.on_event(CoreEvent::TextDelta {
                session_id: id.clone(),
                message_id,
                text: std::mem::take(&mut output.pending_text),
            });
        }
        output.pending_chars = 0;
        output.deadline = None;
        Ok(())
    }
}

#[derive(Default)]
struct Output {
    text: String,
    thinking: String,
    pending_text: String,
    pending_thinking: String,
    pending_chars: usize,
    deadline: Option<Instant>,
    message_id: Option<MessageId>,
    usage: ResponseUsage,
    finished: bool,
}
