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
        loop {
            if let Some(network) = self.network.as_ref()
                && network.enabled()
                && !network.is_online()
            {
                let _ = self.store.set_session_status(
                    &self.session.id,
                    SessionStatus::Paused,
                    now_ms(),
                );
                self.sink.on_event(CoreEvent::NetworkPaused {
                    session_id: self.session.id.clone(),
                });
                match self.wait_for_network(network, &cancel).await {
                    Ok(()) => {
                        let _ = self.store.set_session_status(
                            &self.session.id,
                            SessionStatus::Requesting,
                            now_ms(),
                        );
                        self.sink.on_event(CoreEvent::NetworkResumed {
                            session_id: self.session.id.clone(),
                        });
                    }
                    Err(error) => {
                        return self.finish_usage(model_id.as_str(), output.usage, Err(error));
                    }
                }
            }
            let (tx, mut rx) = mpsc::channel(32);
            let attempt = request.clone();
            let attempt_cancel = cancel.clone();
            let attempt_provider = Arc::clone(&provider);
            let provider_task = tokio::spawn(async move {
                attempt_provider
                    .stream_chat(&attempt, tx, attempt_cancel)
                    .await
            });
            let collected = self.collect(&mut rx, &mut output).await;
            if let Err(error) = collected {
                provider_task.abort();
                return self.finish_usage(model_id.as_str(), output.usage, Err(error));
            }
            let result = provider_task
                .await
                .map_err(|_| CoreError::Internal {
                    message: "provider task stopped".into(),
                })
                .and_then(|provider_result| provider_result.map_err(CoreError::from))
                .and_then(|()| require_finished(output.finished));
            if let Err(error) = result {
                if matches!(error, CoreError::Network { .. })
                    && let Some(network) = self.network.as_ref()
                    && network.enabled()
                {
                    self.sink.on_event(CoreEvent::NetworkPaused {
                        session_id: self.session.id.clone(),
                    });
                    if let Err(wait_error) = self.wait_for_network(network, &cancel).await {
                        return self.finish_usage(model_id.as_str(), output.usage, Err(wait_error));
                    }
                    self.sink.on_event(CoreEvent::NetworkResumed {
                        session_id: self.session.id.clone(),
                    });
                    output.finished = false;
                    continue;
                }
                return self.finish_usage(model_id.as_str(), output.usage, Err(error));
            }
            return self.finish_usage(model_id.as_str(), output.usage, Ok(()));
        }
    }

    async fn wait_for_network(
        &self,
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
