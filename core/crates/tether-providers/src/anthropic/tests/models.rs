use super::KEY;
use crate::test_support::{Reply, request_line_for, start_many};
use crate::{AnthropicConfig, AnthropicProvider, ModelCatalog};
use tether_types::{ModelCaps, ModelInfo, ModelPrice, PriceSource, Provider, Support};
use tokio_util::sync::CancellationToken;

const PAGE_ONE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/model-lists/anthropic-page1.json"
));
const PAGE_TWO: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/model-lists/anthropic-page2.json"
));

fn provider(base_url: String) -> AnthropicProvider {
    let config = AnthropicConfig {
        base_url,
        ..AnthropicConfig::new(KEY)
    };
    AnthropicProvider::new(crate::client_builder().no_proxy().build().unwrap(), config)
}

fn price(input: u64, output: u64, read: u64, write: u64) -> ModelPrice {
    ModelPrice {
        input_micros: input,
        output_micros: output,
        cache_read_micros: Some(read),
        cache_write_micros: Some(write),
        source: PriceSource::Bundled,
    }
}

fn model(
    id: &str,
    name: &str,
    limits: (Option<u32>, Option<u32>),
    price: Option<ModelPrice>,
    caps: ModelCaps,
) -> ModelInfo {
    ModelInfo {
        provider: Provider::Anthropic,
        id: id.to_owned(),
        display_name: name.to_owned(),
        context_window: limits.0,
        max_output_tokens: limits.1,
        price,
        caps,
    }
}

fn expected() -> Vec<ModelInfo> {
    let caps = |images, reasoning| ModelCaps {
        tools: Support::Yes,
        images,
        reasoning,
    };
    vec![
        model(
            "claude-opus-5-5",
            "Claude Opus 5.5",
            (Some(1_000_000), Some(128_000)),
            Some(price(4_000_000, 20_000_000, 200_000, 5_000_000)),
            caps(Support::Yes, Support::Yes),
        ),
        model(
            "claude-haiku-5-5",
            "Claude Haiku 5.5",
            (Some(200_000), Some(64_000)),
            Some(price(100_000, 500_000, 10_000, 125_000)),
            caps(Support::Yes, Support::No),
        ),
        model(
            "claude-old-text",
            "Claude Old Text",
            (None, None),
            None,
            caps(Support::Unknown, Support::Unknown),
        ),
        model(
            "claude-sonnet-5-5",
            "Claude Sonnet 5.5",
            (Some(1_000_000), Some(64_000)),
            Some(price(2_000_000, 10_000_000, 100_000, 2_500_000)),
            caps(Support::Yes, Support::Yes),
        ),
    ]
}

#[tokio::test]
async fn lists_all_live_models_in_page_order() {
    let server = start_many(vec![
        Reply::json("200 OK", &[], PAGE_ONE),
        Reply::json("200 OK", &[], PAGE_TWO),
    ])
    .await;
    let provider = provider(server.base_url);
    let actual = provider
        .list_models(CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(actual, expected());
    let seen = server.seen.await.unwrap().unwrap();
    assert_eq!(seen[0].request_line, "GET /v1/models?limit=1000 HTTP/1.1");
    assert_eq!(
        seen[1].request_line,
        "GET /v1/models?limit=1000&after_id=claude-old-text HTTP/1.1"
    );
    assert_eq!(seen[0].header("x-api-key"), Some(KEY));
    assert_eq!(seen[0].header("anthropic-version"), Some("2023-06-01"));
}

#[tokio::test]
async fn percent_encodes_after_id() {
    let page = r#"{"data":[],"has_more":true,"last_id":"a+b/c=d&e"}"#;
    let final_page = r#"{"data":[],"has_more":false}"#;
    let server = start_many(vec![
        Reply::json("200 OK", &[], page),
        Reply::json("200 OK", &[], final_page),
    ])
    .await;
    let actual = provider(server.base_url)
        .list_models(CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(actual, Vec::<ModelInfo>::new());
    let seen = server.seen.await.unwrap().unwrap();
    assert_eq!(
        seen[1].request_line,
        "GET /v1/models?limit=1000&after_id=a%2Bb%2Fc%3Dd%26e HTTP/1.1"
    );
}

#[test]
fn list_url_is_the_exact_first_page() {
    let client = crate::client_builder().build().unwrap();
    let provider = AnthropicProvider::new(client, AnthropicConfig::new(KEY));
    assert_eq!(
        provider.list_url(),
        "https://api.anthropic.com/v1/models?limit=1000"
    );
}

#[tokio::test]
async fn list_url_is_the_request_the_server_sees() {
    let server = start_many(vec![Reply::json("200 OK", &[], "{}")]).await;
    let provider = provider(server.base_url);
    let list_url = provider.list_url();
    let models = provider.list_models(CancellationToken::new()).await;
    let seen = server.seen.await.unwrap().unwrap();
    assert_eq!(models, Ok(Vec::new()));
    assert_eq!(seen[0].request_line, request_line_for(&list_url));
    assert_eq!(seen[0].request_line, "GET /v1/models?limit=1000 HTTP/1.1");
}
