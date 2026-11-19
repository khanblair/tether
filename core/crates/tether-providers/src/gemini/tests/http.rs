//! Gemini HTTP contract tests.

use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{KEY, collect, request};
use crate::gemini::http::{GeminiErrors, stream_prefix};
use crate::test_support::{Reply, start};
use crate::transport::ErrorStyle;
use crate::{ChatProvider, GeminiConfig, GeminiProvider, ProviderError, StreamEvent};
use reqwest::StatusCode;

const REGION_BODY: &str = r#"{"error":{"code":400,"message":"User location is not supported for the API use.","status":"FAILED_PRECONDITION"}}"#;
const INVALID_KEY_BODY: &str = r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"API_KEY_INVALID","domain":"googleapis.com"}]}}"#;
const DENIED_BODY: &str = r#"{"error":{"code":403,"message":"The caller does not have permission","status":"PERMISSION_DENIED"}}"#;

const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/gemini-text.sse"
));
const TRUNCATED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/gemini-truncated.sse"
));

fn provider(base_url: String) -> GeminiProvider {
    let config = GeminiConfig {
        base_url,
        ..GeminiConfig::new(KEY)
    };
    GeminiProvider::new(crate::client_builder().no_proxy().build().unwrap(), config)
}

async fn run(reply: Reply) -> (Vec<StreamEvent>, Result<(), ProviderError>) {
    let server = start(reply).await;
    let provider = provider(server.base_url);
    let (sender, receiver) = mpsc::channel(8);
    let chat = request();
    let (result, events) = tokio::join!(
        provider.stream_chat(&chat, sender, CancellationToken::new()),
        collect(receiver)
    );
    (events, result)
}

#[tokio::test]
async fn request_uses_header_and_model_path_without_key() {
    let server = start(Reply::stream(TEXT, 37)).await;
    let provider = provider(server.base_url.clone());
    let (sender, receiver) = mpsc::channel(8);
    let mut chat = request();
    chat.model_id = "models/gemini-test".into();
    let result = provider
        .stream_chat(&chat, sender, CancellationToken::new())
        .await;
    drop(receiver);
    let seen = server.seen.await.unwrap().unwrap();
    let _: Value = serde_json::from_str(&seen.body).unwrap();
    assert_eq!(result, Ok(()));
    assert_eq!(
        seen.request_line,
        "POST /v1beta/models/gemini-test:streamGenerateContent?alt=sse HTTP/1.1"
    );
    assert_eq!(seen.header("x-goog-api-key"), Some(KEY));
    assert_eq!(seen.header("authorization"), None);
    assert!(!seen.request_line.contains(KEY) && !seen.request_line.contains("key="));
}

#[tokio::test]
async fn statuses_and_api_key_error_map() {
    let invalid = r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT"}}"#;
    assert!(matches!(
        run(Reply::json("400 Bad Request", &[], invalid)).await.1,
        Err(ProviderError::Auth { .. })
    ));
    assert!(matches!(
        run(Reply::json("403 Forbidden", &[], "{}")).await.1,
        Err(ProviderError::Auth { .. })
    ));
    assert!(matches!(
        run(Reply::json(
            "429 Too Many Requests",
            &["retry-after: 7"],
            "{}"
        ))
        .await
        .1,
        Err(ProviderError::RateLimited {
            retry_after_secs: Some(7),
            ..
        })
    ));
    assert!(
        matches!(run(Reply::json("400 Bad Request", &[], r#"{"error":{"message":"location is not supported"}}"#)).await.1, Err(ProviderError::Blocked { message }) if message == "location is not supported")
    );
    assert!(matches!(
        run(Reply::json("503 Service Unavailable", &[], "{}"))
            .await
            .1,
        Err(ProviderError::Server { status: 503, .. })
    ));
}

#[tokio::test]
async fn truncated_and_cancelled_streams_stop() {
    assert!(matches!(
        run(Reply::stream(TRUNCATED, 17)).await.1,
        Err(ProviderError::MalformedStream { .. })
    ));
    let server = start(Reply::stream(TRUNCATED, 17).then_hold_open()).await;
    let provider = provider(server.base_url);
    let cancel = CancellationToken::new();
    let (sender, mut receiver) = mpsc::channel(8);
    let chat = request();
    let future = provider.stream_chat(&chat, sender, cancel.clone());
    let cancel_task = async {
        receiver.recv().await;
        cancel.cancel();
    };
    let (result, ()) = tokio::join!(future, cancel_task);
    assert_eq!(result, Err(ProviderError::Cancelled));
}

#[test]
fn debug_output_hides_key() {
    let config = GeminiConfig::new(KEY);
    let provider = GeminiProvider::new(crate::client_builder().build().unwrap(), config.clone());
    assert!(!format!("{config:?} {provider:?}").contains(KEY));
}

#[test]
fn stream_prefix_changes_on_each_call() {
    assert_ne!(stream_prefix(), stream_prefix());
}

#[tokio::test]
async fn a_model_id_that_could_change_the_url_is_rejected_before_any_request() {
    let provider = provider("http://127.0.0.1:9".to_owned());
    let mut chat = request();
    chat.model_id = "gemini/../x?key=1".to_owned();
    let (sender, _receiver) = mpsc::channel(1);
    let result = provider
        .stream_chat(&chat, sender, CancellationToken::new())
        .await;
    assert!(matches!(result, Err(ProviderError::InvalidRequest { .. })));
}

#[test]
fn a_call_prefix_carries_the_time_so_it_differs_after_a_restart() {
    let prefix = crate::gemini::http::stream_prefix();
    let parts: Vec<&str> = prefix.split('_').collect();
    assert_eq!(parts.len(), 3, "{prefix}");
    assert_eq!(parts[0], "gemini");
    let millis: u128 = parts[1].parse().unwrap();
    assert!(millis > 1_700_000_000_000, "{prefix}");
    assert!(parts[2].parse::<u64>().is_ok(), "{prefix}");
}

#[tokio::test]
async fn region_and_key_errors_map_exactly() {
    assert_eq!(
        run(Reply::json("400 Bad Request", &[], REGION_BODY))
            .await
            .1,
        Err(ProviderError::Blocked {
            message: "User location is not supported for the API use.".to_owned()
        })
    );
    assert_eq!(
        run(Reply::json("400 Bad Request", &[], INVALID_KEY_BODY))
            .await
            .1,
        Err(ProviderError::Auth {
            message: "API key not valid. Please pass a valid API key.".to_owned()
        })
    );
    assert_eq!(
        run(Reply::json("403 Forbidden", &[], DENIED_BODY)).await.1,
        Err(ProviderError::Auth {
            message: "The caller does not have permission".to_owned()
        })
    );
}

#[test]
fn classify_splits_region_blocks_from_key_errors() {
    let blocked = Some(ProviderError::Blocked {
        message: "User location is not supported for the API use.".to_owned(),
    });
    assert_eq!(
        GeminiErrors.classify(StatusCode::BAD_REQUEST, REGION_BODY.as_bytes()),
        blocked
    );
    assert_eq!(
        GeminiErrors.classify(StatusCode::FORBIDDEN, REGION_BODY.as_bytes()),
        blocked
    );
    assert!(matches!(
        GeminiErrors.classify(StatusCode::BAD_REQUEST, INVALID_KEY_BODY.as_bytes()),
        Some(ProviderError::Auth { .. })
    ));
    assert_eq!(
        GeminiErrors.classify(StatusCode::FORBIDDEN, DENIED_BODY.as_bytes()),
        None
    );
}
