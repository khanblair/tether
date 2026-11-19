//! Sends the request, maps failures, and reads the event stream.

use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::AnthropicConfig;
use super::request::build_body;
use super::stream::StreamState;
use super::wire::ErrorBody;
use crate::sse::{SseEvent, SseParser};
use crate::{ChatRequest, ProviderError, StreamEvent};

/// The API version header value this adapter is written against.
const API_VERSION: &str = "2023-06-01";

/// The most bytes of an error body that are read. An error message is short.
const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;

/// Runs one request from the first byte sent to the end of the stream.
pub(super) async fn stream_chat(
    client: &Client,
    config: &AnthropicConfig,
    request: &ChatRequest,
    events: mpsc::Sender<StreamEvent>,
    cancel: CancellationToken,
) -> Result<(), ProviderError> {
    let body = build_body(request)?;
    let response = tokio::select! {
        biased;
        () = stopped(&events, &cancel) => return Err(ProviderError::Cancelled),
        sent = send_request(client, config, &body) => sent?,
    };
    if !response.status().is_success() {
        return Err(failure(response, &events, &cancel).await);
    }
    read_stream(response, &events, &cancel).await
}

/// Resolves when the caller has stopped listening: it cancelled, or dropped the receiver.
///
/// Every wait on the network also waits on this, so a dropped receiver releases the request
/// at once, even when the server is silent or sends only keep-alive pings.
async fn stopped(events: &mpsc::Sender<StreamEvent>, cancel: &CancellationToken) {
    tokio::select! {
        () = cancel.cancelled() => {}
        () = events.closed() => {}
    }
}

/// Posts the request body. The key goes in a sensitive header so it stays out of debug output.
async fn send_request(
    client: &Client,
    config: &AnthropicConfig,
    body: &Value,
) -> Result<Response, ProviderError> {
    let mut key = HeaderValue::from_str(&config.api_key).map_err(|_| ProviderError::Auth {
        message: "the API key has characters that an HTTP header cannot hold".to_owned(),
    })?;
    key.set_sensitive(true);
    let url = format!("{}/v1/messages", config.base_url.trim_end_matches('/'));
    client
        .post(url)
        .header("x-api-key", key)
        .header("anthropic-version", API_VERSION)
        .json(body)
        .send()
        .await
        .map_err(network_error)
}

/// Maps a transport failure. The URL is stripped from the text.
fn network_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network {
        message: error.without_url().to_string(),
    }
}

/// Reads the error body of a non-2xx response and maps it, unless the caller stops first.
async fn failure(
    response: Response,
    events: &mpsc::Sender<StreamEvent>,
    cancel: &CancellationToken,
) -> ProviderError {
    let status = response.status();
    let retry_after = retry_after_secs(response.headers());
    let body = tokio::select! {
        biased;
        () = stopped(events, cancel) => return ProviderError::Cancelled,
        body = read_error_body(response) => body,
    };
    status_error(status, retry_after, &body)
}

/// Reads up to `MAX_ERROR_BODY_BYTES` of the body.
///
/// A read error ends the read and keeps what arrived. The status code already says what went
/// wrong, and the body only adds the provider's wording.
async fn read_error_body(mut response: Response) -> Vec<u8> {
    let mut body = Vec::new();
    while body.len() < MAX_ERROR_BODY_BYTES {
        match response.chunk().await {
            Ok(Some(chunk)) => body.extend_from_slice(&chunk),
            Ok(None) | Err(_) => break,
        }
    }
    body
}

/// Reads `retry-after` as whole seconds. A date, which the header also allows, gives `None`.
fn retry_after_secs(headers: &HeaderMap) -> Option<u64> {
    headers.get(RETRY_AFTER)?.to_str().ok()?.trim().parse().ok()
}

/// Maps an HTTP status and error body to a provider error.
pub(super) fn status_error(
    status: StatusCode,
    retry_after_secs: Option<u64>,
    body: &[u8],
) -> ProviderError {
    let message = error_message(status, body);
    match status.as_u16() {
        401 | 403 => ProviderError::Auth { message },
        429 => ProviderError::RateLimited {
            message,
            retry_after_secs,
        },
        400 | 404 | 413 | 422 => ProviderError::InvalidRequest { message },
        code => ProviderError::Server {
            status: code,
            message,
        },
    }
}

/// The provider's message from the JSON error body, else the reason text of the status.
fn error_message(status: StatusCode, body: &[u8]) -> String {
    serde_json::from_slice::<ErrorBody>(body)
        .map(|parsed| parsed.error.message)
        .ok()
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| {
            status
                .canonical_reason()
                .unwrap_or("unknown status")
                .to_owned()
        })
}

/// Reads the body until it ends, sending each mapped event to `events`.
async fn read_stream(
    mut response: Response,
    events: &mpsc::Sender<StreamEvent>,
    cancel: &CancellationToken,
) -> Result<(), ProviderError> {
    let mut parser = SseParser::new();
    let mut state = StreamState::default();
    loop {
        let next = tokio::select! {
            biased;
            () = stopped(events, cancel) => return Err(ProviderError::Cancelled),
            next = next_events(&mut response, &mut parser) => next,
        };
        match next {
            Ok(Some(sse_events)) => forward(&mut state, &sse_events, events, cancel).await?,
            Ok(None) => return state.finish(),
            // The response is complete, so a late read error changes nothing.
            Err(_) if state.is_finished() => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

/// Reads the next chunk and parses it. `None` means the body ended.
async fn next_events(
    response: &mut Response,
    parser: &mut SseParser,
) -> Result<Option<Vec<SseEvent>>, ProviderError> {
    let chunk = response.chunk().await.map_err(network_error)?;
    Ok(chunk.map(|bytes| parser.push(&bytes)))
}

/// Maps each SSE event and sends the results in order.
async fn forward(
    state: &mut StreamState,
    sse_events: &[SseEvent],
    events: &mpsc::Sender<StreamEvent>,
    cancel: &CancellationToken,
) -> Result<(), ProviderError> {
    for sse in sse_events {
        for event in state.handle(sse)? {
            deliver(events, cancel, event).await?;
        }
    }
    Ok(())
}

/// Sends one event. A dropped receiver or a cancel stops the stream, even when the channel is full.
async fn deliver(
    events: &mpsc::Sender<StreamEvent>,
    cancel: &CancellationToken,
    event: StreamEvent,
) -> Result<(), ProviderError> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(ProviderError::Cancelled),
        sent = events.send(event) => sent.map_err(|_| ProviderError::Cancelled),
    }
}
