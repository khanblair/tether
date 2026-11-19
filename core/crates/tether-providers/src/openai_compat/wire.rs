//! Wire types for OpenAI-compatible streams and errors.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct Chunk {
    #[serde(default)]
    pub(super) choices: Vec<Choice>,
    #[serde(default)]
    pub(super) usage: Option<Usage>,
    #[serde(default)]
    pub(super) error: Option<WireError>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Choice {
    #[serde(default)]
    pub(super) delta: Delta,
    #[serde(default)]
    pub(super) finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Delta {
    #[serde(default)]
    pub(super) content: Option<String>,
    #[serde(default)]
    pub(super) reasoning_content: Option<String>,
    #[serde(default)]
    pub(super) reasoning: Option<String>,
    #[serde(default)]
    pub(super) tool_calls: Vec<ToolCallDelta>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct ToolCallDelta {
    #[serde(default)]
    pub(super) index: u64,
    #[serde(default)]
    pub(super) id: Option<String>,
    #[serde(default)]
    pub(super) function: FunctionDelta,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct FunctionDelta {
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) arguments: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Usage {
    #[serde(default)]
    pub(super) prompt_tokens: u64,
    #[serde(default)]
    pub(super) completion_tokens: u64,
    #[serde(default)]
    pub(super) prompt_tokens_details: TokenDetails,
    #[serde(default)]
    pub(super) prompt_cache_hit_tokens: u64,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct TokenDetails {
    #[serde(default)]
    pub(super) cached_tokens: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct ErrorBody {
    pub(super) error: WireError,
}

#[derive(Debug, Deserialize)]
pub(super) struct WireError {
    pub(super) message: String,
    #[serde(default)]
    pub(super) code: Option<ErrorCode>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum ErrorCode {
    Number(u16),
    Text(String),
}

impl ErrorCode {
    pub(super) fn number(&self) -> Option<u16> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Text(value) => value.parse().ok(),
        }
    }
}
