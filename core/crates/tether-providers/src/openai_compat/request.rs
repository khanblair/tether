//! Builds OpenAI-compatible Chat Completions request bodies.

use serde_json::{Map, Value, json};

use super::messages::to_messages;
use crate::{ChatRequest, ProviderError, ReasoningEffort, ToolSpec};

pub(super) fn build_body(
    request: &ChatRequest,
    dialect: super::Dialect,
) -> Result<Value, ProviderError> {
    let mut body = Map::new();
    body.insert("model".to_owned(), json!(request.model_id));
    body.insert("stream".to_owned(), Value::Bool(true));
    body.insert("stream_options".to_owned(), json!({"include_usage": true}));
    body.insert("max_tokens".to_owned(), json!(request.max_output_tokens));
    body.insert(
        "messages".to_owned(),
        Value::Array(to_messages(request, dialect)?),
    );
    if !request.tools.is_empty() {
        body.insert("tools".to_owned(), tools_json(&request.tools)?);
    }
    if let Some(effort) = request.reasoning {
        match dialect {
            super::Dialect::Plain | super::Dialect::DeepSeek => {
                body.insert("reasoning_effort".to_owned(), json!(effort_name(effort)));
            }
            super::Dialect::OpenRouter => {
                body.insert(
                    "reasoning".to_owned(),
                    json!({"effort": effort_name(effort)}),
                );
            }
        }
    }
    if dialect == super::Dialect::OpenRouter {
        body.insert("usage".to_owned(), json!({"include": true}));
    }
    Ok(Value::Object(body))
}

fn effort_name(effort: ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::High => "high",
    }
}

fn tools_json(tools: &[ToolSpec]) -> Result<Value, ProviderError> {
    tools
        .iter()
        .map(tool_json)
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}

fn tool_json(tool: &ToolSpec) -> Result<Value, ProviderError> {
    let parameters: Value = serde_json::from_str(&tool.input_schema_json).map_err(|_| {
        ProviderError::InvalidRequest {
            message: format!("the input schema of tool {} is not valid JSON", tool.name),
        }
    })?;
    Ok(json!({
        "type": "function",
        "function": {"name": tool.name, "description": tool.description, "parameters": parameters},
    }))
}
