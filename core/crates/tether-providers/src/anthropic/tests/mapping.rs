//! Tests for the mapping of SSE events to stream events.

use serde_json::{Value, json};
use tether_types::ToolCallId;

use crate::anthropic::stream::StreamState;
use crate::sse::{SseEvent, SseParser};
use crate::{ProviderError, ResponseUsage, StopReason, StreamEvent};

const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/anthropic-text.sse"
));
const TOOL_USE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/anthropic-tool-use.sse"
));
const THINKING: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/anthropic-thinking.sse"
));
const CACHE_USAGE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/anthropic-cache-usage.sse"
));
const ERROR: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/anthropic-error.sse"
));
const TRUNCATED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/anthropic-truncated.sse"
));

/// Maps a whole fixture. Returns the events sent before the first error, and the outcome.
fn run(fixture: &str) -> (Vec<StreamEvent>, Result<(), ProviderError>) {
    let mut state = StreamState::default();
    let mut events = Vec::new();
    for sse in SseParser::new().push(fixture.as_bytes()) {
        match state.handle(&sse) {
            Ok(mapped) => events.extend(mapped),
            Err(error) => return (events, Err(error)),
        }
    }
    (events, state.finish())
}

/// An SSE event with the given data.
fn sse(data: &str) -> SseEvent {
    SseEvent {
        data: data.to_owned(),
        ..SseEvent::default()
    }
}

/// Maps one event on a fresh state.
fn map_one(data: &Value) -> Result<Vec<StreamEvent>, ProviderError> {
    StreamState::default().handle(&sse(&data.to_string()))
}

fn usage(input: u64, output: u64, read: u64, write: u64) -> StreamEvent {
    StreamEvent::Usage(ResponseUsage {
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: read,
        cache_write_tokens: write,
    })
}

fn text_delta(text: &str) -> StreamEvent {
    StreamEvent::TextDelta {
        text: text.to_owned(),
    }
}

fn finished(stop_reason: StopReason) -> StreamEvent {
    StreamEvent::Finished { stop_reason }
}

#[test]
fn a_text_stream_maps_to_deltas_usage_and_finished() {
    let (events, outcome) = run(TEXT);
    assert_eq!(outcome, Ok(()));
    assert_eq!(
        events,
        vec![
            usage(12, 1, 0, 0),
            text_delta("Hello"),
            text_delta(" world"),
            usage(12, 4, 0, 0),
            finished(StopReason::EndTurn),
        ]
    );
}

#[test]
fn a_tool_call_streams_started_fragments_and_finished() {
    let (events, outcome) = run(TOOL_USE);
    let id = ToolCallId::new("toolu_01A");
    let fragment = |json_fragment: &str| StreamEvent::ToolCallArgsDelta {
        id: id.clone(),
        json_fragment: json_fragment.to_owned(),
    };
    assert_eq!(outcome, Ok(()));
    assert_eq!(
        events,
        vec![
            usage(25, 1, 0, 0),
            text_delta("I will read the file."),
            StreamEvent::ToolCallStarted {
                id: id.clone(),
                name: "read_file".to_owned()
            },
            fragment("{\"pa"),
            fragment("th\": \"src/ma"),
            fragment("in.rs\"}"),
            StreamEvent::ToolCallFinished { id: id.clone() },
            usage(25, 40, 0, 0),
            finished(StopReason::ToolUse),
        ]
    );
}

#[test]
fn tool_call_fragments_join_into_the_full_arguments() {
    let (events, _) = run(TOOL_USE);
    let joined: String = events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::ToolCallArgsDelta { json_fragment, .. } => Some(json_fragment.as_str()),
            _ => None,
        })
        .collect();
    let args: Value = serde_json::from_str(&joined).unwrap();
    assert_eq!(args, json!({ "path": "src/main.rs" }));
}

#[test]
fn thinking_deltas_map_and_the_signature_is_dropped() {
    let (events, outcome) = run(THINKING);
    let thinking = |text: &str| StreamEvent::ThinkingDelta {
        text: text.to_owned(),
    };
    assert_eq!(outcome, Ok(()));
    assert_eq!(
        events,
        vec![
            usage(30, 2, 0, 0),
            thinking("The user wants a greeting."),
            thinking(" Keep it short."),
            text_delta("Hello there."),
            usage(30, 18, 0, 0),
            finished(StopReason::EndTurn),
        ]
    );
}

#[test]
fn cache_counts_are_reported_and_output_tokens_are_cumulative() {
    let (events, outcome) = run(CACHE_USAGE);
    assert_eq!(outcome, Ok(()));
    assert_eq!(
        events,
        vec![
            usage(9, 1, 4200, 1500),
            text_delta("Done."),
            usage(9, 7, 4200, 1500),
            finished(StopReason::EndTurn),
        ]
    );
}

#[test]
fn an_error_event_ends_the_stream_with_a_mapped_error() {
    let (events, outcome) = run(ERROR);
    assert_eq!(
        events,
        vec![usage(15, 1, 0, 0), text_delta("The answer is")]
    );
    assert_eq!(
        outcome,
        Err(ProviderError::Server {
            status: 529,
            message: "Overloaded".to_owned()
        })
    );
}

#[test]
fn a_stream_without_message_stop_is_malformed() {
    let (events, outcome) = run(TRUNCATED);
    assert_eq!(
        events,
        vec![
            usage(11, 1, 0, 0),
            text_delta("Half an ans"),
            text_delta("wer")
        ]
    );
    assert!(matches!(
        outcome,
        Err(ProviderError::MalformedStream { .. })
    ));
}

#[test]
fn stop_reasons_map_to_the_engine_values() {
    let cases = [
        (Some("end_turn"), StopReason::EndTurn),
        (Some("stop_sequence"), StopReason::EndTurn),
        (Some("tool_use"), StopReason::ToolUse),
        (Some("max_tokens"), StopReason::MaxTokens),
        (Some("refusal"), StopReason::ContentFilter),
        (
            Some("pause_turn"),
            StopReason::Other("pause_turn".to_owned()),
        ),
        (None, StopReason::EndTurn),
    ];
    for (reason, expected) in cases {
        let mut state = StreamState::default();
        let delta = json!({ "type": "message_delta", "delta": { "stop_reason": reason } });
        state.handle(&sse(&delta.to_string())).unwrap();
        let stop = json!({ "type": "message_stop" });
        let events = state.handle(&sse(&stop.to_string())).unwrap();
        assert_eq!(events, vec![finished(expected)]);
    }
}

#[test]
fn error_events_map_by_their_type() {
    let cases = [
        ("overloaded_error", 529),
        ("api_error", 500),
        ("something_new", 500),
    ];
    for (kind, status) in cases {
        let data = json!({ "type": "error", "error": { "type": kind, "message": "m" } });
        let error = map_one(&data).unwrap_err();
        let expected = ProviderError::Server {
            status,
            message: "m".to_owned(),
        };
        assert_eq!(error, expected);
    }
    let limited =
        json!({ "type": "error", "error": { "type": "rate_limit_error", "message": "m" } });
    assert_eq!(
        map_one(&limited).unwrap_err(),
        ProviderError::RateLimited {
            message: "m".to_owned(),
            retry_after_secs: None
        }
    );
}

#[test]
fn auth_and_request_error_events_map_to_their_variants() {
    let event = |kind: &str| json!({ "type": "error", "error": { "type": kind, "message": "m" } });
    let auth = ProviderError::Auth {
        message: "m".to_owned(),
    };
    let invalid = ProviderError::InvalidRequest {
        message: "m".to_owned(),
    };
    assert_eq!(map_one(&event("authentication_error")).unwrap_err(), auth);
    assert_eq!(map_one(&event("permission_error")).unwrap_err(), auth);
    assert_eq!(
        map_one(&event("invalid_request_error")).unwrap_err(),
        invalid
    );
    assert_eq!(map_one(&event("not_found_error")).unwrap_err(), invalid);
}

#[test]
fn data_that_is_not_json_is_malformed() {
    let error = StreamState::default().handle(&sse("not json")).unwrap_err();
    assert!(matches!(error, ProviderError::MalformedStream { .. }));
}

#[test]
fn a_known_event_missing_a_needed_field_is_malformed() {
    let data =
        json!({ "type": "content_block_delta", "delta": { "type": "text_delta", "text": "x" } });
    let error = map_one(&data).unwrap_err();
    assert!(matches!(error, ProviderError::MalformedStream { .. }));
}

#[test]
fn unknown_events_blocks_and_deltas_are_ignored() {
    let unknown_event = json!({ "type": "future_event", "payload": 1 });
    let unknown_block = json!({
        "type": "content_block_start",
        "index": 0,
        "content_block": { "type": "server_tool_use", "id": "srv_1", "name": "search" }
    });
    let unknown_delta = json!({
        "type": "content_block_delta",
        "index": 0,
        "delta": { "type": "citations_delta", "citation": {} }
    });
    assert_eq!(map_one(&unknown_event), Ok(Vec::new()));
    assert_eq!(map_one(&unknown_block), Ok(Vec::new()));
    assert_eq!(map_one(&unknown_delta), Ok(Vec::new()));
}

#[test]
fn a_message_delta_can_replace_input_and_cache_counts() {
    let mut state = StreamState::default();
    let start = json!({
        "type": "message_start",
        "message": { "usage": { "input_tokens": 5, "cache_read_input_tokens": 100, "output_tokens": 1 } }
    });
    let delta = json!({
        "type": "message_delta",
        "delta": {},
        "usage": { "input_tokens": 6, "cache_creation_input_tokens": 50, "output_tokens": 9 }
    });
    state.handle(&sse(&start.to_string())).unwrap();
    let events = state.handle(&sse(&delta.to_string())).unwrap();
    assert_eq!(events, vec![usage(6, 9, 100, 50)]);
}

#[test]
fn events_after_message_stop_are_ignored() {
    let mut state = StreamState::default();
    let stop = sse(&json!({ "type": "message_stop" }).to_string());
    assert_eq!(state.handle(&stop).unwrap().len(), 1);
    assert_eq!(state.handle(&stop), Ok(Vec::new()));
    assert!(state.is_finished());
}
