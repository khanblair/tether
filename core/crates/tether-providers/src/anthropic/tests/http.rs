//! Tests for sending requests, mapping failures, and reading the stream, against a local server.

use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use std::time::Duration;

use super::super::http::status_error;
use super::server::{Reply, refused_base_url, start};
use super::{KEY, collect, request};
use crate::{
    AnthropicConfig, AnthropicProvider, ChatProvider, ProviderError, ResponseUsage, StopReason,
    StreamEvent,
};

const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/anthropic-text.sse"
));
const TRUNCATED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/anthropic-truncated.sse"
));

/// The size of the pieces a stream is sent in. It is odd on purpose, so pieces split events
/// and even lines at awkward places.
const PIECE_LEN: usize = 37;

type Outcome = (Vec<StreamEvent>, Result<(), ProviderError>);

fn provider(base_url: String) -> AnthropicProvider {
    let config = AnthropicConfig {
        base_url,
        ..AnthropicConfig::new(KEY)
    };
    let client = crate::client_builder().no_proxy().build().unwrap();
    AnthropicProvider::new(client, config)
}

/// Fails the test when an error shows the key, in its message or its debug text.
fn assert_error_hides_key(error: &ProviderError) {
    let shown = format!("{error} {error:?}");
    assert!(!shown.contains(KEY), "a provider error shows the API key");
}

/// Streams the standard request from `base_url` and returns what arrived.
async fn run(base_url: String) -> Outcome {
    let provider = provider(base_url);
    let chat = request();
    let (sender, receiver) = mpsc::channel(8);
    let stream = provider.stream_chat(&chat, sender, CancellationToken::new());
    let (result, events) = tokio::join!(stream, collect(receiver));
    if let Err(error) = &result {
        assert_error_hides_key(error);
    }
    (events, result)
}

/// Runs against a server that answers with `reply`, and returns the outcome.
async fn run_against(reply: Reply) -> Outcome {
    let server = start(reply).await;
    run(server.base_url).await
}

#[tokio::test]
async fn a_200_stream_yields_the_events_and_ends_with_finished() {
    let (events, result) = run_against(Reply::stream(TEXT, PIECE_LEN)).await;
    assert_eq!(result, Ok(()));
    let delta = |text: &str| StreamEvent::TextDelta {
        text: text.to_owned(),
    };
    let usage = |output_tokens| {
        StreamEvent::Usage(ResponseUsage {
            input_tokens: 12,
            output_tokens,
            ..ResponseUsage::default()
        })
    };
    assert_eq!(
        events,
        vec![
            usage(1),
            delta("Hello"),
            delta(" world"),
            usage(4),
            StreamEvent::Finished {
                stop_reason: StopReason::EndTurn
            },
        ]
    );
}

#[tokio::test]
async fn a_401_maps_to_auth() {
    let body =
        r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
    let (events, result) = run_against(Reply::json("401 Unauthorized", &[], body)).await;
    assert_eq!(events, Vec::<StreamEvent>::new());
    assert_eq!(
        result,
        Err(ProviderError::Auth {
            message: "invalid x-api-key".to_owned()
        })
    );
}

#[tokio::test]
async fn a_429_maps_to_rate_limited_with_the_retry_after_seconds() {
    let body = r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#;
    let reply = Reply::json("429 Too Many Requests", &["retry-after: 7"], body);
    let (_, result) = run_against(reply).await;
    assert_eq!(
        result,
        Err(ProviderError::RateLimited {
            message: "slow down".to_owned(),
            retry_after_secs: Some(7)
        })
    );
}

#[tokio::test]
async fn a_400_carries_the_providers_message() {
    let body = r#"{"type":"error","error":{"type":"invalid_request_error","message":"max_tokens is too large"}}"#;
    let (_, result) = run_against(Reply::json("400 Bad Request", &[], body)).await;
    assert_eq!(
        result,
        Err(ProviderError::InvalidRequest {
            message: "max_tokens is too large".to_owned()
        })
    );
}

#[tokio::test]
async fn a_529_maps_to_a_server_error_with_that_status() {
    let body = r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
    let reply = Reply::json("529 Overloaded", &[], body);
    let (_, result) = run_against(reply).await;
    assert_eq!(
        result,
        Err(ProviderError::Server {
            status: 529,
            message: "Overloaded".to_owned()
        })
    );
}

#[tokio::test]
async fn an_error_without_a_json_body_uses_the_status_reason() {
    let reply = Reply::json("503 Service Unavailable", &[], "<html>down</html>");
    let (_, result) = run_against(reply).await;
    assert_eq!(
        result,
        Err(ProviderError::Server {
            status: 503,
            message: "Service Unavailable".to_owned()
        })
    );
}

#[tokio::test]
async fn a_refused_connection_maps_to_network() {
    let (events, result) = run(refused_base_url().await).await;
    assert_eq!(events, Vec::<StreamEvent>::new());
    assert!(matches!(result, Err(ProviderError::Network { .. })));
}

#[tokio::test]
async fn a_body_that_ends_before_message_stop_is_malformed() {
    let (events, result) = run_against(Reply::stream(TRUNCATED, PIECE_LEN)).await;
    assert_eq!(events.len(), 3);
    assert!(matches!(result, Err(ProviderError::MalformedStream { .. })));
}

#[tokio::test]
async fn cancelling_mid_stream_returns_cancelled_at_once() {
    let server = start(Reply::stream(TRUNCATED, PIECE_LEN).then_hold_open()).await;
    let provider = provider(server.base_url);
    let chat = request();
    let cancel = CancellationToken::new();
    let (sender, mut receiver) = mpsc::channel(8);
    let stream = provider.stream_chat(&chat, sender, cancel.clone());
    let cancel_after_first_event = async {
        receiver.recv().await;
        cancel.cancel();
    };
    let (result, ()) = tokio::join!(stream, cancel_after_first_event);
    assert_eq!(result, Err(ProviderError::Cancelled));
}

#[tokio::test]
async fn a_dropped_receiver_returns_cancelled() {
    let server = start(Reply::stream(TEXT, PIECE_LEN)).await;
    let provider = provider(server.base_url);
    let chat = request();
    let (sender, receiver) = mpsc::channel(8);
    drop(receiver);
    let result = provider
        .stream_chat(&chat, sender, CancellationToken::new())
        .await;
    assert_eq!(result, Err(ProviderError::Cancelled));
}

#[tokio::test]
async fn the_server_sees_the_key_the_version_and_the_json_body() {
    let server = start(Reply::stream(TEXT, PIECE_LEN)).await;
    let (_, result) = run(server.base_url.clone()).await;
    let seen = server.seen.await.unwrap().unwrap();
    let body: Value = serde_json::from_str(&seen.body).unwrap();
    assert_eq!(result, Ok(()));
    assert_eq!(seen.request_line, "POST /v1/messages HTTP/1.1");
    assert!(
        seen.header("x-api-key") == Some(KEY),
        "the x-api-key header did not match the configured key"
    );
    assert_eq!(seen.header("anthropic-version"), Some("2023-06-01"));
    assert_eq!(seen.header("content-type"), Some("application/json"));
    assert_eq!(
        body,
        json!({
            "model": "claude-test",
            "max_tokens": 64,
            "stream": true,
            "messages": [{ "role": "user", "content": [{ "type": "text", "text": "Hello" }] }]
        })
    );
}

#[test]
fn debug_output_hides_the_key() {
    let config = AnthropicConfig::new(KEY);
    let provider = provider(config.base_url.clone());
    let shown = format!("{config:?} {provider:?}");
    assert!(!shown.contains(KEY), "debug output shows the API key");
}

#[tokio::test]
async fn a_403_maps_to_auth_like_a_401() {
    let body = r#"{"type":"error","error":{"type":"permission_error","message":"not allowed"}}"#;
    let (_, result) = run_against(Reply::json("403 Forbidden", &[], body)).await;
    assert_eq!(
        result,
        Err(ProviderError::Auth {
            message: "not allowed".to_owned()
        })
    );
}

#[test]
fn every_status_maps_to_the_error_kind_the_engine_expects() {
    use reqwest::StatusCode;

    let map = |code: u16| status_error(StatusCode::from_u16(code).unwrap(), None, b"");
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

#[tokio::test]
async fn a_connection_that_drops_after_message_stop_is_not_an_error() {
    let (events, result) = run_against(Reply::stream(TEXT, PIECE_LEN).then_abort()).await;
    assert_eq!(result, Ok(()));
    assert!(matches!(events.last(), Some(StreamEvent::Finished { .. })));
}

#[tokio::test]
async fn a_connection_that_drops_before_message_stop_is_an_error() {
    let (_, result) = run_against(Reply::stream(TRUNCATED, PIECE_LEN).then_abort()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn a_redirect_is_not_followed_so_the_key_never_reaches_another_host() {
    let elsewhere = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let location = format!("location: http://{}/", elsewhere.local_addr().unwrap());
    let (events, result) = run_against(Reply::json("302 Found", &[&location], "")).await;
    assert_eq!(events, Vec::<StreamEvent>::new());
    assert!(matches!(
        result,
        Err(ProviderError::Server { status: 302, .. })
    ));
    let visit = tokio::time::timeout(Duration::from_millis(300), elsewhere.accept()).await;
    assert!(visit.is_err(), "the redirect target received a connection");
}
