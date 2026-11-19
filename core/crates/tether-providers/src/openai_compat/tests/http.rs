//! HTTP contract tests for the OpenAI-compatible adapter.

use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{KEY, collect, request};
use crate::test_support::{Reply, start};
use crate::{ChatProvider, OpenAiCompatConfig, OpenAiCompatProvider, ProviderError, StreamEvent};

/// Streams one request to `reply`, using the given configuration with the server's address.
async fn stream_with(config: OpenAiCompatConfig, reply: Reply) -> Result<(), ProviderError> {
    let server = start(reply).await;
    let config = OpenAiCompatConfig {
        base_url: server.base_url,
        ..config
    };
    let (sender, _receiver) = tokio::sync::mpsc::channel(8);
    OpenAiCompatProvider::new(client(), config)
        .stream_chat(&request(), sender, CancellationToken::new())
        .await
}

const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/provider-streams/deepseek-text.sse"
));

fn client() -> reqwest::Client {
    crate::client_builder().no_proxy().build().unwrap()
}

fn provider(base_url: String) -> OpenAiCompatProvider {
    OpenAiCompatProvider::new(client(), OpenAiCompatConfig::custom(KEY, base_url))
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
async fn request_headers_and_path_are_exact() {
    let server = start(Reply::stream(TEXT, 7)).await;
    let provider = provider(format!("{}/api/v1", server.base_url));
    let (sender, receiver) = mpsc::channel(8);
    let result = provider
        .stream_chat(&request(), sender, CancellationToken::new())
        .await;
    drop(receiver);
    let seen = server.seen.await.unwrap().unwrap();
    let _: Value = serde_json::from_str(&seen.body).unwrap();
    assert_eq!(result, Ok(()));
    assert_eq!(seen.request_line, "POST /api/v1/chat/completions HTTP/1.1");
    assert_eq!(seen.header("authorization"), Some("Bearer test-key-123"));
    assert_eq!(seen.header("x-api-key"), None);
}

#[tokio::test]
async fn statuses_and_retry_after_are_exact() {
    let body = r#"{"error":{"message":"bad","type":"x","code":"x"}}"#;
    assert_eq!(
        run(Reply::json("401 Unauthorized", &[], body)).await.1,
        Err(ProviderError::Auth {
            message: "bad".to_owned()
        })
    );
    assert_eq!(
        run(Reply::json("402 Payment Required", &[], body)).await.1,
        Err(ProviderError::Auth {
            message: "bad".to_owned()
        })
    );
    assert_eq!(
        run(Reply::json("400 Bad Request", &[], body)).await.1,
        Err(ProviderError::InvalidRequest {
            message: "bad".to_owned()
        })
    );
    assert_eq!(
        run(Reply::json("500 Server Error", &[], body)).await.1,
        Err(ProviderError::Server {
            status: 500,
            message: "bad".to_owned()
        })
    );
    let rate = r#"{"error":{"message":"slow","type":"x","code":429}}"#;
    assert_eq!(
        run(Reply::json(
            "429 Too Many Requests",
            &["retry-after: 7"],
            rate
        ))
        .await
        .1,
        Err(ProviderError::RateLimited {
            message: "slow".to_owned(),
            retry_after_secs: Some(7)
        })
    );
}

#[tokio::test]
async fn truncated_stream_and_cancel_are_exact() {
    let (_, result) = run(Reply::stream("data: {\"choices\":[]}\n\n", 3)).await;
    assert_eq!(
        result,
        Err(ProviderError::MalformedStream {
            message: "the stream ended before [DONE]".to_owned()
        })
    );
    let server = start(Reply::stream(TEXT, 7).then_hold_open()).await;
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
fn debug_hides_key_and_openrouter_headers_are_configured() {
    let deepseek = OpenAiCompatConfig::deepseek(KEY);
    let openrouter = OpenAiCompatConfig::openrouter(KEY);
    let provider = OpenAiCompatProvider::new(client(), deepseek.clone());
    assert!(!format!("{deepseek:?} {provider:?}").contains(KEY));
    assert_eq!(
        openrouter.extra_headers,
        vec![
            (
                "HTTP-Referer".to_owned(),
                "https://github.com/khanblair/tether".to_owned()
            ),
            ("X-Title".to_owned(), "Tether".to_owned())
        ]
    );
}

#[tokio::test]
async fn extra_headers_are_sent_with_the_request() {
    let server = start(Reply::stream(TEXT, 7)).await;
    let mut config = OpenAiCompatConfig::custom(KEY, server.base_url.clone());
    config.extra_headers = vec![("X-Title".to_owned(), "Tether".to_owned())];
    let provider = OpenAiCompatProvider::new(client(), config);
    let (sender, receiver) = mpsc::channel(8);
    let result = provider
        .stream_chat(&request(), sender, CancellationToken::new())
        .await;
    drop(receiver);
    let seen = server.seen.await.unwrap().unwrap();
    assert_eq!(result, Ok(()));
    assert_eq!(seen.header("x-title"), Some("Tether"));
    assert_eq!(seen.header("authorization"), Some("Bearer test-key-123"));
}

#[tokio::test]
async fn an_extra_header_cannot_replace_the_authorization_header() {
    let mut config = OpenAiCompatConfig::custom(KEY, "http://127.0.0.1:9");
    config.extra_headers = vec![("Authorization".to_owned(), "Bearer other".to_owned())];
    let provider = OpenAiCompatProvider::new(client(), config);
    let (sender, _receiver) = mpsc::channel(1);
    let result = provider
        .stream_chat(&request(), sender, CancellationToken::new())
        .await;
    assert!(matches!(result, Err(ProviderError::InvalidRequest { .. })));
}

#[tokio::test]
async fn a_403_maps_to_auth_for_deepseek_and_openrouter() {
    let body = r#"{"error":{"message":"permission denied","type":"x","code":"x"}}"#;
    for config in [
        OpenAiCompatConfig::deepseek(KEY),
        OpenAiCompatConfig::openrouter(KEY),
    ] {
        assert_eq!(
            stream_with(config, Reply::json("403 Forbidden", &[], body)).await,
            Err(ProviderError::Auth {
                message: "permission denied".to_owned()
            })
        );
    }
}
