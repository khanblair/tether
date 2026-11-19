//! Builds OpenAI Responses API request bodies.

use serde_json::{Map, Value, json};

use super::messages::to_api_input;
use crate::{ChatRequest, ProviderError, ReasoningEffort, ToolSpec};

/// Builds a request that asks for reasoning summaries, as the default configuration does.
#[cfg(test)]
pub(super) fn build_body(request: &ChatRequest) -> Result<Value, ProviderError> {
    build_body_with(request, true)
}

/// Builds a streaming Responses API request. With `reasoning_summary` set, a reasoning effort
/// also asks for a summary, because OpenAI sends summary deltas only on request.
pub(super) fn build_body_with(
    request: &ChatRequest,
    reasoning_summary: bool,
) -> Result<Value, ProviderError> {
    let mut body = Map::new();
    body.insert("model".to_owned(), json!(request.model_id));
    body.insert("stream".to_owned(), Value::Bool(true));
    body.insert("store".to_owned(), Value::Bool(false));
    body.insert(
        "max_output_tokens".to_owned(),
        json!(request.max_output_tokens),
    );
    if !request.system.is_empty() {
        body.insert("instructions".to_owned(), json!(request.system));
    }
    body.insert(
        "input".to_owned(),
        Value::Array(to_api_input(&request.messages)?),
    );
    if !request.tools.is_empty() {
        body.insert("tools".to_owned(), tools_json(&request.tools)?);
    }
    if let Some(effort) = request.reasoning {
        let mut reasoning = json!({ "effort": effort_name(effort) });
        if reasoning_summary {
            reasoning["summary"] = json!("auto");
        }
        body.insert("reasoning".to_owned(), reasoning);
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
        "name": tool.name,
        "description": tool.description,
        "parameters": parameters,
    }))
}
