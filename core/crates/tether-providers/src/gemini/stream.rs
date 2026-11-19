//! Maps Gemini streaming responses to provider events.

use std::collections::HashSet;

use tether_types::ToolCallId;

use super::wire::{Response, Usage, WireError};
use crate::sse::SseEvent;
use crate::transport::StreamMapper;
use crate::{ProviderError, ResponseUsage, StopReason, StreamEvent};

#[derive(Debug, Default)]
pub(super) struct StreamState {
    calls: HashSet<ToolCallId>,
    next_call: usize,
    has_tool_call: bool,
    finished: bool,
}

impl StreamMapper for StreamState {
    fn handle(&mut self, event: &SseEvent) -> Result<Vec<StreamEvent>, ProviderError> {
        if self.finished {
            return Ok(Vec::new());
        }
        let response: Response =
            serde_json::from_str(&event.data).map_err(|error| ProviderError::MalformedStream {
                message: format!("could not read a Gemini stream event: {error}"),
            })?;
        if let Some(error) = response.error {
            return Err(map_stream_error(error));
        }
        let mut events = response
            .usage
            .as_ref()
            .map(usage_event)
            .into_iter()
            .collect::<Vec<_>>();
        if let Some(candidate) = response.candidates.first() {
            events.extend(self.content_events(candidate.content.as_ref()));
            events.extend(self.finished_event(candidate.finish_reason.as_deref()));
        } else if response
            .prompt_feedback
            .as_ref()
            .and_then(|feedback| feedback.block_reason.as_ref())
            .is_some()
        {
            self.finished = true;
            events.push(StreamEvent::Finished {
                stop_reason: StopReason::ContentFilter,
            });
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
                message: "the Gemini stream ended before a finish reason or block".to_owned(),
            })
        }
    }
}

impl StreamState {
    fn content_events(&mut self, content: Option<&super::wire::Content>) -> Vec<StreamEvent> {
        content
            .into_iter()
            .flat_map(|content| content.parts.iter())
            .flat_map(|part| self.part_events(part))
            .collect()
    }

    fn part_events(&mut self, part: &super::wire::Part) -> Vec<StreamEvent> {
        if let Some(call) = &part.function_call {
            return self.function_call(call);
        }
        let Some(text) = &part.text.as_ref().filter(|text| !text.is_empty()) else {
            return Vec::new();
        };
        vec![if part.thought.unwrap_or(false) {
            StreamEvent::ThinkingDelta {
                text: (*text).clone(),
            }
        } else {
            StreamEvent::TextDelta {
                text: (*text).clone(),
            }
        }]
    }

    fn finished_event(&mut self, reason: Option<&str>) -> Vec<StreamEvent> {
        // Google may send this placeholder before the real reason, so it does not end the stream.
        let Some(reason) = reason.filter(|reason| *reason != "FINISH_REASON_UNSPECIFIED") else {
            return Vec::new();
        };
        self.finished = true;
        vec![StreamEvent::Finished {
            stop_reason: finish_reason(reason, self.has_tool_call),
        }]
    }

    fn function_call(&mut self, call: &super::wire::FunctionCall) -> Vec<StreamEvent> {
        let id = call.id.clone().unwrap_or_else(|| {
            let id = format!("gemini_call_{}", self.next_call);
            self.next_call += 1;
            id
        });
        let id = ToolCallId::new(id);
        self.calls.insert(id.clone());
        self.has_tool_call = true;
        let mut events = vec![StreamEvent::ToolCallStarted {
            id: id.clone(),
            name: call.name.clone().unwrap_or_default(),
        }];
        if let Some(args) = &call.args {
            let fragment = serde_json::to_string(args).unwrap_or_else(|_| "{}".to_owned());
            events.push(StreamEvent::ToolCallArgsDelta {
                id: id.clone(),
                json_fragment: fragment,
            });
        }
        events.push(StreamEvent::ToolCallFinished { id });
        events
    }
}

fn usage_event(usage: &Usage) -> StreamEvent {
    StreamEvent::Usage(ResponseUsage {
        input_tokens: usage.prompt.saturating_sub(usage.cached),
        output_tokens: usage.candidates + usage.thoughts,
        cache_read_tokens: usage.cached,
        cache_write_tokens: 0,
    })
}

fn finish_reason(reason: &str, has_tool_call: bool) -> StopReason {
    match reason {
        "STOP" => {
            if has_tool_call {
                StopReason::ToolUse
            } else {
                StopReason::EndTurn
            }
        }
        "MAX_TOKENS" => StopReason::MaxTokens,
        "SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII" | "IMAGE_SAFETY" => {
            StopReason::ContentFilter
        }
        other => StopReason::Other(other.to_owned()),
    }
}

fn map_stream_error(error: WireError) -> ProviderError {
    let message = error
        .message
        .unwrap_or_else(|| "Gemini stream error".to_owned());
    match error.status.as_deref() {
        Some("RESOURCE_EXHAUSTED") => ProviderError::RateLimited {
            message,
            retry_after_secs: None,
        },
        Some("PERMISSION_DENIED" | "UNAUTHENTICATED") => ProviderError::Auth { message },
        Some("INVALID_ARGUMENT" | "NOT_FOUND" | "FAILED_PRECONDITION") => {
            ProviderError::InvalidRequest { message }
        }
        _ if error.code == Some(429) => ProviderError::RateLimited {
            message,
            retry_after_secs: None,
        },
        _ if matches!(error.code, Some(401 | 403)) => ProviderError::Auth { message },
        _ if matches!(error.code, Some(400 | 404 | 412)) => {
            ProviderError::InvalidRequest { message }
        }
        _ => ProviderError::Server {
            status: 500,
            message,
        },
    }
}
