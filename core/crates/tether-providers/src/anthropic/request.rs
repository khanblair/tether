//! Builds the JSON body of a Messages API request.

use serde_json::{Map, Value, json};

use super::messages::to_api_messages;
use crate::{ChatRequest, ProviderError, ReasoningEffort, ToolSpec};

/// Anthropic needs a thinking budget of at least this many tokens.
const MIN_THINKING_BUDGET: u32 = 1024;

/// Builds the request body.
///
/// The system block and the last tool carry a cache breakpoint, so the stable prefix of the
/// request is cached. That is two of the four breakpoints Anthropic allows.
pub(super) fn build_body(request: &ChatRequest) -> Result<Value, ProviderError> {
    let mut body = Map::new();
    body.insert("model".to_owned(), json!(request.model_id));
    body.insert("max_tokens".to_owned(), json!(request.max_output_tokens));
    body.insert("stream".to_owned(), Value::Bool(true));
    if !request.system.is_empty() {
        let block = json!({ "type": "text", "text": request.system, "cache_control": ephemeral() });
        body.insert("system".to_owned(), Value::Array(vec![block]));
    }
    if !request.tools.is_empty() {
        body.insert("tools".to_owned(), tools_json(&request.tools)?);
    }
    let messages = to_api_messages(&request.messages)?;
    // Anthropic wants the signed thinking blocks of the last assistant turn back during a tool
    // loop, and Tether does not store the signatures yet. Until it does, a request that answers
    // tool calls runs without thinking, which Anthropic accepts.
    let thinking = if ends_with_tool_results(&messages) {
        None
    } else {
        thinking_budget(request.reasoning, request.max_output_tokens)
    };
    body.insert("messages".to_owned(), Value::Array(messages));
    if let Some(budget_tokens) = thinking {
        let thinking = json!({ "type": "enabled", "budget_tokens": budget_tokens });
        body.insert("thinking".to_owned(), thinking);
    }
    Ok(Value::Object(body))
}

/// True when the last message is a user message that answers tool calls.
fn ends_with_tool_results(messages: &[Value]) -> bool {
    let Some(last) = messages.last() else {
        return false;
    };
    let blocks = last["content"].as_array().map_or(&[][..], Vec::as_slice);
    last["role"] == "user" && blocks.iter().any(|block| block["type"] == "tool_result")
}

/// The cache control marker for a breakpoint.
fn ephemeral() -> Value {
    json!({ "type": "ephemeral" })
}

/// Maps the tools. Only the last one carries a cache breakpoint, which covers all of them.
fn tools_json(tools: &[ToolSpec]) -> Result<Value, ProviderError> {
    let mut mapped = Vec::with_capacity(tools.len());
    for tool in tools {
        mapped.push(tool_json(tool)?);
    }
    if let Some(last) = mapped.last_mut() {
        last.insert("cache_control".to_owned(), ephemeral());
    }
    Ok(Value::Array(
        mapped.into_iter().map(Value::Object).collect(),
    ))
}

/// Maps one tool. The schema arrives as text and must parse.
fn tool_json(tool: &ToolSpec) -> Result<Map<String, Value>, ProviderError> {
    let schema: Value = serde_json::from_str(&tool.input_schema_json).map_err(|_| {
        ProviderError::InvalidRequest {
            message: format!("the input schema of tool {} is not valid JSON", tool.name),
        }
    })?;
    let mut mapped = Map::new();
    mapped.insert("name".to_owned(), json!(tool.name));
    mapped.insert("description".to_owned(), json!(tool.description));
    mapped.insert("input_schema".to_owned(), schema);
    Ok(mapped)
}

/// The thinking budget for an effort level, or `None` when thinking must be left off.
///
/// Anthropic needs `max_tokens` above the budget and a budget of at least 1024 tokens. A
/// budget that does not fit is lowered, and dropped when it would fall under the minimum.
fn thinking_budget(effort: Option<ReasoningEffort>, max_tokens: u32) -> Option<u32> {
    let wanted = match effort? {
        ReasoningEffort::Low => 2048,
        ReasoningEffort::Medium => 8192,
        ReasoningEffort::High => 16384,
    };
    let budget = wanted.min(max_tokens.saturating_sub(1));
    (budget >= MIN_THINKING_BUDGET).then_some(budget)
}
