//! Maps Tether messages to Chat Completions messages.

use serde_json::{Value, json};
use tether_types::{Message, MessagePart, Role};

use crate::ProviderError;

const SUMMARY_HEADER: &str = "[Summary of the earlier conversation]\n";

pub(super) fn to_messages(request: &crate::ChatRequest) -> Result<Vec<Value>, ProviderError> {
    let mut messages = Vec::new();
    if !request.system.is_empty() {
        messages.push(json!({"role": "system", "content": request.system}));
    }
    for message in request.messages.iter().filter(|message| !message.rewound) {
        if message.role == Role::SystemNote {
            continue;
        }
        messages.extend(message_values(message)?);
    }
    Ok(messages)
}

fn message_values(message: &Message) -> Result<Vec<Value>, ProviderError> {
    let mut parts = collect_parts(&message.parts)?;
    // Every tool result is its own `tool` message, ahead of any text the message also holds.
    let mut values = std::mem::take(&mut parts.tool_results);
    values.extend(text_and_calls_message(message.role, parts));
    Ok(values)
}

fn text_and_calls_message(role: Role, parts: Parts) -> Option<Value> {
    let role = if parts.has_summary {
        "user"
    } else {
        match role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool | Role::SystemNote => return None,
        }
    };
    if parts.text.is_empty() && parts.tool_calls.is_empty() {
        return None;
    }
    let content = if parts.text.is_empty() {
        Value::Null
    } else {
        Value::String(parts.text)
    };
    let mut value = json!({"role": role, "content": content});
    if !parts.tool_calls.is_empty() {
        value["tool_calls"] = Value::Array(parts.tool_calls);
    }
    Some(value)
}

struct Parts {
    text: String,
    tool_calls: Vec<Value>,
    tool_results: Vec<Value>,
    has_summary: bool,
}

fn collect_parts(parts: &[MessagePart]) -> Result<Parts, ProviderError> {
    let mut values = Parts {
        text: String::new(),
        tool_calls: Vec::new(),
        tool_results: Vec::new(),
        has_summary: false,
    };
    for part in parts {
        collect_part(part, &mut values)?;
    }
    Ok(values)
}

fn collect_part(part: &MessagePart, values: &mut Parts) -> Result<(), ProviderError> {
    match part {
        MessagePart::Text { text } if !text.trim().is_empty() => values.text.push_str(text),
        MessagePart::ToolCall {
            id,
            tool,
            args_json,
        } => {
            let _: Value =
                serde_json::from_str(args_json).map_err(|_| ProviderError::InvalidRequest {
                    message: format!(
                        "the arguments of tool call {id} for {tool} are not valid JSON"
                    ),
                })?;
            values.tool_calls.push(json!({"id": id.as_str(), "type": "function", "function": {"name": tool, "arguments": args_json}}));
        }
        MessagePart::ToolResult {
            tool_call_id, text, ..
        } => {
            values.tool_results.push(
                json!({"role": "tool", "tool_call_id": tool_call_id.as_str(), "content": text}),
            );
        }
        MessagePart::CompactionSummary { text } => {
            values.has_summary = true;
            values.text.push_str(SUMMARY_HEADER);
            values.text.push_str(text);
        }
        MessagePart::ImageRef { .. } => {
            return Err(ProviderError::InvalidRequest {
                message: "images are not supported by the OpenAI-compatible adapter yet".to_owned(),
            });
        }
        MessagePart::Thinking { .. } | MessagePart::Text { .. } => {}
    }
    Ok(())
}
