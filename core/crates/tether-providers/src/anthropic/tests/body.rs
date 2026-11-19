//! Tests for the request body.

use serde_json::{Value, json};
use tether_types::{MessagePart, Role, ToolCallId};

use super::{message, request, text};
use crate::anthropic::request::build_body;
use crate::{ChatRequest, ProviderError, ReasoningEffort, ToolSpec};

fn tool(name: &str) -> ToolSpec {
    ToolSpec {
        name: name.to_owned(),
        description: format!("The {name} tool."),
        input_schema_json: r#"{"type":"object","properties":{"path":{"type":"string"}}}"#
            .to_owned(),
    }
}

fn tool_call(id: &str, args_json: &str) -> MessagePart {
    MessagePart::ToolCall {
        id: ToolCallId::new(id),
        tool: "read_file".to_owned(),
        args_json: args_json.to_owned(),
    }
}

fn tool_result(id: &str, is_error: bool, text: &str) -> MessagePart {
    MessagePart::ToolResult {
        tool_call_id: ToolCallId::new(id),
        is_error,
        text: text.to_owned(),
        result_ref: None,
    }
}

fn body_of(request: &ChatRequest) -> Value {
    build_body(request).unwrap()
}

fn with_reasoning(effort: ReasoningEffort, max_output_tokens: u32) -> Value {
    let mut chat = request();
    chat.reasoning = Some(effort);
    chat.max_output_tokens = max_output_tokens;
    body_of(&chat)
}

#[test]
fn a_text_only_request_has_no_system_tools_or_thinking() {
    assert_eq!(
        body_of(&request()),
        json!({
            "model": "claude-test",
            "max_tokens": 64,
            "stream": true,
            "messages": [{ "role": "user", "content": [{ "type": "text", "text": "Hello" }] }]
        })
    );
}

#[test]
fn only_the_system_block_and_the_last_tool_carry_cache_control() {
    let mut chat = request();
    chat.system = "Be brief.".to_owned();
    chat.tools = vec![tool("read_file"), tool("edit_file")];
    let body = body_of(&chat);
    let schema = json!({ "type": "object", "properties": { "path": { "type": "string" } } });
    assert_eq!(
        body["system"],
        json!([{ "type": "text", "text": "Be brief.", "cache_control": { "type": "ephemeral" } }])
    );
    assert_eq!(
        body["tools"],
        json!([
            { "name": "read_file", "description": "The read_file tool.", "input_schema": schema },
            {
                "name": "edit_file",
                "description": "The edit_file tool.",
                "input_schema": schema,
                "cache_control": { "type": "ephemeral" }
            }
        ])
    );
}

#[test]
fn a_tool_call_a_tool_result_and_a_user_message_merge_with_results_first() {
    let mut chat = request();
    chat.messages = vec![
        message(Role::User, vec![text("Read it.")]),
        message(
            Role::Assistant,
            vec![text("Reading."), tool_call("toolu_1", r#"{"path":"a.rs"}"#)],
        ),
        message(
            Role::Tool,
            vec![tool_result("toolu_1", false, "fn main() {}")],
        ),
        message(Role::User, vec![text("Thanks.")]),
    ];
    assert_eq!(
        body_of(&chat)["messages"],
        json!([
            { "role": "user", "content": [{ "type": "text", "text": "Read it." }] },
            { "role": "assistant", "content": [
                { "type": "text", "text": "Reading." },
                { "type": "tool_use", "id": "toolu_1", "name": "read_file", "input": { "path": "a.rs" } }
            ] },
            { "role": "user", "content": [
                { "type": "tool_result", "tool_use_id": "toolu_1", "content": "fn main() {}" },
                { "type": "text", "text": "Thanks." }
            ] }
        ])
    );
}

#[test]
fn is_error_is_sent_only_when_true() {
    let mut chat = request();
    chat.messages = vec![message(
        Role::Tool,
        vec![
            tool_result("a", true, "failed"),
            tool_result("b", false, "ok"),
        ],
    )];
    assert_eq!(
        body_of(&chat)["messages"][0]["content"],
        json!([
            { "type": "tool_result", "tool_use_id": "a", "content": "failed", "is_error": true },
            { "type": "tool_result", "tool_use_id": "b", "content": "ok" }
        ])
    );
}

#[test]
fn thinking_gets_a_budget_for_each_effort() {
    let budget = |effort| with_reasoning(effort, 32_000)["thinking"].clone();
    let enabled = |tokens: u32| json!({ "type": "enabled", "budget_tokens": tokens });
    assert_eq!(budget(ReasoningEffort::Low), enabled(2048));
    assert_eq!(budget(ReasoningEffort::Medium), enabled(8192));
    assert_eq!(budget(ReasoningEffort::High), enabled(16384));
}

#[test]
fn the_thinking_budget_is_lowered_to_stay_under_max_tokens() {
    let low = with_reasoning(ReasoningEffort::Low, 2048);
    let high = with_reasoning(ReasoningEffort::High, 5000);
    assert_eq!(low["thinking"]["budget_tokens"], 2047);
    assert_eq!(high["thinking"]["budget_tokens"], 4999);
}

#[test]
fn thinking_is_left_out_when_the_budget_would_be_under_the_minimum() {
    assert!(
        with_reasoning(ReasoningEffort::High, 1024)
            .get("thinking")
            .is_none()
    );
    assert!(
        with_reasoning(ReasoningEffort::Low, 0)
            .get("thinking")
            .is_none()
    );
    assert_eq!(
        with_reasoning(ReasoningEffort::High, 1025)["thinking"]["budget_tokens"],
        1024
    );
}

#[test]
fn an_image_part_is_rejected() {
    let mut chat = request();
    chat.messages = vec![message(
        Role::User,
        vec![MessagePart::ImageRef {
            blob_hash: "abc".to_owned(),
            media_type: "image/png".to_owned(),
        }],
    )];
    let error = build_body(&chat).unwrap_err();
    assert!(matches!(error, ProviderError::InvalidRequest { .. }));
}

#[test]
fn tool_call_arguments_that_are_not_json_are_rejected() {
    let mut chat = request();
    chat.messages = vec![message(
        Role::Assistant,
        vec![tool_call("toolu_1", "{oops")],
    )];
    let error = build_body(&chat).unwrap_err();
    assert!(matches!(error, ProviderError::InvalidRequest { .. }));
}

#[test]
fn a_tool_schema_that_is_not_json_is_rejected() {
    let mut chat = request();
    let mut broken = tool("read_file");
    broken.input_schema_json = "not json".to_owned();
    chat.tools = vec![broken];
    let error = build_body(&chat).unwrap_err();
    assert!(matches!(error, ProviderError::InvalidRequest { .. }));
}

#[test]
fn a_system_note_is_skipped() {
    let mut chat = request();
    chat.messages = vec![
        message(Role::User, vec![text("One.")]),
        message(Role::SystemNote, vec![text("Rewound to here.")]),
        message(Role::User, vec![text("Two.")]),
    ];
    assert_eq!(
        body_of(&chat)["messages"],
        json!([{ "role": "user", "content": [
            { "type": "text", "text": "One." },
            { "type": "text", "text": "Two." }
        ] }])
    );
}

#[test]
fn thinking_parts_and_empty_messages_are_dropped() {
    let mut chat = request();
    chat.messages = vec![
        message(Role::User, vec![text("Hi.")]),
        message(
            Role::Assistant,
            vec![MessagePart::Thinking {
                text: "hmm".to_owned(),
            }],
        ),
        message(Role::Assistant, vec![text("  ")]),
    ];
    assert_eq!(
        body_of(&chat)["messages"],
        json!([{ "role": "user", "content": [{ "type": "text", "text": "Hi." }] }])
    );
}

#[test]
fn a_compaction_summary_becomes_a_labeled_text_block() {
    let mut chat = request();
    let summary = MessagePart::CompactionSummary {
        text: "We fixed a bug.".to_owned(),
    };
    chat.messages = vec![message(Role::User, vec![summary])];
    assert_eq!(
        body_of(&chat)["messages"][0]["content"],
        json!([{
            "type": "text",
            "text": "[Summary of the earlier conversation]\nWe fixed a bug."
        }])
    );
}

#[test]
fn tool_results_come_before_text_even_inside_one_message() {
    let mut chat = request();
    chat.messages = vec![message(
        Role::User,
        vec![text("A note."), tool_result("toolu_1", false, "ok")],
    )];
    assert_eq!(
        body_of(&chat)["messages"],
        json!([{ "role": "user", "content": [
            { "type": "tool_result", "tool_use_id": "toolu_1", "content": "ok" },
            { "type": "text", "text": "A note." }
        ] }])
    );
}

#[test]
fn a_rewound_message_is_never_sent() {
    let mut chat = request();
    let mut undone = message(Role::User, vec![text("Undone.")]);
    undone.rewound = true;
    chat.messages.insert(0, undone);
    assert_eq!(
        body_of(&chat)["messages"],
        json!([{ "role": "user", "content": [{ "type": "text", "text": "Hello" }] }])
    );
}

#[test]
fn thinking_is_left_out_when_the_request_answers_tool_calls() {
    let mut chat = request();
    chat.reasoning = Some(ReasoningEffort::High);
    chat.max_output_tokens = 32_000;
    chat.messages = vec![
        message(Role::User, vec![text("Read it.")]),
        message(Role::Assistant, vec![tool_call("toolu_1", "{}")]),
        message(Role::Tool, vec![tool_result("toolu_1", false, "ok")]),
    ];
    assert!(body_of(&chat).get("thinking").is_none());
    chat.messages
        .push(message(Role::Assistant, vec![text("Done.")]));
    chat.messages
        .push(message(Role::User, vec![text("Thanks.")]));
    assert!(body_of(&chat).get("thinking").is_some());
}
