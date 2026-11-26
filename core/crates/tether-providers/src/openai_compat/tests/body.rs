//! Request body mapping tests.

use serde_json::{Value, json};
use tether_types::{MessagePart, Role, ToolCallId};

use super::{message, request, text};
use crate::openai_compat::request::build_body;
use crate::{Dialect, ProviderError, ReasoningEffort, ToolSpec};

fn tool() -> ToolSpec {
    ToolSpec {
        name: "read_file".to_owned(),
        description: "Reads a file.".to_owned(),
        input_schema_json: r#"{"type":"object","properties":{"path":{"type":"string"}}}"#
            .to_owned(),
    }
}

fn body(request: &crate::ChatRequest, dialect: Dialect) -> Value {
    build_body(request, dialect).unwrap()
}

#[test]
fn text_body_is_exact() {
    assert_eq!(
        body(&request(), Dialect::Plain),
        json!({"model":"test-model","stream":true,"stream_options":{"include_usage":true},"max_tokens":64,"messages":[{"role":"user","content":"Hello"}]})
    );
}

#[test]
fn system_and_tools_are_exact() {
    let mut chat = request();
    chat.system = "Be brief.".to_owned();
    chat.tools = vec![tool()];
    assert_eq!(
        body(&chat, Dialect::Plain),
        json!({"model":"test-model","stream":true,"stream_options":{"include_usage":true},"max_tokens":64,"messages":[{"role":"system","content":"Be brief."},{"role":"user","content":"Hello"}],"tools":[{"type":"function","function":{"name":"read_file","description":"Reads a file.","parameters":{"type":"object","properties":{"path":{"type":"string"}}}}}]})
    );
}

#[test]
fn history_keeps_assistant_tools_and_tool_result() {
    let mut chat = request();
    chat.messages = vec![
        message(
            Role::Assistant,
            vec![
                text("Reading."),
                MessagePart::ToolCall {
                    id: ToolCallId::new("call_1"),
                    tool: "read_file".to_owned(),
                    args_json: r#"{"path":"a.rs"}"#.to_owned(),
                },
            ],
        ),
        message(
            Role::Tool,
            vec![MessagePart::ToolResult {
                tool_call_id: ToolCallId::new("call_1"),
                is_error: true,
                text: "failed".to_owned(),
                result_ref: None,
            }],
        ),
        message(Role::User, vec![text("Try again.")]),
    ];
    assert_eq!(
        body(&chat, Dialect::Plain)["messages"],
        json!([
            {"role":"assistant","content":"Reading.","tool_calls":[{"id":"call_1","type":"function","function":{"name":"read_file","arguments":"{\"path\":\"a.rs\"}"}}]},
            {"role":"tool","tool_call_id":"call_1","content":"failed"},
            {"role":"user","content":"Try again."}
        ])
    );
}

#[test]
fn deepseek_replays_one_thinking_part_when_tools_are_present() {
    let mut chat = request();
    chat.tools = vec![tool()];
    chat.messages = vec![message(
        Role::Assistant,
        vec![
            MessagePart::Thinking {
                text: "Plan".to_owned(),
            },
            text("Done."),
            call("a"),
        ],
    )];
    assert_eq!(
        body(&chat, Dialect::DeepSeek)["messages"],
        json!([{"role":"assistant","content":"Done.","reasoning_content":"Plan","tool_calls":[{"id":"a","type":"function","function":{"name":"read_file","arguments":"{}"}}]}])
    );
}

#[test]
fn deepseek_joins_several_thinking_parts_in_order() {
    let mut chat = request();
    chat.tools = vec![tool()];
    chat.messages = vec![message(
        Role::Assistant,
        vec![
            MessagePart::Thinking {
                text: "A".to_owned(),
            },
            MessagePart::Thinking {
                text: "B".to_owned(),
            },
            call("a"),
        ],
    )];
    assert_eq!(
        body(&chat, Dialect::DeepSeek)["messages"][0]["reasoning_content"],
        "AB"
    );
}

#[test]
fn deepseek_omits_reasoning_when_no_thinking_parts_exist() {
    let mut chat = request();
    chat.tools = vec![tool()];
    chat.messages = vec![message(Role::Assistant, vec![text("Done."), call("a")])];
    assert_eq!(
        body(&chat, Dialect::DeepSeek)["messages"],
        json!([{"role":"assistant","content":"Done.","tool_calls":[{"id":"a","type":"function","function":{"name":"read_file","arguments":"{}"}}]}])
    );
}

#[test]
fn deepseek_replays_reasoning_on_an_assistant_message_without_tool_calls() {
    let mut chat = request();
    chat.tools = vec![tool()];
    chat.messages = vec![message(
        Role::Assistant,
        vec![
            MessagePart::Thinking {
                text: "Plan".to_owned(),
            },
            text("Done."),
        ],
    )];
    assert_eq!(
        body(&chat, Dialect::DeepSeek)["messages"],
        json!([{"role":"assistant","content":"Done.","reasoning_content":"Plan"}])
    );
}

#[test]
fn plain_and_openrouter_drop_assistant_reasoning() {
    let mut chat = request();
    chat.tools = vec![tool()];
    chat.messages = vec![message(
        Role::Assistant,
        vec![
            MessagePart::Thinking {
                text: "Plan".to_owned(),
            },
            text("Done."),
        ],
    )];
    for dialect in [Dialect::Plain, Dialect::OpenRouter] {
        assert_eq!(
            body(&chat, dialect)["messages"],
            json!([{"role":"assistant","content":"Done."}])
        );
    }
}

#[test]
fn dialect_reasoning_fields_are_exact() {
    let mut chat = request();
    chat.reasoning = Some(ReasoningEffort::High);
    assert_eq!(body(&chat, Dialect::Plain)["reasoning_effort"], "high");
    assert_eq!(
        body(&chat, Dialect::OpenRouter)["reasoning"],
        json!({"effort":"high"})
    );
    assert_eq!(body(&chat, Dialect::DeepSeek)["reasoning_effort"], "high");
    assert_eq!(body(&chat, Dialect::DeepSeek).get("reasoning"), None);
    chat.reasoning = Some(ReasoningEffort::Low);
    assert_eq!(body(&chat, Dialect::DeepSeek)["reasoning_effort"], "low");
    chat.reasoning = Some(ReasoningEffort::Medium);
    assert_eq!(body(&chat, Dialect::DeepSeek)["reasoning_effort"], "medium");
    assert_eq!(body(&chat, Dialect::DeepSeek).get("reasoning"), None);
    chat.reasoning = None;
    assert_eq!(body(&chat, Dialect::DeepSeek).get("reasoning_effort"), None);
    assert_eq!(body(&chat, Dialect::DeepSeek).get("reasoning"), None);
    assert_eq!(
        body(&chat, Dialect::OpenRouter)["usage"],
        json!({"include":true})
    );
}

#[test]
fn invalid_inputs_and_skips_are_exact() {
    let mut chat = request();
    let mut rewound = message(Role::User, vec![text("gone")]);
    rewound.rewound = true;
    chat.messages = vec![
        rewound,
        message(Role::SystemNote, vec![text("note")]),
        message(
            Role::User,
            vec![
                text("  "),
                MessagePart::Thinking {
                    text: "hidden".to_owned(),
                },
            ],
        ),
        message(
            Role::User,
            vec![MessagePart::CompactionSummary {
                text: "Earlier.".to_owned(),
            }],
        ),
    ];
    assert_eq!(
        body(&chat, Dialect::Plain)["messages"],
        json!([{"role":"user","content":"[Summary of the earlier conversation]\nEarlier."}])
    );
    let mut image = request();
    image.messages = vec![message(
        Role::User,
        vec![MessagePart::ImageRef {
            blob_hash: "x".to_owned(),
            media_type: "image/png".to_owned(),
        }],
    )];
    assert_eq!(
        build_body(&image, Dialect::Plain),
        Err(ProviderError::InvalidRequest {
            message: "images are not supported by the OpenAI-compatible adapter yet".to_owned()
        })
    );
    let mut bad = tool();
    bad.input_schema_json = "bad".to_owned();
    chat.tools = vec![bad];
    assert_eq!(
        build_body(&chat, Dialect::Plain),
        Err(ProviderError::InvalidRequest {
            message: "the input schema of tool read_file is not valid JSON".to_owned()
        })
    );
}

fn call(id: &str) -> MessagePart {
    MessagePart::ToolCall {
        id: ToolCallId::new(id),
        tool: "read_file".to_owned(),
        args_json: "{}".to_owned(),
    }
}

fn result(id: &str, output: &str) -> MessagePart {
    MessagePart::ToolResult {
        tool_call_id: ToolCallId::new(id),
        is_error: false,
        text: output.to_owned(),
        result_ref: None,
    }
}

#[test]
fn every_tool_result_in_a_message_becomes_its_own_tool_message() {
    let mut chat = request();
    chat.messages = vec![message(
        Role::Tool,
        vec![result("a", "one"), result("b", "two")],
    )];
    assert_eq!(
        body(&chat, Dialect::Plain)["messages"],
        json!([
            {"role":"tool","tool_call_id":"a","content":"one"},
            {"role":"tool","tool_call_id":"b","content":"two"}
        ])
    );
}

#[test]
fn text_in_a_message_with_a_tool_result_is_kept_after_the_result() {
    let mut chat = request();
    chat.messages = vec![message(
        Role::User,
        vec![result("a", "one"), text("Thanks.")],
    )];
    assert_eq!(
        body(&chat, Dialect::Plain)["messages"],
        json!([
            {"role":"tool","tool_call_id":"a","content":"one"},
            {"role":"user","content":"Thanks."}
        ])
    );
}

#[test]
fn an_assistant_message_with_only_tool_calls_has_null_content() {
    let mut chat = request();
    chat.messages = vec![message(Role::Assistant, vec![call("a")])];
    assert_eq!(
        body(&chat, Dialect::Plain)["messages"],
        json!([{"role":"assistant","content":null,"tool_calls":[
            {"id":"a","type":"function","function":{"name":"read_file","arguments":"{}"}}
        ]}])
    );
}

#[test]
fn a_compaction_summary_is_a_user_message_whatever_its_role() {
    let mut chat = request();
    chat.messages = vec![message(
        Role::Assistant,
        vec![MessagePart::CompactionSummary {
            text: "Earlier.".to_owned(),
        }],
    )];
    assert_eq!(
        body(&chat, Dialect::Plain)["messages"],
        json!([{"role":"user","content":"[Summary of the earlier conversation]\nEarlier."}])
    );
}

fn thinking(text: &str) -> MessagePart {
    MessagePart::Thinking {
        text: text.to_owned(),
    }
}

fn deepseek_messages(role: Role, parts: Vec<MessagePart>, with_tools: bool) -> Value {
    let mut chat = request();
    if with_tools {
        chat.tools = vec![tool()];
    }
    chat.messages = vec![message(role, parts)];
    body(&chat, Dialect::DeepSeek)["messages"].clone()
}

#[test]
fn deepseek_reasoning_is_not_replayed_when_the_request_has_no_tools() {
    let sent = deepseek_messages(
        Role::Assistant,
        vec![thinking("Plan."), text("Done.")],
        false,
    );
    assert_eq!(sent, json!([{"role":"assistant","content":"Done."}]));
}

#[test]
fn deepseek_reasoning_is_replayed_on_an_assistant_message_when_the_request_has_tools() {
    let sent = deepseek_messages(
        Role::Assistant,
        vec![thinking("Plan."), text("Done.")],
        true,
    );
    assert_eq!(
        sent,
        json!([{"role":"assistant","content":"Done.","reasoning_content":"Plan."}])
    );
}

#[test]
fn a_thinking_part_in_a_summary_message_is_never_sent_as_reasoning() {
    let summary = MessagePart::CompactionSummary {
        text: "Earlier.".to_owned(),
    };
    let sent = deepseek_messages(Role::Assistant, vec![thinking("Plan."), summary], true);
    assert_eq!(
        sent,
        json!([{"role":"user","content":"[Summary of the earlier conversation]\nEarlier."}])
    );
}

#[test]
fn an_empty_thinking_part_is_not_replayed() {
    let sent = deepseek_messages(Role::Assistant, vec![thinking(""), text("Done.")], true);
    assert_eq!(sent, json!([{"role":"assistant","content":"Done."}]));
}
