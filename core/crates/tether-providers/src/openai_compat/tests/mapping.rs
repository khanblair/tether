//! Stream mapping tests.

use tether_types::ToolCallId;

use crate::openai_compat::stream::StreamState;
use crate::sse::SseParser;
use crate::transport::StreamMapper;
use crate::{ProviderError, ResponseUsage, StopReason, StreamEvent};

const ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/"
);

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}{name}")).unwrap()
}

fn run(input: &str) -> (Vec<StreamEvent>, Result<(), ProviderError>) {
    run_with(input, crate::Dialect::Plain)
}

fn run_with(input: &str, dialect: crate::Dialect) -> (Vec<StreamEvent>, Result<(), ProviderError>) {
    let mut state = StreamState::new(dialect);
    let mut events = Vec::new();
    for sse in SseParser::new().push(input.as_bytes()) {
        match state.handle(&sse) {
            Ok(mapped) => events.extend(mapped),
            Err(error) => return (events, Err(error)),
        }
    }
    (events, state.finish())
}

#[test]
fn deepseek_text_is_exact() {
    let (events, result) = run(&fixture("deepseek-text.sse"));
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
fn reasoning_and_usage_are_exact() {
    let (events, result) = run_with(&fixture("deepseek-reasoning.sse"), crate::Dialect::DeepSeek);
    assert_eq!(result, Ok(()));
    assert_eq!(
        events,
        vec![
            StreamEvent::ThinkingDelta {
                text: "Plan".to_owned()
            },
            StreamEvent::TextDelta {
                text: "Answer".to_owned()
            },
            StreamEvent::Usage(ResponseUsage {
                input_tokens: 6,
                output_tokens: 3,
                cache_read_tokens: 2,
                ..ResponseUsage::default()
            }),
            StreamEvent::Finished {
                stop_reason: StopReason::EndTurn
            }
        ]
    );
    let (events, result) = run(&fixture("openrouter-reasoning.sse"));
    assert_eq!(result, Ok(()));
    assert_eq!(
        events,
        vec![
            StreamEvent::ThinkingDelta {
                text: "Think".to_owned()
            },
            StreamEvent::TextDelta {
                text: "Answer".to_owned()
            },
            StreamEvent::Usage(ResponseUsage {
                input_tokens: 8,
                output_tokens: 3,
                ..ResponseUsage::default()
            }),
            StreamEvent::Finished {
                stop_reason: StopReason::EndTurn
            }
        ]
    );
}

#[test]
fn tool_call_and_stop_reason_are_exact() {
    let (events, result) = run(&fixture("deepseek-tool-call.sse"));
    let id = ToolCallId::new("call_ds");
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
                json_fragment: "{\"p".to_owned()
            },
            StreamEvent::ToolCallArgsDelta {
                id: id.clone(),
                json_fragment: "ath\":\"src/".to_owned()
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
    let (events, result) = run(&fixture("deepseek-length.sse"));
    assert_eq!(result, Ok(()));
    assert_eq!(
        events,
        vec![
            StreamEvent::TextDelta {
                text: "cut".to_owned()
            },
            StreamEvent::Finished {
                stop_reason: StopReason::MaxTokens
            }
        ]
    );
}

#[test]
fn comments_cache_error_and_truncation_are_exact() {
    let (events, result) = run(&fixture("openrouter-text.sse"));
    assert_eq!(result, Ok(()));
    assert_eq!(
        events,
        vec![
            StreamEvent::TextDelta {
                text: "Hello".to_owned()
            },
            StreamEvent::Usage(ResponseUsage {
                input_tokens: 9,
                output_tokens: 4,
                cache_read_tokens: 3,
                ..ResponseUsage::default()
            }),
            StreamEvent::Finished {
                stop_reason: StopReason::EndTurn
            }
        ]
    );
    let (events, result) = run(&fixture("openrouter-error.sse"));
    assert_eq!(
        events,
        vec![StreamEvent::TextDelta {
            text: "Partial".to_owned()
        }]
    );
    assert_eq!(
        result,
        Err(ProviderError::Auth {
            message: "credits are required".to_owned()
        })
    );
    let (events, result) = run(&fixture("openai-compat-truncated.sse"));
    assert_eq!(
        events,
        vec![StreamEvent::TextDelta {
            text: "Cut off".to_owned()
        }]
    );
    assert_eq!(
        result,
        Err(ProviderError::MalformedStream {
            message: "the stream ended before [DONE]".to_owned()
        })
    );
}

#[test]
fn two_indexes_finish_in_order_and_empty_fragments_are_ignored() {
    let input = concat!(
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":2,\"id\":\"two\",\"function\":{\"name\":\"b\",\"arguments\":\"\"}}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":1,\"id\":\"one\",\"function\":{\"name\":\"a\",\"arguments\":\"{}\"}}]},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let (events, result) = run(input);
    assert_eq!(result, Ok(()));
    assert_eq!(
        events,
        vec![
            StreamEvent::ToolCallStarted {
                id: ToolCallId::new("two"),
                name: "b".to_owned()
            },
            StreamEvent::ToolCallStarted {
                id: ToolCallId::new("one"),
                name: "a".to_owned()
            },
            StreamEvent::ToolCallArgsDelta {
                id: ToolCallId::new("one"),
                json_fragment: "{}".to_owned()
            },
            StreamEvent::ToolCallFinished {
                id: ToolCallId::new("one")
            },
            StreamEvent::ToolCallFinished {
                id: ToolCallId::new("two")
            },
            StreamEvent::Finished {
                stop_reason: StopReason::ToolUse
            }
        ]
    );
}

#[test]
fn a_call_without_an_id_gets_a_prefix_that_differs_per_response() {
    let sse = concat!(
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,",
        "\"function\":{\"name\":\"read_file\",\"arguments\":\"{}\"}}]}}]}\n\n",
    );
    let started = |input: &str| match run(input).0.first() {
        Some(StreamEvent::ToolCallStarted { id, .. }) => id.clone(),
        other => panic!("unexpected {other:?}"),
    };
    let first = started(sse);
    let second = started(sse);
    assert_ne!(first, second);
    assert!(first.as_str().starts_with("compat_") && first.as_str().ends_with("_0"));
}

#[test]
fn an_error_chunk_with_code_429_is_rate_limited() {
    let sse = "data: {\"error\":{\"code\":429,\"message\":\"slow down\"}}\n\n";
    assert_eq!(
        run(sse).1,
        Err(ProviderError::RateLimited {
            message: "slow down".to_owned(),
            retry_after_secs: None
        })
    );
}

#[test]
fn empty_text_deltas_are_ignored() {
    let sse = concat!(
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\"},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    assert_eq!(
        run(sse).0,
        vec![StreamEvent::Finished {
            stop_reason: StopReason::EndTurn
        }]
    );
}
