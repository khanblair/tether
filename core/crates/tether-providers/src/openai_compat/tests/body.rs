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
fn dialect_reasoning_fields_are_exact() {
    let mut chat = request();
    chat.reasoning = Some(ReasoningEffort::High);
    assert_eq!(body(&chat, Dialect::Plain)["reasoning_effort"], "high");
    assert_eq!(
        body(&chat, Dialect::OpenRouter)["reasoning"],
        json!({"effort":"high"})
    );
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
