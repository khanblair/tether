//! Wire types for OpenAI Responses streaming and HTTP errors.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(super) enum WireEvent {
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded { item: OutputItem },
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta { delta: String },
    #[serde(rename = "response.refusal.delta")]
    RefusalDelta { delta: String },
    #[serde(rename = "response.reasoning_summary_text.delta")]
    ReasoningSummaryDelta { delta: String },
    #[serde(rename = "response.reasoning_text.delta")]
    ReasoningDelta { delta: String },
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionArgsDelta { item_id: String, delta: String },
    #[serde(rename = "response.output_item.done")]
    OutputItemDone { item: OutputItem },
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionArgsDone { item_id: String },
    #[serde(rename = "response.completed")]
    Completed { response: ResponseBody },
    #[serde(rename = "response.incomplete")]
    Incomplete { response: ResponseBody },
    #[serde(rename = "response.failed")]
    Failed { response: FailedResponse },
    #[serde(rename = "error")]
    Error {
        message: String,
        code: Option<String>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub(super) struct OutputItem {
    pub(super) id: Option<String>,
    pub(super) call_id: Option<String>,
    pub(super) name: Option<String>,
    #[serde(rename = "type")]
    pub(super) kind: String,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct ResponseBody {
    pub(super) usage: Option<Usage>,
    pub(super) incomplete_details: Option<IncompleteDetails>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FailedResponse {
    pub(super) error: Option<WireError>,
}

#[derive(Debug, Deserialize)]
pub(super) struct IncompleteDetails {
    pub(super) reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Usage {
    pub(super) input_tokens: u64,
    pub(super) output_tokens: u64,
    #[serde(default)]
    pub(super) input_tokens_details: TokenDetails,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct TokenDetails {
    pub(super) cached_tokens: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct WireError {
    pub(super) message: String,
    pub(super) code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ErrorBody {
    pub(super) error: HttpError,
}

#[derive(Debug, Deserialize)]
pub(super) struct HttpError {
    pub(super) message: String,
    pub(super) code: Option<String>,
}
