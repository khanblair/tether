//! Maps Tether messages to OpenAI Responses input items.

use serde_json::{Value, json};
use tether_types::{Message, MessagePart, Role, ToolCallId};

use crate::ProviderError;

const SUMMARY_HEADER: &str = "[Summary of the earlier conversation]\n";

/// Maps messages without merging adjacent input items.
pub(super) fn to_api_input(messages: &[Message]) -> Result<Vec<Value>, ProviderError> {
    messages
        .iter()
        .filter(|message| !message.rewound)
        .filter(|message| message.role != Role::SystemNote)
        .try_fold(Vec::new(), |mut input, message| {
            let items = message
                .parts
                .iter()
                .map(|part| input_item(message.role, part))
                .collect::<Result<Vec<_>, _>>()?;
            input.extend(items.into_iter().flatten());
            Ok(input)
        })
}

fn input_item(role: Role, part: &MessagePart) -> Result<Option<Value>, ProviderError> {
    match (role, part) {
        (Role::User, MessagePart::Text { text }) if text.trim().is_empty() => Ok(None),
        (Role::Assistant, MessagePart::Text { text }) if text.trim().is_empty() => Ok(None),
        (Role::User, MessagePart::Text { text }) => {
            Ok(Some(message_item("user", "input_text", text)))
        }
        (Role::Assistant, MessagePart::Text { text }) => {
            Ok(Some(message_item("assistant", "output_text", text)))
        }
        (
            Role::Assistant,
            MessagePart::ToolCall {
                id,
                tool,
                args_json,
            },
        ) => tool_call_item(id, tool, args_json).map(Some),
        (
            Role::Tool,
            MessagePart::ToolResult {
                tool_call_id, text, ..
            },
        ) => Ok(Some(json!({
            "type": "function_call_output",
            "call_id": tool_call_id.as_str(),
            "output": text,
        }))),
        (_, MessagePart::CompactionSummary { text }) => Ok(Some(message_item(
            "user",
            "input_text",
            &format!("{SUMMARY_HEADER}{text}"),
        ))),
        (_, MessagePart::ImageRef { .. }) => Err(ProviderError::InvalidRequest {
            message: "images are not supported by the OpenAI adapter yet".to_owned(),
        }),
        _ => Ok(None),
    }
}

fn message_item(role: &str, content_type: &str, text: &str) -> Value {
    json!({
        "role": role,
        "content": [{ "type": content_type, "text": text }],
    })
}

fn tool_call_item(id: &ToolCallId, name: &str, args_json: &str) -> Result<Value, ProviderError> {
    let _: Value = serde_json::from_str(args_json).map_err(|_| ProviderError::InvalidRequest {
        message: format!("the arguments of tool call {id} for {name} are not valid JSON"),
    })?;
    Ok(json!({
        "type": "function_call",
        "call_id": id.as_str(),
        "name": name,
        "arguments": args_json,
    }))
}
