//! Maps Tether messages to Gemini contents.

use std::collections::HashMap;

use serde_json::{Value, json};
use tether_types::{Message, MessagePart, Role, ToolCallId};

use crate::ProviderError;

const SUMMARY_HEADER: &str = "[Summary of the earlier conversation]\n";

/// Maps live transcript messages and merges adjacent Gemini roles.
pub(super) fn to_contents(messages: &[Message]) -> Result<Vec<Value>, ProviderError> {
    let mut contents: Vec<Value> = Vec::new();
    let mut calls = HashMap::<ToolCallId, String>::new();
    for message in messages {
        if message.rewound || message.role == Role::SystemNote {
            continue;
        }
        if let Some(content) = map_message(message, &mut calls)? {
            push_content(&mut contents, content);
        }
    }
    Ok(contents)
}

fn map_message(
    message: &Message,
    calls: &mut HashMap<ToolCallId, String>,
) -> Result<Option<Value>, ProviderError> {
    let role = match message.role {
        Role::Assistant => "model",
        Role::User | Role::Tool => "user",
        Role::SystemNote => return Ok(None),
    };
    let parts = message
        .parts
        .iter()
        .map(|part| map_part(part, calls))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    (!parts.is_empty())
        .then(|| json!({"role":role,"parts":parts}))
        .map(Ok)
        .transpose()
}

fn push_content(contents: &mut Vec<Value>, content: Value) {
    let role = content["role"].as_str().unwrap_or_default();
    let Some(new_parts) = content["parts"].as_array().cloned() else {
        contents.push(content);
        return;
    };
    let Some(last) = contents.last_mut() else {
        contents.push(content);
        return;
    };
    if last["role"] == role
        && let Some(parts) = last["parts"].as_array_mut()
    {
        parts.extend(new_parts);
        return;
    }
    contents.push(content);
}

fn map_part(
    part: &MessagePart,
    calls: &mut HashMap<ToolCallId, String>,
) -> Result<Option<Value>, ProviderError> {
    match part {
        MessagePart::Text { text } if text.trim().is_empty() => Ok(None),
        MessagePart::Text { text } => Ok(Some(json!({"text":text}))),
        MessagePart::CompactionSummary { text } => {
            Ok(Some(json!({"text":format!("{SUMMARY_HEADER}{text}")})))
        }
        MessagePart::Thinking { .. } => Ok(None),
        MessagePart::ImageRef { .. } => Err(ProviderError::InvalidRequest {
            message: "images are not supported by the Gemini adapter yet".to_owned(),
        }),
        MessagePart::ToolCall {
            id,
            tool,
            args_json,
        } => {
            let args: Value =
                serde_json::from_str(args_json).map_err(|_| invalid_args(id, tool))?;
            if !args.is_object() {
                return Err(invalid_args(id, tool));
            }
            calls.insert(id.clone(), tool.clone());
            Ok(Some(json!({"functionCall":{"name":tool,"args":args}})))
        }
        MessagePart::ToolResult {
            tool_call_id,
            is_error,
            text,
            ..
        } => {
            let Some(name) = calls.get(tool_call_id) else {
                return Err(ProviderError::InvalidRequest {
                    message: format!("tool result refers to unknown call id {tool_call_id}"),
                });
            };
            let response = if *is_error {
                json!({"error":text})
            } else {
                json!({"result":text})
            };
            Ok(Some(
                json!({"functionResponse":{"name":name,"response":response}}),
            ))
        }
    }
}

fn invalid_args(id: &ToolCallId, tool: &str) -> ProviderError {
    ProviderError::InvalidRequest {
        message: format!("the arguments of tool call {id} for {tool} must be a JSON object"),
    }
}
