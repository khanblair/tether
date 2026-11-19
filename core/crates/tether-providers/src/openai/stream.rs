//! Maps OpenAI Responses streaming events to provider events.

use std::collections::{HashMap, HashSet};

use tether_types::ToolCallId;

use super::wire::{OutputItem, ResponseBody, Usage, WireError, WireEvent};
use crate::sse::SseEvent;
use crate::transport::StreamMapper;
use crate::{ProviderError, ResponseUsage, StopReason, StreamEvent};

#[derive(Debug, Default)]
pub(super) struct StreamState {
    calls: HashMap<String, ToolCallId>,
    finished_calls: HashSet<ToolCallId>,
    usage: ResponseUsage,
    has_tool_call: bool,
    finished: bool,
}

impl StreamMapper for StreamState {
    fn handle(&mut self, event: &SseEvent) -> Result<Vec<StreamEvent>, ProviderError> {
        if self.finished {
            return Ok(Vec::new());
        }
        let wire: WireEvent =
            serde_json::from_str(&event.data).map_err(|error| ProviderError::MalformedStream {
                message: format!("could not read a stream event: {error}"),
            })?;
        match wire {
            WireEvent::OutputItemAdded { item } => Ok(self.item_started(item)),
            WireEvent::OutputTextDelta { delta } | WireEvent::RefusalDelta { delta } => {
                Ok(vec![StreamEvent::TextDelta { text: delta }])
            }
            WireEvent::ReasoningSummaryDelta { delta } | WireEvent::ReasoningDelta { delta } => {
                Ok(vec![StreamEvent::ThinkingDelta { text: delta }])
            }
            WireEvent::FunctionArgsDelta { item_id, delta } => Ok(self.args_delta(&item_id, delta)),
            WireEvent::OutputItemDone { item } => Ok(self.item_finished(&item)),
            WireEvent::FunctionArgsDone { item_id } => Ok(self.finish_call(&item_id)),
            WireEvent::Completed { response } => Ok(self.complete(&response)),
            WireEvent::Incomplete { response } => Ok(self.incomplete(&response)),
            WireEvent::Failed { response } => Err(stream_error(response.error)),
            WireEvent::Error { message, code } => {
                Err(stream_error(Some(WireError { message, code })))
            }
            WireEvent::Other => Ok(Vec::new()),
        }
    }

    fn is_finished(&self) -> bool {
        self.finished
    }

    fn finish(&self) -> Result<(), ProviderError> {
        if self.finished {
            Ok(())
        } else {
            Err(ProviderError::MalformedStream {
                message: "the stream ended before response.completed or response.incomplete"
                    .to_owned(),
            })
        }
    }
}

impl StreamState {
    fn item_started(&mut self, item: OutputItem) -> Vec<StreamEvent> {
        if item.kind != "function_call" {
            return Vec::new();
        }
        let Some(call_id) = item.call_id else {
            return Vec::new();
        };
        let id = ToolCallId::new(call_id);
        if let Some(item_id) = item.id {
            self.calls.insert(item_id, id.clone());
        }
        self.has_tool_call = true;
        vec![StreamEvent::ToolCallStarted {
            id,
            name: item.name.unwrap_or_default(),
        }]
    }

    fn args_delta(&self, item_id: &str, delta: String) -> Vec<StreamEvent> {
        if delta.is_empty() {
            return Vec::new();
        }
        self.calls
            .get(item_id)
            .cloned()
            .map(|id| {
                vec![StreamEvent::ToolCallArgsDelta {
                    id,
                    json_fragment: delta,
                }]
            })
            .unwrap_or_default()
    }

    fn item_finished(&mut self, item: &OutputItem) -> Vec<StreamEvent> {
        if item.kind != "function_call" {
            return Vec::new();
        }
        item.id
            .as_deref()
            .map(|item_id| self.finish_call(item_id))
            .unwrap_or_default()
    }

    fn finish_call(&mut self, item_id: &str) -> Vec<StreamEvent> {
        let Some(id) = self.calls.get(item_id).cloned() else {
            return Vec::new();
        };
        if self.finished_calls.insert(id.clone()) {
            vec![StreamEvent::ToolCallFinished { id }]
        } else {
            Vec::new()
        }
    }

    fn complete(&mut self, response: &ResponseBody) -> Vec<StreamEvent> {
        self.finished = true;
        let mut events = response
            .usage
            .as_ref()
            .map(|usage| self.update_usage(usage))
            .into_iter()
            .collect::<Vec<_>>();
        events.push(StreamEvent::Finished {
            stop_reason: if self.has_tool_call {
                StopReason::ToolUse
            } else {
                StopReason::EndTurn
            },
        });
        events
    }

    fn incomplete(&mut self, response: &ResponseBody) -> Vec<StreamEvent> {
        self.finished = true;
        let mut events = response
            .usage
            .as_ref()
            .map(|usage| self.update_usage(usage))
            .into_iter()
            .collect::<Vec<_>>();
        let reason = response
            .incomplete_details
            .as_ref()
            .and_then(|details| details.reason.as_deref());
        events.push(StreamEvent::Finished {
            stop_reason: incomplete_reason(reason),
        });
        events
    }

    fn update_usage(&mut self, usage: &Usage) -> StreamEvent {
        let cached = usage.input_tokens_details.cached_tokens;
        self.usage = ResponseUsage {
            input_tokens: usage.input_tokens.saturating_sub(cached),
            output_tokens: usage.output_tokens,
            cache_read_tokens: cached,
            cache_write_tokens: 0,
        };
        StreamEvent::Usage(self.usage)
    }
}

fn incomplete_reason(reason: Option<&str>) -> StopReason {
    match reason {
        Some("max_output_tokens") => StopReason::MaxTokens,
        Some("content_filter") => StopReason::ContentFilter,
        Some(other) => StopReason::Other(other.to_owned()),
        None => StopReason::Other("incomplete".to_owned()),
    }
}

fn stream_error(error: Option<WireError>) -> ProviderError {
    let Some(error) = error else {
        return ProviderError::Server {
            status: 500,
            message: "OpenAI response failed".to_owned(),
        };
    };
    match error.code.as_deref() {
        Some("rate_limit_exceeded") => ProviderError::RateLimited {
            message: error.message,
            retry_after_secs: None,
        },
        Some("invalid_api_key" | "authentication_error" | "insufficient_quota") => {
            ProviderError::Auth {
                message: error.message,
            }
        }
        Some("context_length_exceeded" | "invalid_request_error" | "invalid_prompt") => {
            ProviderError::InvalidRequest {
                message: error.message,
            }
        }
        _ => ProviderError::Server {
            status: 500,
            message: error.message,
        },
    }
}
