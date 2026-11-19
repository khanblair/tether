//! Builds Gemini generate-content request bodies.

use serde_json::{Map, Value, json};

use super::messages::to_contents;
use crate::{ChatRequest, ProviderError, ReasoningEffort, ToolSpec};

/// Builds a streaming Gemini request.
pub(super) fn build_body(request: &ChatRequest) -> Result<Value, ProviderError> {
    let mut body = Map::new();
    if !request.system.is_empty() {
        body.insert(
            "systemInstruction".to_owned(),
            json!({"parts":[{"text":request.system}]}),
        );
    }
    body.insert(
        "contents".to_owned(),
        Value::Array(to_contents(&request.messages)?),
    );
    body.insert(
        "generationConfig".to_owned(),
        generation_config(request.max_output_tokens, request.reasoning),
    );
    if !request.tools.is_empty() {
        body.insert("tools".to_owned(), tools_json(&request.tools)?);
    }
    Ok(Value::Object(body))
}

fn generation_config(max_output_tokens: u32, reasoning: Option<ReasoningEffort>) -> Value {
    let mut config = Map::new();
    config.insert("maxOutputTokens".to_owned(), json!(max_output_tokens));
    if let Some(effort) = reasoning {
        config.insert(
            "thinkingConfig".to_owned(),
            json!({"includeThoughts":true,"thinkingBudget":thinking_budget(effort)}),
        );
    }
    Value::Object(config)
}

fn thinking_budget(effort: ReasoningEffort) -> u32 {
    match effort {
        ReasoningEffort::Low => 1024,
        ReasoningEffort::Medium => 8192,
        ReasoningEffort::High => 24576,
    }
}

fn tools_json(tools: &[ToolSpec]) -> Result<Value, ProviderError> {
    let declarations = tools
        .iter()
        .map(|tool| {
            let parameters: Value =
                serde_json::from_str(&tool.input_schema_json).map_err(|_| {
                    ProviderError::InvalidRequest {
                        message: format!(
                            "the input schema of tool {} is not valid JSON",
                            tool.name
                        ),
                    }
                })?;
            Ok(json!({
                "name": tool.name,
                "description": tool.description,
                "parameters": parameters,
            }))
        })
        .collect::<Result<Vec<_>, ProviderError>>()?;
    Ok(json!([{"functionDeclarations": declarations}]))
}
