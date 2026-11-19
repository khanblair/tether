//! Maps Tether messages to Anthropic message blocks.

use serde_json::{Value, json};
use tether_types::{Message, MessagePart, Role, ToolCallId};

use crate::ProviderError;

/// Text put before a compaction summary so the model knows what it reads.
const SUMMARY_HEADER: &str = "[Summary of the earlier conversation]\n";

/// One Anthropic message under construction.
struct ApiMessage {
    /// `user` or `assistant`.
    role: &'static str,
    /// Content blocks in the order they will be sent.
    blocks: Vec<Value>,
}

impl ApiMessage {
    /// Builds the JSON message. Anthropic wants tool results ahead of other user content.
    fn into_json(mut self) -> Value {
        self.blocks
            .sort_by_key(|block| block["type"] != "tool_result");
        json!({ "role": self.role, "content": self.blocks })
    }
}

/// Maps the transcript to Anthropic messages.
///
/// Rewound messages, system notes, and messages with no sendable block are dropped. Neighbors with the same role
/// are merged, because Anthropic expects roles to alternate and tool results travel in a user
/// message.
pub(super) fn to_api_messages(messages: &[Message]) -> Result<Vec<Value>, ProviderError> {
    let mut merged: Vec<ApiMessage> = Vec::new();
    for message in messages.iter().filter(|m| !m.rewound) {
        let Some(role) = api_role(message.role) else {
            continue;
        };
        let blocks = blocks_of(&message.parts)?;
        if !blocks.is_empty() {
            push_merged(&mut merged, role, blocks);
        }
    }
    Ok(merged.into_iter().map(ApiMessage::into_json).collect())
}

/// The Anthropic role for a Tether role, or `None` for a note that is never sent.
fn api_role(role: Role) -> Option<&'static str> {
    match role {
        Role::User | Role::Tool => Some("user"),
        Role::Assistant => Some("assistant"),
        Role::SystemNote => None,
    }
}

/// Adds blocks to the last message when it has the same role, else starts a new message.
fn push_merged(merged: &mut Vec<ApiMessage>, role: &'static str, blocks: Vec<Value>) {
    match merged.last_mut() {
        Some(last) if last.role == role => last.blocks.extend(blocks),
        _ => merged.push(ApiMessage { role, blocks }),
    }
}

/// Maps the parts of one message, in order, skipping parts that are never sent.
fn blocks_of(parts: &[MessagePart]) -> Result<Vec<Value>, ProviderError> {
    let mut blocks = Vec::new();
    for part in parts {
        blocks.extend(block_of(part)?);
    }
    Ok(blocks)
}

/// Maps one part to a content block, or `None` when the part is not sent.
fn block_of(part: &MessagePart) -> Result<Option<Value>, ProviderError> {
    match part {
        // Anthropic rejects a text block with no visible text.
        MessagePart::Text { text } if text.trim().is_empty() => Ok(None),
        MessagePart::Text { text } => Ok(Some(text_block(text))),
        MessagePart::CompactionSummary { text } => {
            Ok(Some(text_block(&format!("{SUMMARY_HEADER}{text}"))))
        }
        MessagePart::ToolCall {
            id,
            tool,
            args_json,
        } => tool_use_block(id, tool, args_json).map(Some),
        MessagePart::ToolResult {
            tool_call_id,
            is_error,
            text,
            ..
        } => Ok(Some(tool_result_block(tool_call_id, *is_error, text))),
        // The signature Anthropic needs to accept replayed thinking is not stored.
        MessagePart::Thinking { .. } => Ok(None),
        MessagePart::ImageRef { .. } => Err(ProviderError::InvalidRequest {
            message: "images are not supported by the Anthropic adapter yet".to_owned(),
        }),
    }
}

/// A plain text block.
fn text_block(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

/// A tool call the model made earlier. The arguments must be valid JSON.
fn tool_use_block(id: &ToolCallId, tool: &str, args_json: &str) -> Result<Value, ProviderError> {
    let input: Value =
        serde_json::from_str(args_json).map_err(|_| ProviderError::InvalidRequest {
            message: format!("the arguments of tool call {id} for {tool} are not valid JSON"),
        })?;
    Ok(json!({ "type": "tool_use", "id": id.as_str(), "name": tool, "input": input }))
}

/// The answer to a tool call. `is_error` is sent only when it is true.
fn tool_result_block(id: &ToolCallId, is_error: bool, text: &str) -> Value {
    let mut block = json!({ "type": "tool_result", "tool_use_id": id.as_str(), "content": text });
    if is_error {
        block["is_error"] = Value::Bool(true);
    }
    block
}
