use super::KEY;
use crate::test_support::{Reply, request_line_for, start};
use crate::{ModelCatalog, OpenAiConfig, OpenAiProvider};
use tether_types::{ModelCaps, ModelInfo, ModelPrice, PriceSource, Provider, Support};
use tokio_util::sync::CancellationToken;

const LIST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/model-lists/openai.json"
));

fn provider(base_url: String) -> OpenAiProvider {
    let config = OpenAiConfig {
        base_url,
        ..OpenAiConfig::new(KEY)
    };
    OpenAiProvider::new(crate::client_builder().no_proxy().build().unwrap(), config)
}

fn price(input: u64, output: u64, read: u64) -> ModelPrice {
    ModelPrice {
        input_micros: input,
        output_micros: output,
        cache_read_micros: Some(read),
        cache_write_micros: None,
        source: PriceSource::Bundled,
    }
}

fn model(id: &str, price: Option<ModelPrice>) -> ModelInfo {
    ModelInfo {
        provider: Provider::OpenAi,
        id: id.to_owned(),
        display_name: id.to_owned(),
        context_window: price.map(|_| 1_050_000),
        max_output_tokens: price.map(|_| 128_000),
        price,
        caps: if id == "o3" {
            ModelCaps::UNKNOWN
        } else {
            ModelCaps {
                tools: Support::Yes,
                images: Support::Yes,
                reasoning: Support::Yes,
            }
        },
    }
}

#[tokio::test]
async fn lists_only_chat_models_with_bundled_details() {
    let server = start(Reply::json("200 OK", &[], LIST)).await;
    let actual = provider(server.base_url)
        .list_models(CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(
        actual,
        vec![
            model("gpt-5.6-luna", Some(price(200_000, 1_200_000, 20_000))),
            model(
                "gpt-5.6-luna-2026-10-01",
                Some(price(200_000, 1_200_000, 20_000))
            ),
            model("gpt-6-sol", Some(price(2_000_000, 10_000_000, 200_000))),
            model("o3", None),
        ]
    );
    let seen = server.seen.await.unwrap().unwrap();
    assert_eq!(seen.request_line, "GET /v1/models HTTP/1.1");
    assert_eq!(seen.header("authorization"), Some("Bearer test-key-123"));
    assert_eq!(seen.header("x-api-key"), None);
}

#[tokio::test]
async fn excludes_search_models_from_the_catalog() {
    let body =
        r#"{"data":[{"id":"gpt-6-search"},{"id":"chatgpt-search-preview"},{"id":"gpt-6-sol"}]}"#;
    let server = start(Reply::json("200 OK", &[], body)).await;
    let actual = provider(server.base_url)
        .list_models(CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(
        actual,
        vec![model(
            "gpt-6-sol",
            Some(price(2_000_000, 10_000_000, 200_000))
        )]
    );
}

#[test]
fn list_url_is_the_exact_model_list() {
    let client = crate::client_builder().build().unwrap();
    let provider = OpenAiProvider::new(client, OpenAiConfig::new(KEY));
    assert_eq!(provider.list_url(), "https://api.openai.com/v1/models");
}

#[tokio::test]
async fn list_url_is_the_request_the_server_sees() {
    let server = start(Reply::json("200 OK", &[], "{}")).await;
    let provider = provider(server.base_url);
    let list_url = provider.list_url();
    let models = provider.list_models(CancellationToken::new()).await;
    let seen = server.seen.await.unwrap().unwrap();
    assert_eq!(models, Ok(Vec::new()));
    assert_eq!(seen.request_line, request_line_for(&list_url));
    assert_eq!(seen.request_line, "GET /v1/models HTTP/1.1");
}
