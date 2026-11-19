//! Turns Anthropic's streaming events into [`StreamEvent`] values.

use std::collections::HashMap;

use tether_types::ToolCallId;

use super::wire::{BlockStart, Delta, MessageDeltaBody, WireError, WireEvent, WireUsage};
use crate::sse::SseEvent;
use crate::transport::StreamMapper;
use crate::{ProviderError, ResponseUsage, StopReason, StreamEvent};

/// What the adapter remembers between events of one response.
#[derive(Debug, Default)]
pub(super) struct StreamState {
    /// Tool call ids by content block index. A block leaves the map when it stops.
    tool_calls: HashMap<u64, ToolCallId>,
    /// Running token totals.
    usage: ResponseUsage,
    /// The stop reason from the last `message_delta` that named one.
    stop_reason: Option<String>,
    /// True once `message_stop` produced the `Finished` event.
    finished: bool,
}

impl StreamMapper for StreamState {
    /// Maps one SSE event to zero or more stream events.
    ///
    /// The `type` field of the data picks the handler, not the `event:` name. After
    /// `message_stop` every later event is ignored, so `Finished` is never sent twice.
    fn handle(&mut self, event: &SseEvent) -> Result<Vec<StreamEvent>, ProviderError> {
        if self.finished {
            return Ok(Vec::new());
        }
        let wire: WireEvent =
            serde_json::from_str(&event.data).map_err(|error| ProviderError::MalformedStream {
                message: format!("could not read a stream event: {error}"),
            })?;
        Ok(match wire {
            WireEvent::MessageStart { message } => vec![self.update_usage(&message.usage)],
            WireEvent::ContentBlockStart {
                index,
                content_block,
            } => self.block_started(index, content_block),
            WireEvent::ContentBlockDelta { index, delta } => self.block_delta(index, delta),
            WireEvent::ContentBlockStop { index } => self.block_stopped(index),
            WireEvent::MessageDelta { delta, usage } => self.message_delta(delta, usage),
            WireEvent::MessageStop => self.stop(),
            WireEvent::Error { error } => return Err(stream_error(error)),
            WireEvent::Other => Vec::new(),
        })
    }

    /// True once the response ended with `message_stop`.
    fn is_finished(&self) -> bool {
        self.finished
    }

    /// Checks the body ended the right way. A body that stops before `message_stop` is cut off.
    fn finish(&self) -> Result<(), ProviderError> {
        if self.finished {
            return Ok(());
        }
        Err(ProviderError::MalformedStream {
            message: "the stream ended before message_stop".to_owned(),
        })
    }
}

impl StreamState {
    /// Merges the counts an event names into the totals and returns them.
    ///
    /// Counts are cumulative, so a later value replaces an earlier one.
    fn update_usage(&mut self, usage: &WireUsage) -> StreamEvent {
        let totals = &mut self.usage;
        totals.input_tokens = usage.input.unwrap_or(totals.input_tokens);
        totals.output_tokens = usage.output.unwrap_or(totals.output_tokens);
        totals.cache_write_tokens = usage.cache_write.unwrap_or(totals.cache_write_tokens);
        totals.cache_read_tokens = usage.cache_read.unwrap_or(totals.cache_read_tokens);
        StreamEvent::Usage(*totals)
    }

    /// Starts tracking a tool call. Other block types need no state.
    fn block_started(&mut self, index: u64, block: BlockStart) -> Vec<StreamEvent> {
        let BlockStart::ToolUse { id, name } = block else {
            return Vec::new();
        };
        let id = ToolCallId::new(id);
        self.tool_calls.insert(index, id.clone());
        vec![StreamEvent::ToolCallStarted { id, name }]
    }

    /// Maps a delta to text, reasoning, or a fragment of tool call arguments.
    fn block_delta(&self, index: u64, delta: Delta) -> Vec<StreamEvent> {
        match delta {
            Delta::Text { text } => vec![StreamEvent::TextDelta { text }],
            Delta::Thinking { thinking } => vec![StreamEvent::ThinkingDelta { text: thinking }],
            Delta::InputJson { partial_json } => self.args_delta(index, partial_json),
            Delta::Other => Vec::new(),
        }
    }

    /// Wraps a fragment of arguments. Empty fragments and fragments of unknown blocks are dropped.
    fn args_delta(&self, index: u64, json_fragment: String) -> Vec<StreamEvent> {
        if json_fragment.is_empty() {
            return Vec::new();
        }
        let call = self.tool_calls.get(&index);
        call.map(|id| StreamEvent::ToolCallArgsDelta {
            id: id.clone(),
            json_fragment,
        })
        .into_iter()
        .collect()
    }

    /// Ends a tool call when its block stops.
    fn block_stopped(&mut self, index: u64) -> Vec<StreamEvent> {
        let call = self.tool_calls.remove(&index);
        call.map(|id| StreamEvent::ToolCallFinished { id })
            .into_iter()
            .collect()
    }

    /// Remembers the stop reason and reports the new token totals.
    fn message_delta(
        &mut self,
        delta: MessageDeltaBody,
        usage: Option<WireUsage>,
    ) -> Vec<StreamEvent> {
        if delta.stop_reason.is_some() {
            self.stop_reason = delta.stop_reason;
        }
        usage
            .map(|usage| self.update_usage(&usage))
            .into_iter()
            .collect()
    }

    /// Ends the response with the remembered stop reason.
    fn stop(&mut self) -> Vec<StreamEvent> {
        self.finished = true;
        let stop_reason = map_stop_reason(self.stop_reason.as_deref());
        vec![StreamEvent::Finished { stop_reason }]
    }
}

/// Maps Anthropic's stop reason text. A missing reason counts as a normal end of turn.
fn map_stop_reason(reason: Option<&str>) -> StopReason {
    match reason {
        None | Some("end_turn" | "stop_sequence") => StopReason::EndTurn,
        Some("tool_use") => StopReason::ToolUse,
        Some("max_tokens") => StopReason::MaxTokens,
        Some("refusal") => StopReason::ContentFilter,
        Some(other) => StopReason::Other(other.to_owned()),
    }
}

/// Maps an in-stream `error` event to the error the same failure would give as an HTTP status.
fn stream_error(error: WireError) -> ProviderError {
    let message = error.message;
    match error.kind.as_str() {
        "overloaded_error" => ProviderError::Server {
            status: 529,
            message,
        },
        "rate_limit_error" => ProviderError::RateLimited {
            message,
            retry_after_secs: None,
        },
        "authentication_error" | "permission_error" => ProviderError::Auth { message },
        "invalid_request_error" | "not_found_error" => ProviderError::InvalidRequest { message },
        _ => ProviderError::Server {
            status: 500,
            message,
        },
    }
}
