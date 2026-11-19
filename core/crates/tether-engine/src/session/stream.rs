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
    ) -> (UsageSnapshot, Result<(), CoreError>) {
        let (tx, mut rx) = mpsc::channel(32);
        let task = tokio::spawn(async move { provider.stream_chat(&request, tx, cancel).await });
        let mut output = Output::default();
        let result = self.collect(&mut rx, &mut output).await;
        if let Err(error) = result {
            task.abort();
            return (output.usage, Err(error));
        }
        let result = task
            .await
            .map_err(|_| CoreError::Internal {
                message: "provider task stopped".into(),
            })
            .and_then(|provider_result| provider_result.map_err(CoreError::from))
            .and_then(|()| require_finished(output.finished));
        (output.usage, result)
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
            StreamEvent::Usage(usage) => output.usage = usage_snapshot(usage),
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
    usage: UsageSnapshot,
    finished: bool,
}

fn usage_snapshot(usage: ResponseUsage) -> UsageSnapshot {
    UsageSnapshot {
        tokens_in: usage
            .input_tokens
            .saturating_add(usage.cache_read_tokens)
            .saturating_add(usage.cache_write_tokens),
        tokens_out: usage.output_tokens,
        cost_micros: 0,
    }
}
