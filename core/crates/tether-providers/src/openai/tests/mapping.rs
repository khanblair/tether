//! Stream mapping tests.

use serde_json::json;
use tether_types::ToolCallId;

use crate::openai::stream::StreamState;
use crate::sse::SseParser;
use crate::transport::StreamMapper;
use crate::{ProviderError, ResponseUsage, StopReason, StreamEvent};

const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/openai-text.sse"
));
const TOOL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/openai-tool-call.sse"
));
const REASONING: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/openai-reasoning.sse"
));
const CACHE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/openai-cache-usage.sse"
));
const INCOMPLETE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/openai-incomplete.sse"
));
const ERROR: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/openai-error.sse"
));
const TRUNCATED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/openai-truncated.sse"
));

fn run(fixture: &str) -> (Vec<StreamEvent>, Result<(), ProviderError>) {
    let mut state = StreamState::default();
    let mut events = Vec::new();
    for event in SseParser::new().push(fixture.as_bytes()) {
        match state.handle(&event) {
            Ok(mapped) => events.extend(mapped),
            Err(error) => return (events, Err(error)),
        }
    }
    (events, state.finish())
}

#[test]
fn text_maps_to_usage_and_finished() {
    let (events, result) = run(TEXT);
    assert_eq!(result, Ok(()));
    assert_eq!(
        events,
        vec![
            StreamEvent::TextDelta {
                text: "Hello".to_owned()
            },
            StreamEvent::TextDelta {
                text: " world".to_owned()
            },
            StreamEvent::Usage(ResponseUsage {
                input_tokens: 12,
                output_tokens: 4,
                ..ResponseUsage::default()
            }),
            StreamEvent::Finished {
                stop_reason: StopReason::EndTurn
            }
        ]
    );
}

#[test]
fn tool_arguments_finish_once() {
    let (events, result) = run(TOOL);
    let id = ToolCallId::new("call_1");
    assert_eq!(result, Ok(()));
    assert_eq!(
        events,
        vec![
            StreamEvent::ToolCallStarted {
                id: id.clone(),
                name: "read_file".to_owned()
            },
            StreamEvent::ToolCallArgsDelta {
                id: id.clone(),
                json_fragment: "{\"pa".to_owned()
            },
            StreamEvent::ToolCallArgsDelta {
                id: id.clone(),
                json_fragment: "th\":\"src/".to_owned()
            },
            StreamEvent::ToolCallArgsDelta {
                id: id.clone(),
                json_fragment: "main.rs\"}".to_owned()
            },
            StreamEvent::ToolCallFinished { id },
            StreamEvent::Usage(ResponseUsage {
                input_tokens: 20,
                output_tokens: 8,
                ..ResponseUsage::default()
            }),
            StreamEvent::Finished {
                stop_reason: StopReason::ToolUse
            }
        ]
    );
}

#[test]
fn reasoning_and_cache_usage_map() {
    let (reasoning, _) = run(REASONING);
    assert!(
        reasoning
            .iter()
            .any(|event| matches!(event, StreamEvent::ThinkingDelta { text } if text == "Plan"))
    );
    let (cache, result) = run(CACHE);
    assert_eq!(result, Ok(()));
    assert!(cache.contains(&StreamEvent::Usage(ResponseUsage {
        input_tokens: 80,
        output_tokens: 7,
        cache_read_tokens: 420,
        cache_write_tokens: 0
    })));
}

#[test]
fn incomplete_error_and_truncated_streams_map() {
    let (incomplete, result) = run(INCOMPLETE);
    assert_eq!(result, Ok(()));
    assert_eq!(
        incomplete,
        vec![
            StreamEvent::TextDelta {
                text: "Cut off".to_owned()
            },
            StreamEvent::Usage(ResponseUsage {
                input_tokens: 11,
                output_tokens: 4,
                ..ResponseUsage::default()
            }),
            StreamEvent::Finished {
                stop_reason: StopReason::MaxTokens
            }
        ]
    );
    let (partial, result) = run(ERROR);
    assert_eq!(
        partial,
        vec![StreamEvent::TextDelta {
            text: "Partial".to_owned()
        }]
    );
    assert!(matches!(result, Err(ProviderError::RateLimited { .. })));
    let (_, result) = run(TRUNCATED);
    assert!(matches!(result, Err(ProviderError::MalformedStream { .. })));
}

#[test]
fn failed_response_maps_by_error_code() {
    let mut state = StreamState::default();
    let event = crate::sse::SseEvent {
        data: json!({
            "type": "response.failed",
            "response": {"error": {"code": "invalid_prompt", "message": "bad prompt"}}
        })
        .to_string(),
        ..Default::default()
    };
    assert_eq!(
        state.handle(&event).unwrap_err(),
        ProviderError::InvalidRequest {
            message: "bad prompt".to_owned()
        }
    );
}

#[test]
fn unknown_events_are_ignored() {
    let mut state = StreamState::default();
    let event = crate::sse::SseEvent {
        data: json!({"type":"future.event"}).to_string(),
        ..Default::default()
    };
    assert_eq!(state.handle(&event).unwrap(), Vec::new());
}

#[test]
fn empty_argument_fragments_are_not_forwarded() {
    let sse = concat!(
        "data: {\"type\":\"response.output_item.added\",\"item\":{\"type\":\"function_call\",",
        "\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"read_file\"}}\n\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"item_id\":\"fc_1\",\"delta\":\"\"}\n\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"item_id\":\"fc_1\",\"delta\":\"{}\"}\n\n",
    );
    let (events, _) = run(sse);
    let id = tether_types::ToolCallId::new("call_1");
    assert_eq!(
        events,
        vec![
            StreamEvent::ToolCallStarted {
                id: id.clone(),
                name: "read_file".to_owned()
            },
            StreamEvent::ToolCallArgsDelta {
                id,
                json_fragment: "{}".to_owned()
            },
        ]
    );
}
