//! Maps OpenAI-compatible Chat Completions streams to provider events.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_types::ToolCallId;

use super::wire::{Chunk, ErrorCode, WireError};
use crate::sse::SseEvent;
use crate::transport::StreamMapper;
use crate::{ProviderError, ResponseUsage, StopReason, StreamEvent};

static RESPONSE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct OpenCall {
    id: ToolCallId,
    finished: bool,
}

#[derive(Debug)]
pub(super) struct StreamState {
    calls: BTreeMap<u64, OpenCall>,
    stop_reason: Option<StopReason>,
    finished: bool,
    prefix: String,
    dialect: super::Dialect,
}

impl Default for StreamState {
    fn default() -> Self {
        Self::new(super::Dialect::Plain)
    }
}

impl StreamState {
    pub(super) fn new(dialect: super::Dialect) -> Self {
        Self {
            calls: BTreeMap::new(),
            stop_reason: None,
            finished: false,
            prefix: response_prefix(),
            dialect,
        }
    }
}

impl StreamMapper for StreamState {
    fn handle(&mut self, event: &SseEvent) -> Result<Vec<StreamEvent>, ProviderError> {
        if self.finished {
            return Ok(Vec::new());
        }
        if event.data.trim() == "[DONE]" {
            return Ok(self.done());
        }
        let chunk: Chunk =
            serde_json::from_str(&event.data).map_err(|error| ProviderError::MalformedStream {
                message: format!("could not read an OpenAI-compatible stream event: {error}"),
            })?;
        if let Some(error) = chunk.error {
            return Err(stream_error(error));
        }
        let mut events = Vec::new();
        for choice in chunk.choices {
            events.extend(self.choice_events(choice));
        }
        if let Some(usage) = chunk.usage {
            events.push(usage_event(&usage, self.dialect));
        }
        Ok(events)
    }

    fn is_finished(&self) -> bool {
        self.finished
    }

    fn finish(&self) -> Result<(), ProviderError> {
        if self.finished {
            Ok(())
        } else {
            Err(ProviderError::MalformedStream {
                message: "the stream ended before [DONE]".to_owned(),
            })
        }
    }
}

impl StreamState {
    fn choice_events(&mut self, choice: super::wire::Choice) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        events.extend(text_event(choice.delta.content));
        events.extend(text_event(choice.delta.refusal));
        events.extend(thinking_event(
            choice.delta.reasoning_content.or(choice.delta.reasoning),
        ));
        for call in choice.delta.tool_calls {
            events.extend(self.tool_call(call));
        }
        if let Some(reason) = choice.finish_reason {
            self.stop_reason = Some(finish_reason(&reason, !self.calls.is_empty()));
            events.extend(self.finish_calls());
        }
        events
    }

    fn tool_call(&mut self, delta: super::wire::ToolCallDelta) -> Vec<StreamEvent> {
        let index = delta.index;
        let mut events = Vec::new();
        if !self.calls.contains_key(&index) {
            let id = ToolCallId::new(
                delta
                    .id
                    .unwrap_or_else(|| format!("{}_{}", self.prefix, index)),
            );
            let name = delta.function.name.unwrap_or_default();
            self.calls.insert(
                index,
                OpenCall {
                    id: id.clone(),
                    finished: false,
                },
            );
            events.push(StreamEvent::ToolCallStarted { id, name });
        }
        if let Some(arguments) = delta.function.arguments.filter(|value| !value.is_empty())
            && let Some(call) = self.calls.get(&index)
        {
            events.push(StreamEvent::ToolCallArgsDelta {
                id: call.id.clone(),
                json_fragment: arguments,
            });
        }
        events
    }

    fn finish_calls(&mut self) -> Vec<StreamEvent> {
        self.calls.values_mut().filter_map(finish_call).collect()
    }

    fn done(&mut self) -> Vec<StreamEvent> {
        self.finished = true;
        let mut events = self.finish_calls();
        events.push(StreamEvent::Finished {
            stop_reason: self
                .stop_reason
                .clone()
                .unwrap_or(default_stop_reason(&self.calls)),
        });
        events
    }
}

fn response_prefix() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let counter = RESPONSE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("compat_{millis}_{counter}")
}

fn finish_call(call: &mut OpenCall) -> Option<StreamEvent> {
    if call.finished {
        return None;
    }
    call.finished = true;
    Some(StreamEvent::ToolCallFinished {
        id: call.id.clone(),
    })
}

fn default_stop_reason(calls: &BTreeMap<u64, OpenCall>) -> StopReason {
    if calls.is_empty() {
        StopReason::EndTurn
    } else {
        StopReason::ToolUse
    }
}

fn usage_event(usage: &super::wire::Usage, dialect: super::Dialect) -> StreamEvent {
    let cached = if dialect == super::Dialect::DeepSeek {
        usage.prompt_cache_hit_tokens
    } else {
        usage.prompt_tokens_details.cached_tokens
    };
    let written = usage.prompt_tokens_details.cache_write_tokens;
    StreamEvent::Usage(ResponseUsage {
        input_tokens: usage
            .prompt_tokens
            .saturating_sub(cached.saturating_add(written)),
        output_tokens: usage.completion_tokens,
        cache_read_tokens: cached,
        cache_write_tokens: written,
    })
}

fn finish_reason(reason: &str, has_tool_call: bool) -> StopReason {
    match reason {
        "stop" => {
            if has_tool_call {
                StopReason::ToolUse
            } else {
                StopReason::EndTurn
            }
        }
        "tool_calls" => StopReason::ToolUse,
        "length" => StopReason::MaxTokens,
        "content_filter" => StopReason::ContentFilter,
        other => StopReason::Other(other.to_owned()),
    }
}

fn stream_error(error: WireError) -> ProviderError {
    match error.code.as_ref().and_then(ErrorCode::number) {
        Some(401..=403) => ProviderError::Auth {
            message: error.message,
        },
        Some(429) => ProviderError::RateLimited {
            message: error.message,
            retry_after_secs: None,
        },
        Some(400 | 404 | 413 | 422) => ProviderError::InvalidRequest {
            message: error.message,
        },
        _ => ProviderError::Server {
            status: 500,
            message: error.message,
        },
    }
}

fn text_event(text: Option<String>) -> Vec<StreamEvent> {
    text.filter(|value| !value.is_empty())
        .map(|text| vec![StreamEvent::TextDelta { text }])
        .unwrap_or_default()
}

fn thinking_event(text: Option<String>) -> Vec<StreamEvent> {
    text.filter(|value| !value.is_empty())
        .map(|text| vec![StreamEvent::ThinkingDelta { text }])
        .unwrap_or_default()
}
