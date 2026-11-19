//! Request body mapping tests.

use serde_json::{Value, json};
use tether_types::{MessagePart, Role, ToolCallId};

use super::{message, request, text};
use crate::openai::request::build_body;
use crate::{ProviderError, ReasoningEffort, ToolSpec};

fn tool() -> ToolSpec {
    ToolSpec {
        name: "read_file".to_owned(),
        description: "Reads a file.".to_owned(),
        input_schema_json: r#"{"type":"object","properties":{"path":{"type":"string"}}}"#
            .to_owned(),
    }
}

fn body(request: &crate::ChatRequest) -> Value {
    build_body(request).unwrap()
}

#[test]
fn text_request_has_responses_fields() {
    assert_eq!(
        body(&request()),
        json!({
            "model":"gpt-test", "stream":true, "store":false, "max_output_tokens":64,
            "input":[{"role":"user","content":[{"type":"input_text","text":"Hello"}]}]
        })
    );
}

#[test]
fn instructions_and_tools_are_mapped() {
    let mut chat = request();
    chat.system = "Be brief.".to_owned();
    chat.tools = vec![tool()];
    assert_eq!(body(&chat)["instructions"], "Be brief.");
    assert_eq!(
        body(&chat)["tools"][0],
        json!({
        "type":"function", "name":"read_file", "description":"Reads a file.",
            "parameters":{"type":"object","properties":{"path":{"type":"string"}}}
        })
    );
}

#[test]
fn history_is_flat_and_keeps_tool_items_in_order() {
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
        body(&chat)["input"],
        json!([
            {"role":"assistant","content":[{"type":"output_text","text":"Reading."}]},
            {"type":"function_call","call_id":"call_1","name":"read_file","arguments":"{\"path\":\"a.rs\"}"},
            {"type":"function_call_output","call_id":"call_1","output":"failed"},
            {"role":"user","content":[{"type":"input_text","text":"Try again."}]}
        ])
    );
    assert!(body(&chat)["input"][1].get("id").is_none());
}

#[test]
fn reasoning_summary_and_rejections_work() {
    let mut chat = request();
    chat.reasoning = Some(ReasoningEffort::High);
    chat.messages = vec![message(
        Role::User,
        vec![MessagePart::CompactionSummary {
            text: "Earlier.".to_owned(),
        }],
    )];
    assert_eq!(body(&chat)["reasoning"], json!({"effort":"high"}));
    assert_eq!(
        body(&chat)["input"][0]["content"][0]["text"],
        "[Summary of the earlier conversation]\nEarlier."
    );
}

#[test]
fn images_and_bad_json_are_rejected() {
    let mut image = request();
    image.messages = vec![message(
        Role::User,
        vec![MessagePart::ImageRef {
            blob_hash: "x".to_owned(),
            media_type: "image/png".to_owned(),
        }],
    )];
    assert!(matches!(
        build_body(&image),
        Err(ProviderError::InvalidRequest { .. })
    ));
    let mut bad_tool = tool();
    bad_tool.input_schema_json = "bad".to_owned();
    let mut schema = request();
    schema.tools = vec![bad_tool];
    assert!(matches!(
        build_body(&schema),
        Err(ProviderError::InvalidRequest { .. })
    ));
}

#[test]
fn system_notes_rewound_messages_and_whitespace_are_skipped() {
    let mut chat = request();
    let mut rewound = message(Role::User, vec![text("gone")]);
    rewound.rewound = true;
    chat.messages = vec![
        rewound,
        message(Role::SystemNote, vec![text("note")]),
        message(Role::User, vec![text("  ")]),
        message(Role::User, vec![text("kept")]),
    ];
    assert_eq!(
        body(&chat)["input"],
        json!([{"role":"user","content":[{"type":"input_text","text":"kept"}]}])
    );
}

#[test]
fn tool_call_arguments_that_are_not_json_are_rejected() {
    let mut chat = request();
    chat.messages = vec![message(
        Role::Assistant,
        vec![MessagePart::ToolCall {
            id: tether_types::ToolCallId::new("call_1"),
            tool: "read_file".to_owned(),
            args_json: "{nope".to_owned(),
        }],
    )];
    assert!(matches!(
        build_body(&chat),
        Err(ProviderError::InvalidRequest { .. })
    ));
}
