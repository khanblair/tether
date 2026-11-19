//! HTTP contract tests for the OpenAI adapter.

use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{KEY, collect, request};
use crate::openai::http::OpenAiErrors;
use crate::test_support::{Reply, start};
use crate::transport::ErrorStyle;
use crate::{ChatProvider, OpenAiConfig, OpenAiProvider, ProviderError, StreamEvent};
use reqwest::StatusCode;

const REGION_MESSAGE: &str = "Country, region, or territory not supported";
const REGION_BODY: &str = r#"{"error":{"message":"Country, region, or territory not supported","type":"request_forbidden","param":null,"code":"unsupported_country_region_territory"}}"#;
const PERMISSION_BODY: &str = r#"{"error":{"message":"insufficient permissions","type":"invalid_request_error","param":null,"code":"insufficient_permissions"}}"#;

const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/openai-text.sse"
));
const TRUNCATED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/openai-truncated.sse"
));
const PIECE_LEN: usize = 37;

fn provider(base_url: String) -> OpenAiProvider {
    let config = OpenAiConfig {
        base_url,
        ..OpenAiConfig::new(KEY)
    };
    let client = crate::client_builder().no_proxy().build().unwrap();
    OpenAiProvider::new(client, config)
}

async fn run(reply: Reply) -> (Vec<StreamEvent>, Result<(), ProviderError>) {
    let server = start(reply).await;
    let provider = provider(server.base_url);
    let chat = request();
    let (sender, receiver) = mpsc::channel(8);
    let (result, events) = tokio::join!(
        provider.stream_chat(&chat, sender, CancellationToken::new()),
        collect(receiver)
    );
    (events, result)
}

#[tokio::test]
async fn a_split_200_stream_finishes() {
    let (events, result) = run(Reply::stream(TEXT, PIECE_LEN)).await;
    assert_eq!(result, Ok(()));
    assert!(matches!(events.last(), Some(StreamEvent::Finished { .. })));
}

#[tokio::test]
async fn statuses_map_and_quota_is_auth() {
    for (status, expected) in [
        ("401 Unauthorized", "auth"),
        ("400 Bad Request", "invalid"),
        ("500 Server Error", "server"),
    ] {
        let body = r#"{"error":{"message":"provider message","type":"x","param":null,"code":"x"}}"#;
        let (_, result) = run(Reply::json(status, &[], body)).await;
        assert!(matches!(
            (expected, result),
            ("auth", Err(ProviderError::Auth { .. }))
                | ("invalid", Err(ProviderError::InvalidRequest { .. }))
                | ("server", Err(ProviderError::Server { .. }))
        ));
    }
    let body =
        r#"{"error":{"message":"slow","type":"x","param":null,"code":"rate_limit_exceeded"}}"#;
    assert!(matches!(
        run(Reply::json(
            "429 Too Many Requests",
            &["retry-after: 7"],
            body
        ))
        .await
        .1,
        Err(ProviderError::RateLimited {
            retry_after_secs: Some(7),
            ..
        })
    ));
    let body =
        r#"{"error":{"message":"quota","type":"x","param":null,"code":"insufficient_quota"}}"#;
    assert!(matches!(
        run(Reply::json("429 Too Many Requests", &[], body)).await.1,
        Err(ProviderError::Auth { .. })
    ));
}

#[tokio::test]
async fn request_uses_bearer_without_x_api_key() {
    let server = start(Reply::stream(TEXT, PIECE_LEN)).await;
    let provider = provider(server.base_url.clone());
    let (sender, receiver) = mpsc::channel(8);
    let result = provider
        .stream_chat(&request(), sender, CancellationToken::new())
        .await;
    drop(receiver);
    let seen = server.seen.await.unwrap().unwrap();
    let _: Value = serde_json::from_str(&seen.body).unwrap();
    assert_eq!(result, Ok(()));
    assert_eq!(seen.request_line, "POST /v1/responses HTTP/1.1");
    assert!(
        seen.header("authorization") == Some("Bearer test-key-123"),
        "the authorization header did not match the configured key"
    );
    assert_eq!(seen.header("x-api-key"), None);
}

#[tokio::test]
async fn truncated_and_cancelled_streams_stop() {
    let (_, result) = run(Reply::stream(TRUNCATED, PIECE_LEN)).await;
    assert!(matches!(result, Err(ProviderError::MalformedStream { .. })));
    let server = start(Reply::stream(TRUNCATED, PIECE_LEN).then_hold_open()).await;
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
    let config = OpenAiConfig::new(KEY);
    let client = crate::client_builder().build().unwrap();
    let provider = OpenAiProvider::new(client, config.clone());
    assert!(!format!("{config:?} {provider:?}").contains(KEY));
}

#[tokio::test]
async fn a_region_403_maps_to_blocked_with_the_providers_message() {
    let (_, result) = run(Reply::json("403 Forbidden", &[], REGION_BODY)).await;
    assert_eq!(
        result,
        Err(ProviderError::Blocked {
            message: REGION_MESSAGE.to_owned()
        })
    );
}

#[tokio::test]
async fn a_plain_403_maps_to_auth() {
    let (_, result) = run(Reply::json("403 Forbidden", &[], PERMISSION_BODY)).await;
    assert_eq!(
        result,
        Err(ProviderError::Auth {
            message: "insufficient permissions".to_owned()
        })
    );
}

#[tokio::test]
async fn a_403_code_classification_ignores_unrelated_messages() {
    let region_body = r#"{"error":{"message":"Request failed.","type":"request_forbidden","param":null,"code":"unsupported_country_region_territory"}}"#;
    let (_, region_result) = run(Reply::json("403 Forbidden", &[], region_body)).await;
    assert_eq!(
        region_result,
        Err(ProviderError::Blocked {
            message: "Request failed.".to_owned()
        })
    );

    let permission_body = r#"{"error":{"message":"Request failed.","type":"invalid_request_error","param":null,"code":"insufficient_permissions"}}"#;
    let (_, permission_result) = run(Reply::json("403 Forbidden", &[], permission_body)).await;
    assert_eq!(
        permission_result,
        Err(ProviderError::Auth {
            message: "Request failed.".to_owned()
        })
    );
}

#[test]
fn classify_blocks_a_region_403_by_code_or_by_message_in_any_case() {
    let blocked = |message: &str| {
        Some(ProviderError::Blocked {
            message: message.to_owned(),
        })
    };
    assert_eq!(
        OpenAiErrors.classify(StatusCode::FORBIDDEN, REGION_BODY.as_bytes()),
        blocked(REGION_MESSAGE)
    );
    let by_message =
        br#"{"error":{"message":"COUNTRY, REGION, OR TERRITORY not supported","code":null}}"#;
    assert_eq!(
        OpenAiErrors.classify(StatusCode::FORBIDDEN, by_message),
        blocked("COUNTRY, REGION, OR TERRITORY not supported")
    );
}

#[test]
fn classify_leaves_other_403s_and_other_statuses_to_the_default_mapping() {
    assert_eq!(
        OpenAiErrors.classify(StatusCode::FORBIDDEN, PERMISSION_BODY.as_bytes()),
        None
    );
    assert_eq!(
        OpenAiErrors.classify(StatusCode::UNAUTHORIZED, REGION_BODY.as_bytes()),
        None
    );
}
