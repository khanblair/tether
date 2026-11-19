//! OpenAI request headers and transport invocation.

use reqwest::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::OpenAiConfig;
use super::request::build_body_with;
use super::stream::StreamState;
use super::wire::{ErrorBody, HttpError};
use crate::transport::{Call, ErrorStyle, run, secret_header};
use crate::{ChatRequest, ProviderError, StreamEvent};

pub(super) struct OpenAiErrors;

/// True when OpenAI refuses the request because of the caller's country or region.
fn is_region_block(error: &HttpError) -> bool {
    error.code.as_deref() == Some("unsupported_country_region_territory")
        || error
            .message
            .to_ascii_lowercase()
            .contains("country, region, or territory")
}

impl ErrorStyle for OpenAiErrors {
    fn message(&self, body: &[u8]) -> Option<String> {
        serde_json::from_slice::<ErrorBody>(body)
            .ok()
            .map(|parsed| parsed.error.message)
            .filter(|message| !message.is_empty())
    }

    fn classify(&self, status: reqwest::StatusCode, body: &[u8]) -> Option<ProviderError> {
        let parsed = serde_json::from_slice::<ErrorBody>(body).ok()?;
        let error = parsed.error;
        if status == reqwest::StatusCode::FORBIDDEN && is_region_block(&error) {
            return Some(ProviderError::Blocked {
                message: error.message,
            });
        }
        let quota = error.code.as_deref() == Some("insufficient_quota");
        (status == reqwest::StatusCode::TOO_MANY_REQUESTS && quota).then_some(ProviderError::Auth {
            message: error.message,
        })
    }
}

/// Sends one request and maps its response stream.
pub(super) async fn stream_chat(
    client: &Client,
    config: &OpenAiConfig,
    request: &ChatRequest,
    events: mpsc::Sender<StreamEvent>,
    cancel: CancellationToken,
) -> Result<(), ProviderError> {
    let body = build_body_with(request, config.reasoning_summary)?;
    let mut headers = HeaderMap::new();
    let value = format!("Bearer {}", config.api_key);
    headers.insert(AUTHORIZATION, secret_header(&value)?);
    let call = Call {
        client,
        url: format!("{}/v1/responses", config.base_url.trim_end_matches('/')),
        headers,
        body: &body,
        errors: &OpenAiErrors,
    };
    run(call, StreamState::default(), events, cancel).await
}
