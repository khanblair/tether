//! One provider response with saved text batches and bounded tool calls.

use std::sync::Arc;
use std::time::Duration;

use tether_providers::{ChatProvider, ChatRequest, ResponseUsage, StopReason, StreamEvent};
use tether_types::{CoreError, CoreEvent, MessageId, SessionStatus, ToolCallId, UsageSnapshot};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};

use super::{
    MAX_ARGUMENT_BYTES, MAX_CALLS_PER_EXCHANGE, TEXT_BATCH_CHARS, TEXT_BATCH_MS, ToolLoopContext,
    cancelled, invalid, now_ms, storage_error,
};

pub(super) struct Call {
    pub(super) id: ToolCallId,
    pub(super) name: String,
    pub(super) args: String,
    pub(super) complete: bool,
}

pub(super) struct Response {
    pub(super) text: String,
    pub(super) thinking: String,
    pub(super) message_id: Option<MessageId>,
    pub(super) calls: Vec<Call>,
    pub(super) stop: StopReason,
    pub(super) usage: UsageSnapshot,
    pub(super) price_known: bool,
}

#[derive(Default)]
struct StreamDraft {
    text: String,
    thinking: String,
    pending_text: String,
    pending_thinking: String,
    pending_chars: usize,
    deadline: Option<Instant>,
    message_id: Option<MessageId>,
}

impl StreamDraft {
    fn push_text(&mut self, delta: &str) {
        self.text.push_str(delta);
        self.pending_text.push_str(delta);
        self.mark_pending(delta);
    }

    fn push_thinking(&mut self, delta: &str) {
        self.thinking.push_str(delta);
        self.pending_thinking.push_str(delta);
        self.mark_pending(delta);
    }

    fn mark_pending(&mut self, delta: &str) {
        self.pending_chars += delta.chars().count();
        self.deadline
            .get_or_insert_with(|| Instant::now() + Duration::from_millis(TEXT_BATCH_MS));
    }

    #[expect(
        clippy::excessive_nesting,
        reason = "each persisted batch emits its matching status and deltas"
    )]
    fn flush(&mut self, context: &mut ToolLoopContext<'_>) -> Result<(), CoreError> {
        if self.pending_text.is_empty() && self.pending_thinking.is_empty() {
            self.deadline = None;
            return Ok(());
        }
        let id = if let Some(id) = &self.message_id {
            id.clone()
        } else {
            let id = context
                .store
                .create_assistant_message(context.session_id, now_ms())
                .map_err(|_| storage_error())?;
            self.message_id = Some(id.clone());
            context
                .store
                .set_session_status(context.session_id, SessionStatus::Streaming, now_ms())
                .map_err(|_| storage_error())?;
            if let Some(sink) = context.sink {
                sink.on_event(CoreEvent::SessionUpdated {
                    session_id: context.session_id.clone(),
                    status: SessionStatus::Streaming,
                });
            }
            id
        };
        context
            .store
            .save_assistant_parts(&id, &self.text, &self.thinking)
            .map_err(|_| storage_error())?;
        if let Some(sink) = context.sink {
            if !self.pending_thinking.is_empty() {
                sink.on_event(CoreEvent::ThinkingDelta {
                    session_id: context.session_id.clone(),
                    message_id: id.clone(),
                    text: std::mem::take(&mut self.pending_thinking),
                });
            }
            if !self.pending_text.is_empty() {
                sink.on_event(CoreEvent::TextDelta {
                    session_id: context.session_id.clone(),
                    message_id: id,
                    text: std::mem::take(&mut self.pending_text),
                });
            }
        } else {
            self.pending_text.clear();
            self.pending_thinking.clear();
        }
        self.pending_chars = 0;
        self.deadline = None;
        Ok(())
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the provider stream selects cancellation, batch deadline, and events"
)]
pub(super) async fn response(
    provider: Arc<dyn ChatProvider>,
    request: ChatRequest,
    context: &mut ToolLoopContext<'_>,
) -> Result<Response, CoreError> {
    let model_id = request.model_id.clone();
    let (tx, mut rx) = mpsc::channel(32);
    let token = context.cancel.clone();
    let task = tokio::spawn(async move { provider.stream_chat(&request, tx, token).await });
    let mut draft = StreamDraft::default();
    let mut calls: Vec<Call> = Vec::new();
    let mut stop = None;
    let mut usage = ResponseUsage::default();
    loop {
        let event = if let Some(deadline) = draft.deadline {
            tokio::select! {
                () = context.cancel.cancelled() => {
                    task.abort();
                    draft.flush(context)?;
                    return Err(cancelled());
                }
                () = sleep_until(deadline) => { draft.flush(context)?; continue; }
                event = rx.recv() => event,
            }
        } else {
            tokio::select! {
                () = context.cancel.cancelled() => {
                    task.abort();
                    draft.flush(context)?;
                    return Err(cancelled());
                }
                event = rx.recv() => event,
            }
        };
        let Some(event) = event else { break };
        if stop.is_some() {
            task.abort();
            draft.flush(context)?;
            return Err(invalid("provider sent an event after finish"));
        }
        if let Err(error) = accept(event, &mut draft, &mut calls, &mut stop, &mut usage) {
            task.abort();
            draft.flush(context)?;
            return Err(error);
        }
        if draft.pending_chars >= TEXT_BATCH_CHARS {
            draft.flush(context)?;
        }
    }
    draft.flush(context)?;
    task.await
        .map_err(|_| invalid("provider task stopped"))?
        .map_err(CoreError::from)?;
    let stop = stop.ok_or_else(|| invalid("provider stream ended without finish"))?;
    if calls.iter().any(|call| !call.complete) {
        return Err(invalid("provider left a tool call unfinished"));
    }
    if !calls.is_empty() && stop != StopReason::ToolUse {
        return Err(invalid(
            "provider finished without requesting its tool calls",
        ));
    }
    if calls.iter().any(|call| {
        !matches!(
            serde_json::from_str::<serde_json::Value>(&call.args),
            Ok(serde_json::Value::Object(_))
        )
    }) {
        return Err(invalid("tool arguments must be a JSON object"));
    }
    let priced = crate::usage::price_response(context.store, context.session_id, &model_id, usage)
        .map_err(|_| storage_error())?;
    Ok(Response {
        text: draft.text,
        thinking: draft.thinking,
        message_id: draft.message_id,
        calls,
        stop,
        usage: priced.snapshot,
        price_known: priced.price_known,
    })
}

fn accept(
    event: StreamEvent,
    draft: &mut StreamDraft,
    calls: &mut Vec<Call>,
    stop: &mut Option<StopReason>,
    usage: &mut ResponseUsage,
) -> Result<(), CoreError> {
    match event {
        StreamEvent::TextDelta { text: delta } => draft.push_text(&delta),
        StreamEvent::ThinkingDelta { text: delta } => draft.push_thinking(&delta),
        StreamEvent::Usage(value) => *usage = value,
        StreamEvent::ToolCallStarted { id, name } => {
            if calls.len() >= MAX_CALLS_PER_EXCHANGE || calls.iter().any(|call| call.id == id) {
                return Err(invalid("too many or duplicate tool calls"));
            }
            calls.push(Call {
                id,
                name,
                args: String::new(),
                complete: false,
            });
        }
        StreamEvent::ToolCallArgsDelta { id, json_fragment } => {
            let call = calls
                .iter_mut()
                .find(|call| call.id == id && !call.complete)
                .ok_or_else(|| invalid("tool arguments have no active call"))?;
            if call.args.len().saturating_add(json_fragment.len()) > MAX_ARGUMENT_BYTES {
                return Err(invalid("tool arguments exceed the limit"));
            }
            call.args.push_str(&json_fragment);
        }
        StreamEvent::ToolCallFinished { id } => {
            let call = calls
                .iter_mut()
                .find(|call| call.id == id && !call.complete)
                .ok_or_else(|| invalid("tool finish has no active call"))?;
            call.complete = true;
        }
        StreamEvent::Finished { stop_reason } => *stop = Some(stop_reason),
    }
    Ok(())
}
