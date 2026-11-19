//! The Anthropic request: URL, headers, and how Anthropic words its errors.

use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::AnthropicConfig;
use super::request::build_body;
use super::stream::StreamState;
use super::wire::ErrorBody;
use crate::transport::{Call, ErrorStyle, run, secret_header};
use crate::{ChatRequest, ProviderError, StreamEvent};

/// The API version header value this adapter is written against.
const API_VERSION: &str = "2023-06-01";

/// Anthropic's error bodies are `{"type":"error","error":{"type":..,"message":..}}`.
pub(super) struct AnthropicErrors;

impl ErrorStyle for AnthropicErrors {
    fn message(&self, body: &[u8]) -> Option<String> {
        serde_json::from_slice::<ErrorBody>(body)
            .ok()
            .map(|parsed| parsed.error.message)
            .filter(|message| !message.is_empty())
    }
}

/// Runs one request from the first byte sent to the end of the stream.
pub(super) async fn stream_chat(
    client: &Client,
    config: &AnthropicConfig,
    request: &ChatRequest,
    events: mpsc::Sender<StreamEvent>,
    cancel: CancellationToken,
) -> Result<(), ProviderError> {
    let body = build_body(request)?;
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", secret_header(&config.api_key)?);
    headers.insert("anthropic-version", HeaderValue::from_static(API_VERSION));
    let call = Call {
        client,
        url: format!("{}/v1/messages", config.base_url.trim_end_matches('/')),
        headers,
        body: &body,
        errors: &AnthropicErrors,
    };
    run(call, StreamState::default(), events, cancel).await
}
