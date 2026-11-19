//! Gemini stream mapping tests.

use serde_json::json;
use tether_types::ToolCallId;

use crate::gemini::stream::StreamState;
use crate::sse::SseParser;
use crate::transport::StreamMapper;
use crate::{ProviderError, ResponseUsage, StopReason, StreamEvent};

const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/gemini-text.sse"
));
const TOOL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/gemini-tool-call.sse"
));
const THINKING: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/gemini-thinking.sse"
));
const CACHE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/gemini-cache-usage.sse"
));
const MAX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/gemini-max-tokens.sse"
));
const BLOCKED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/gemini-blocked.sse"
));
const ERROR: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/gemini-error.sse"
));
const TRUNCATED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/gemini-truncated.sse"
));

fn run(fixture: &str) -> (Vec<StreamEvent>, Result<(), ProviderError>) {
    run_with_prefix(fixture, "gemini_test_0")
}

fn run_with_prefix(fixture: &str, prefix: &str) -> (Vec<StreamEvent>, Result<(), ProviderError>) {
    let mut state = StreamState::new(prefix.to_owned());
    let mut events = Vec::new();
    for event in SseParser::new().push(fixture.as_bytes()) {
        match state.handle(&event) {
            Ok(mapped) => events.extend(mapped),
            Err(error) => return (events, Err(error)),
        }
    }
    (events, state.finish())
}

fn tool_events(id: &str) -> Vec<StreamEvent> {
    let id = ToolCallId::new(id);
    vec![
        StreamEvent::ToolCallStarted {
            id: id.clone(),
            name: "read_file".into(),
        },
        StreamEvent::ToolCallArgsDelta {
            id: id.clone(),
            json_fragment: "{\"path\":\"a.rs\"}".into(),
        },
        StreamEvent::ToolCallFinished { id },
        StreamEvent::Finished {
            stop_reason: StopReason::ToolUse,
        },
    ]
}

#[test]
fn text_usage_and_stop_map_in_order() {
    let (events, result) = run(TEXT);
    assert_eq!(result, Ok(()));
    assert_eq!(
        events[0],
        StreamEvent::TextDelta {
            text: "Hello".into()
        }
    );
    assert!(events.contains(&StreamEvent::TextDelta {
        text: " world".into()
    }));
    assert!(events.contains(&StreamEvent::Usage(ResponseUsage {
        input_tokens: 10,
        output_tokens: 4,
        ..ResponseUsage::default()
    })));
    assert_eq!(
        events.last(),
        Some(&StreamEvent::Finished {
            stop_reason: StopReason::EndTurn
        })
    );
}

#[test]
fn tool_call_thinking_cache_and_max_map() {
    let (events, result) = run(TOOL);
    let id = ToolCallId::new("gemini_test_0_0");
    assert_eq!(result, Ok(()));
    assert!(events.contains(&StreamEvent::ToolCallStarted {
        id: id.clone(),
        name: "read_file".into()
    }));
    assert!(events.contains(&StreamEvent::ToolCallArgsDelta {
        id: id.clone(),
        json_fragment: "{\"path\":\"a.rs\"}".into()
    }));
    assert!(events.contains(&StreamEvent::ToolCallFinished { id }));
    assert!(run(THINKING).0.contains(&StreamEvent::ThinkingDelta {
        text: "Plan".into()
    }));
    assert!(run(CACHE).0.contains(&StreamEvent::Usage(ResponseUsage {
        input_tokens: 20,
        output_tokens: 9,
        cache_read_tokens: 40,
        cache_write_tokens: 0
    })));
    assert!(run(MAX).0.contains(&StreamEvent::Finished {
        stop_reason: StopReason::MaxTokens
    }));
}

#[test]
fn blocked_and_error_streams_map() {
    assert_eq!(
        run(BLOCKED).0,
        vec![StreamEvent::Finished {
            stop_reason: StopReason::ContentFilter
        }]
    );
    let (events, result) = run(ERROR);
    assert_eq!(
        events,
        vec![StreamEvent::TextDelta {
            text: "Partial".into()
        }]
    );
    assert!(matches!(result, Err(ProviderError::RateLimited { .. })));
    assert!(matches!(
        run(TRUNCATED).1,
        Err(ProviderError::MalformedStream { .. })
    ));
}

#[test]
fn unknown_parts_are_ignored() {
    let mut state = StreamState::new("gemini_test_0".to_owned());
    let event = crate::sse::SseEvent {
        data: json!({"candidates":[{"content":{"parts":[{"inlineData":{}}]}}]}).to_string(),
        ..Default::default()
    };
    assert_eq!(state.handle(&event).unwrap(), Vec::new());
}

#[test]
fn a_function_call_stream_ends_with_tool_use() {
    let id = ToolCallId::new("gemini_test_0_0");
    assert_eq!(
        run(TOOL).0,
        vec![
            StreamEvent::ToolCallStarted {
                id: id.clone(),
                name: "read_file".into()
            },
            StreamEvent::ToolCallArgsDelta {
                id: id.clone(),
                json_fragment: "{\"path\":\"a.rs\"}".into()
            },
            StreamEvent::ToolCallFinished { id },
            StreamEvent::Finished {
                stop_reason: StopReason::ToolUse
            },
        ]
    );
}

#[test]
fn calls_without_ids_get_different_generated_ids() {
    let sse = concat!(
        "data: {\"candidates\":[{\"content\":{\"parts\":[",
        "{\"functionCall\":{\"name\":\"a\",\"args\":{}}},",
        "{\"functionCall\":{\"name\":\"b\",\"args\":{}}}]}}]}\n\n",
    );
    let started: Vec<_> = run(sse)
        .0
        .into_iter()
        .filter_map(|event| match event {
            StreamEvent::ToolCallStarted { id, .. } => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(
        started,
        vec![
            ToolCallId::new("gemini_test_0_0"),
            ToolCallId::new("gemini_test_0_1")
        ]
    );
}

#[test]
fn different_response_prefixes_produce_different_generated_ids() {
    assert_eq!(
        run_with_prefix(TOOL, "response_a").0,
        tool_events("response_a_0")
    );
    assert_eq!(
        run_with_prefix(TOOL, "response_b").0,
        tool_events("response_b_0")
    );
}

#[test]
fn a_provider_call_id_is_preserved() {
    let sse = "data: {\"candidates\":[{\"content\":{\"parts\":[{\"functionCall\":{\"id\":\"provider-call\",\"name\":\"a\",\"args\":{}}}]},\"finishReason\":\"STOP\"}]}\n\n";

    assert_eq!(
        run_with_prefix(sse, "response").0,
        vec![
            StreamEvent::ToolCallStarted {
                id: ToolCallId::new("provider-call"),
                name: "a".into(),
            },
            StreamEvent::ToolCallArgsDelta {
                id: ToolCallId::new("provider-call"),
                json_fragment: "{}".into(),
            },
            StreamEvent::ToolCallFinished {
                id: ToolCallId::new("provider-call"),
            },
            StreamEvent::Finished {
                stop_reason: StopReason::ToolUse,
            },
        ]
    );
}

#[test]
fn empty_text_parts_are_ignored() {
    let sse = concat!(
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"\"}]},",
        "\"finishReason\":\"STOP\"}]}\n\n",
    );
    assert_eq!(
        run(sse).0,
        vec![StreamEvent::Finished {
            stop_reason: StopReason::EndTurn
        }]
    );
}

#[test]
fn an_unspecified_finish_reason_does_not_end_the_stream() {
    let sse = concat!(
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hi\"}]},",
        "\"finishReason\":\"FINISH_REASON_UNSPECIFIED\"}]}\n\n",
        "data: {\"candidates\":[{\"finishReason\":\"STOP\"}]}\n\n",
    );
    let (events, result) = run(sse);
    assert_eq!(result, Ok(()));
    assert_eq!(
        events,
        vec![
            StreamEvent::TextDelta { text: "Hi".into() },
            StreamEvent::Finished {
                stop_reason: StopReason::EndTurn
            },
        ]
    );
}
