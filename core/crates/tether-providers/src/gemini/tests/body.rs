//! Gemini request body mapping tests.

use serde_json::{Value, json};
use tether_types::{MessagePart, Role, ToolCallId};

use super::{message, request, text};
use crate::gemini::request::build_body;
use crate::{ProviderError, ReasoningEffort, ToolSpec};

fn tool() -> ToolSpec {
    ToolSpec {
        name: "read_file".into(),
        description: "Reads a file.".into(),
        input_schema_json: r#"{"type":"object","properties":{"path":{"type":"string"}}}"#.into(),
    }
}

fn schema_tool() -> ToolSpec {
    ToolSpec {
        name: "read_file".into(),
        description: "Reads a file.".into(),
        input_schema_json: r#"{"type":"object","properties":{"options":{"type":"object","properties":{"encoding":{"type":"string"}}}},"additionalProperties":false}"#.into(),
    }
}

fn body(request: &crate::ChatRequest) -> Value {
    build_body(request).unwrap()
}

#[test]
fn text_system_and_tools_use_gemini_fields() {
    let mut chat = request();
    chat.system = "Be brief.".into();
    chat.tools = vec![tool()];
    assert_eq!(
        body(&chat)["contents"],
        json!([{"role":"user","parts":[{"text":"Hello"}]}])
    );
    assert_eq!(
        body(&chat)["systemInstruction"],
        json!({"parts":[{"text":"Be brief."}]})
    );
    assert_eq!(
        body(&chat)["tools"][0]["functionDeclarations"][0]["parametersJsonSchema"],
        serde_json::from_str::<Value>(&tool().input_schema_json).unwrap()
    );
}

#[test]
fn json_schema_is_preserved_under_parameters_json_schema() {
    let mut chat = request();
    chat.tools = vec![schema_tool()];

    assert_eq!(
        body(&chat),
        json!({
            "contents":[{"role":"user","parts":[{"text":"Hello"}]}],
            "generationConfig":{"maxOutputTokens":64},
            "tools":[{"functionDeclarations":[{
                "name":"read_file",
                "description":"Reads a file.",
                "parametersJsonSchema":{
                    "type":"object",
                    "properties":{"options":{"type":"object","properties":{"encoding":{"type":"string"}}}},
                    "additionalProperties":false
                }
            }]}]
        })
    );
}

#[test]
fn history_merges_roles_and_finds_tool_result_name() {
    let mut chat = request();
    chat.messages = vec![
        message(
            Role::Assistant,
            vec![
                text("Reading."),
                MessagePart::ToolCall {
                    id: ToolCallId::new("call_1"),
                    tool: "read_file".into(),
                    args_json: r#"{"path":"a.rs"}"#.into(),
                },
            ],
        ),
        message(
            Role::Tool,
            vec![MessagePart::ToolResult {
                tool_call_id: ToolCallId::new("call_1"),
                is_error: false,
                text: "done".into(),
                result_ref: None,
            }],
        ),
        message(Role::User, vec![text("Try again.")]),
    ];
    assert_eq!(
        body(&chat)["contents"],
        json!([{"role":"model","parts":[{"text":"Reading."},{"functionCall":{"name":"read_file","args":{"path":"a.rs"}}}]},{"role":"user","parts":[{"functionResponse":{"name":"read_file","response":{"result":"done"}}},{"text":"Try again."}]}])
    );
}

#[test]
fn invalid_parts_and_summaries_are_rejected_or_prefixed() {
    let mut image = request();
    image.messages = vec![message(
        Role::User,
        vec![MessagePart::ImageRef {
            blob_hash: "x".into(),
            media_type: "image/png".into(),
        }],
    )];
    assert!(matches!(
        build_body(&image),
        Err(ProviderError::InvalidRequest { .. })
    ));
    let mut bad = request();
    bad.messages = vec![message(
        Role::Assistant,
        vec![MessagePart::ToolCall {
            id: ToolCallId::new("x"),
            tool: "t".into(),
            args_json: "[]".into(),
        }],
    )];
    assert!(matches!(
        build_body(&bad),
        Err(ProviderError::InvalidRequest { .. })
    ));
    let mut malformed = request();
    malformed.messages = vec![message(
        Role::Assistant,
        vec![MessagePart::ToolCall {
            id: ToolCallId::new("x"),
            tool: "t".into(),
            args_json: "bad".into(),
        }],
    )];
    assert!(matches!(
        build_body(&malformed),
        Err(ProviderError::InvalidRequest { .. })
    ));
    let mut summary = request();
    summary.messages = vec![message(
        Role::User,
        vec![MessagePart::CompactionSummary {
            text: "Earlier".into(),
        }],
    )];
    assert_eq!(
        body(&summary)["contents"][0]["parts"][0]["text"],
        "[Summary of the earlier conversation]\nEarlier"
    );
}

#[test]
fn skipped_messages_and_bad_tools_are_rejected() {
    let mut chat = request();
    let mut rewound = message(Role::User, vec![text("gone")]);
    rewound.rewound = true;
    chat.messages = vec![
        rewound,
        message(Role::SystemNote, vec![text("note")]),
        message(Role::User, vec![text(" "), text("kept")]),
    ];
    assert_eq!(
        body(&chat)["contents"],
        json!([{ "role":"user", "parts":[{"text":"kept"}]}])
    );
    let mut unknown = request();
    unknown.messages = vec![message(
        Role::Tool,
        vec![MessagePart::ToolResult {
            tool_call_id: ToolCallId::new("missing"),
            is_error: true,
            text: "bad".into(),
            result_ref: None,
        }],
    )];
    assert!(matches!(
        build_body(&unknown),
        Err(ProviderError::InvalidRequest { .. })
    ));
}

#[test]
fn tool_errors_and_bad_schemas_are_rejected_or_mapped() {
    let mut error_result = request();
    error_result.messages = vec![
        message(
            Role::Assistant,
            vec![MessagePart::ToolCall {
                id: ToolCallId::new("call_1"),
                tool: "read_file".into(),
                args_json: "{}".into(),
            }],
        ),
        message(
            Role::Tool,
            vec![MessagePart::ToolResult {
                tool_call_id: ToolCallId::new("call_1"),
                is_error: true,
                text: "failed".into(),
                result_ref: None,
            }],
        ),
    ];
    assert_eq!(
        body(&error_result)["contents"][1]["parts"][0]["functionResponse"]["response"],
        json!({"error":"failed"})
    );
    let mut schema = request();
    schema.tools = vec![ToolSpec {
        input_schema_json: "bad".into(),
        ..tool()
    }];
    assert!(matches!(
        build_body(&schema),
        Err(ProviderError::InvalidRequest { .. })
    ));
}

#[test]
fn model_prefix_and_reasoning_config_map() {
    let mut chat = request();
    chat.model_id = "models/gemini-test".into();
    chat.reasoning = Some(ReasoningEffort::High);
    assert_eq!(
        body(&chat)["generationConfig"],
        json!({"maxOutputTokens":64,"thinkingConfig":{"includeThoughts":true,"thinkingBudget":24576}})
    );
}

#[test]
fn a_text_only_request_has_no_system_instruction_or_tools() {
    let value = body(&request());
    assert!(value.get("systemInstruction").is_none());
    assert!(value.get("tools").is_none());
    assert_eq!(value["generationConfig"], json!({ "maxOutputTokens": 64 }));
}

#[test]
fn each_reasoning_effort_has_its_own_thinking_budget() {
    let budget = |effort| {
        let mut chat = request();
        chat.reasoning = Some(effort);
        body(&chat)["generationConfig"]["thinkingConfig"]["thinkingBudget"].clone()
    };
    assert_eq!(budget(crate::ReasoningEffort::Low), json!(1024));
    assert_eq!(budget(crate::ReasoningEffort::Medium), json!(8192));
    assert_eq!(budget(crate::ReasoningEffort::High), json!(24576));
}
