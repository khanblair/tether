//! Gemini request headers, URL, and HTTP error mapping.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;
use reqwest::header::HeaderMap;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::GeminiConfig;
use super::request::build_body;
use super::stream::StreamState;
use super::wire::ErrorBody;
use crate::transport::{Call, ErrorStyle, run, secret_header};
use crate::{ChatRequest, ProviderError, StreamEvent};

struct GeminiErrors;

static STREAM_PREFIX_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) fn stream_prefix() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let counter = STREAM_PREFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("gemini_{millis}_{counter}")
}

impl ErrorStyle for GeminiErrors {
    fn message(&self, body: &[u8]) -> Option<String> {
        serde_json::from_slice::<ErrorBody>(body)
            .ok()
            .map(|parsed| parsed.error.message)
            .filter(|message| !message.is_empty())
    }

    fn classify(&self, status: reqwest::StatusCode, body: &[u8]) -> Option<ProviderError> {
        let parsed = serde_json::from_slice::<ErrorBody>(body).ok()?;
        let error = parsed.error;
        let invalid_key = error
            .details
            .iter()
            .any(|detail| detail.reason.as_deref() == Some("API_KEY_INVALID"))
            || error
                .message
                .to_ascii_lowercase()
                .contains("api key not valid");
        (status == reqwest::StatusCode::BAD_REQUEST && invalid_key).then_some(ProviderError::Auth {
            message: error.message,
        })
    }
}

/// True for a name made only of letters, digits, `.`, `-`, and `_`, so it cannot change the URL path
/// or add a query.
fn is_plain_model_name(model: &str) -> bool {
    !model.is_empty()
        && model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Sends one request and maps its response stream.
pub(super) async fn stream_chat(
    client: &Client,
    config: &GeminiConfig,
    request: &ChatRequest,
    events: mpsc::Sender<StreamEvent>,
    cancel: CancellationToken,
) -> Result<(), ProviderError> {
    let body = build_body(request)?;
    let mut headers = HeaderMap::new();
    headers.insert("x-goog-api-key", secret_header(&config.api_key)?);
    let model = request
        .model_id
        .strip_prefix("models/")
        .unwrap_or(&request.model_id);
    if !is_plain_model_name(model) {
        return Err(ProviderError::InvalidRequest {
            message: "the model id has characters that a Gemini model name cannot have".to_owned(),
        });
    }
    let call = Call {
        client,
        url: format!(
            "{}/v1beta/models/{model}:streamGenerateContent?alt=sse",
            config.base_url.trim_end_matches('/')
        ),
        headers,
        body: &body,
        errors: &GeminiErrors,
    };
    run(call, StreamState::new(stream_prefix()), events, cancel).await
}
