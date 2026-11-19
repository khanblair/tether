//! The request and stream plumbing every adapter shares: sending, mapping HTTP failures,
//! reading the event stream, and stopping when the caller does.

use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::sse::{SseEvent, SseParser};
use crate::{ProviderError, StreamEvent};

/// The most bytes of an error body that are read. An error message is short.
const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;

/// What an adapter supplies to turn one provider's event stream into [`StreamEvent`] values.
pub(crate) trait StreamMapper {
    /// Maps one SSE event to zero or more stream events.
    fn handle(&mut self, event: &SseEvent) -> Result<Vec<StreamEvent>, ProviderError>;

    /// True once the provider's own end-of-stream marker arrived.
    fn is_finished(&self) -> bool;

    /// Checks the body ended the right way. A body that stops early is cut off.
    fn finish(&self) -> Result<(), ProviderError>;
}

/// How one provider words its HTTP errors.
pub(crate) trait ErrorStyle: Sync {
    /// The provider's message from an error body, when the body has one.
    fn message(&self, body: &[u8]) -> Option<String>;

    /// A kind that replaces the status mapping, for providers that reuse a status code
    /// for several failures. `None` keeps the default mapping. The raw body is provided so
    /// implementations can inspect provider error codes without exposing them in messages.
    fn classify(&self, _status: StatusCode, _body: &[u8]) -> Option<ProviderError> {
        None
    }
}

/// One streaming request.
pub(crate) struct Call<'a> {
    /// The shared HTTP client from [`crate::http_client`].
    pub(crate) client: &'a Client,
    /// The full URL to post to.
    pub(crate) url: String,
    /// The provider's headers, including the key.
    pub(crate) headers: HeaderMap,
    /// The JSON body.
    pub(crate) body: &'a Value,
    /// How the provider words its errors.
    pub(crate) errors: &'a dyn ErrorStyle,
}

/// A header value that holds a secret. It is marked sensitive so it stays out of debug output.
///
/// # Errors
/// Returns an `Auth` error, without echoing the value, when it has characters a header
/// cannot hold.
pub(crate) fn secret_header(value: &str) -> Result<HeaderValue, ProviderError> {
    let mut header = HeaderValue::from_str(value).map_err(|_| ProviderError::Auth {
        message: "the API key has characters that an HTTP header cannot hold".to_owned(),
    })?;
    header.set_sensitive(true);
    Ok(header)
}

/// Runs one request from the first byte sent to the end of the stream.
///
/// # Errors
/// Returns the mapped `ProviderError` for a failed request, a failed or malformed stream, or
/// `Cancelled` when the caller cancels or drops the receiver.
pub(crate) async fn run<M: StreamMapper>(
    call: Call<'_>,
    mapper: M,
    events: mpsc::Sender<StreamEvent>,
    cancel: CancellationToken,
) -> Result<(), ProviderError> {
    let response = tokio::select! {
        biased;
        () = stopped(&events, &cancel) => return Err(ProviderError::Cancelled),
        sent = send(&call) => sent?,
    };
    if !response.status().is_success() {
        return Err(failure(response, call.errors, &events, &cancel).await);
    }
    read_stream(response, mapper, &events, &cancel).await
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

/// Posts the request body.
async fn send(call: &Call<'_>) -> Result<Response, ProviderError> {
    call.client
        .post(&call.url)
        .headers(call.headers.clone())
        .json(call.body)
        .send()
        .await
        .map_err(network_error)
}

/// Maps a transport failure. The URL is stripped from the text.
pub(crate) fn network_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network {
        message: error.without_url().to_string(),
    }
}

/// Reads the error body of a non-2xx response and maps it, unless the caller stops first.
async fn failure(
    response: Response,
    errors: &dyn ErrorStyle,
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
    status_error(status, retry_after, &body, errors)
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
pub(crate) fn status_error(
    status: StatusCode,
    retry_after_secs: Option<u64>,
    body: &[u8],
    errors: &dyn ErrorStyle,
) -> ProviderError {
    let message = errors.message(body).unwrap_or_else(|| {
        status
            .canonical_reason()
            .unwrap_or("unknown status")
            .to_owned()
    });
    if let Some(kind) = errors.classify(status, body) {
        return kind;
    }
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

/// Reads the body until it ends, sending each mapped event to `events`.
async fn read_stream<M: StreamMapper>(
    mut response: Response,
    mut mapper: M,
    events: &mpsc::Sender<StreamEvent>,
    cancel: &CancellationToken,
) -> Result<(), ProviderError> {
    let mut parser = SseParser::new();
    loop {
        let next = tokio::select! {
            biased;
            () = stopped(events, cancel) => return Err(ProviderError::Cancelled),
            next = next_events(&mut response, &mut parser) => next,
        };
        match next {
            Ok(Some(sse_events)) => forward(&mut mapper, &sse_events, events, cancel).await?,
            Ok(None) => return mapper.finish(),
            // The response is complete, so a late read error changes nothing.
            Err(_) if mapper.is_finished() => return Ok(()),
            Err(error) => return Err(error),
        }
        // The final event is forwarded, so the response is complete. Do not wait for the server
        // to close the body: a silent or keep-alive-only connection would hold the call open.
        if mapper.is_finished() {
            return Ok(());
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
async fn forward<M: StreamMapper>(
    mapper: &mut M,
    sse_events: &[SseEvent],
    events: &mpsc::Sender<StreamEvent>,
    cancel: &CancellationToken,
) -> Result<(), ProviderError> {
    for sse in sse_events {
        for event in mapper.handle(sse)? {
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Plain;

    impl ErrorStyle for Plain {
        fn message(&self, _body: &[u8]) -> Option<String> {
            None
        }
    }

    #[test]
    fn every_status_maps_to_the_error_kind_the_engine_expects() {
        let map = |code: u16| status_error(StatusCode::from_u16(code).unwrap(), None, b"", &Plain);
        for code in [401, 403] {
            assert!(matches!(map(code), ProviderError::Auth { .. }), "{code}");
        }
        for code in [400, 404, 413, 422] {
            assert!(
                matches!(map(code), ProviderError::InvalidRequest { .. }),
                "{code}"
            );
        }
        for code in [302, 418, 500, 502, 503, 529] {
            assert!(
                matches!(map(code), ProviderError::Server { status, .. } if status == code),
                "{code}"
            );
        }
        assert!(matches!(map(429), ProviderError::RateLimited { .. }));
    }

    #[test]
    fn a_secret_header_is_sensitive_and_a_bad_one_is_not_echoed() {
        assert!(secret_header("test-key-123").unwrap().is_sensitive());
        let error = secret_header("bad\nkey-value").unwrap_err();
        assert!(!format!("{error:?}").contains("key-value"));
    }
}
