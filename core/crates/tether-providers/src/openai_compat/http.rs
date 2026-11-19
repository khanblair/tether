//! OpenAI-compatible headers and transport invocation.

use reqwest::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::OpenAiCompatConfig;
use super::request::build_body;
use super::stream::StreamState;
use super::wire::ErrorBody;
use crate::transport::{Call, ErrorStyle, run, secret_header};
use crate::{ChatRequest, ProviderError, StreamEvent};

pub(super) struct CompatErrors;

impl ErrorStyle for CompatErrors {
    fn message(&self, body: &[u8]) -> Option<String> {
        serde_json::from_slice::<ErrorBody>(body)
            .ok()
            .map(|parsed| parsed.error.message)
            .filter(|message| !message.is_empty())
    }

    fn classify(&self, status: reqwest::StatusCode, body: &[u8]) -> Option<ProviderError> {
        if status != reqwest::StatusCode::PAYMENT_REQUIRED {
            return None;
        }
        let message = serde_json::from_slice::<ErrorBody>(body)
            .ok()
            .map(|parsed| parsed.error.message)
            .filter(|message| !message.is_empty())
            .unwrap_or_else(|| "payment required".to_owned());
        Some(ProviderError::Auth { message })
    }
}

pub(super) async fn stream_chat(
    client: &Client,
    config: &OpenAiCompatConfig,
    request: &ChatRequest,
    events: mpsc::Sender<StreamEvent>,
    cancel: CancellationToken,
) -> Result<(), ProviderError> {
    let body = build_body(request, config.dialect)?;
    let headers = build_headers(config)?;
    let call = Call {
        client,
        url: format!("{}/chat/completions", config.base_url.trim_end_matches('/')),
        headers,
        body: &body,
        errors: &CompatErrors,
    };
    run(call, StreamState::new(config.dialect), events, cancel).await
}

pub(super) fn build_headers(config: &OpenAiCompatConfig) -> Result<HeaderMap, ProviderError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        secret_header(&format!("Bearer {}", config.api_key))?,
    );
    for (name, value) in &config.extra_headers {
        let name = HeaderName::try_from(name).map_err(|_| ProviderError::InvalidRequest {
            message: "an extra header name is invalid".to_owned(),
        })?;
        if name == AUTHORIZATION {
            return Err(ProviderError::InvalidRequest {
                message: "an extra header cannot replace the Authorization header".to_owned(),
            });
        }
        let value = HeaderValue::try_from(value).map_err(|_| ProviderError::InvalidRequest {
            message: "an extra header value is invalid".to_owned(),
        })?;
        headers.insert(name, value);
    }
    Ok(headers)
}
