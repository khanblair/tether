//! Wire types for Gemini streaming and HTTP errors.

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub(super) struct Response {
    #[serde(default)]
    pub(super) candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    pub(super) usage: Option<Usage>,
    #[serde(rename = "promptFeedback")]
    pub(super) prompt_feedback: Option<PromptFeedback>,
    pub(super) error: Option<WireError>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Candidate {
    pub(super) content: Option<Content>,
    #[serde(rename = "finishReason")]
    pub(super) finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Content {
    #[serde(default)]
    pub(super) parts: Vec<Part>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Part {
    pub(super) text: Option<String>,
    pub(super) thought: Option<bool>,
    #[serde(rename = "functionCall")]
    pub(super) function_call: Option<FunctionCall>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct FunctionCall {
    pub(super) name: Option<String>,
    pub(super) args: Option<serde_json::Value>,
    pub(super) id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Usage {
    #[serde(default)]
    #[serde(rename = "promptTokenCount")]
    pub(super) prompt: u64,
    #[serde(default)]
    #[serde(rename = "candidatesTokenCount")]
    pub(super) candidates: u64,
    #[serde(default)]
    #[serde(rename = "cachedContentTokenCount")]
    pub(super) cached: u64,
    #[serde(default)]
    #[serde(rename = "thoughtsTokenCount")]
    pub(super) thoughts: u64,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct PromptFeedback {
    #[serde(rename = "blockReason")]
    pub(super) block_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ErrorBody {
    pub(super) error: HttpError,
}

#[derive(Debug, Deserialize)]
pub(super) struct HttpError {
    pub(super) message: String,
    #[serde(default)]
    pub(super) details: Vec<ErrorDetail>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ErrorDetail {
    pub(super) reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WireError {
    pub(super) message: Option<String>,
    pub(super) code: Option<u16>,
    pub(super) status: Option<String>,
}
