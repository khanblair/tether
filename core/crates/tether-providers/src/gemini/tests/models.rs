use super::KEY;
use crate::test_support::{Reply, request_line_for, start_many};
use crate::{GeminiConfig, GeminiProvider, ModelCatalog};
use tether_types::{ModelCaps, ModelInfo, ModelPrice, PriceSource, Provider, Support};
use tokio_util::sync::CancellationToken;

const PAGE_ONE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/model-lists/gemini-page1.json"
));
const PAGE_TWO: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/model-lists/gemini-page2.json"
));

fn provider(base_url: String) -> GeminiProvider {
    let config = GeminiConfig {
        base_url,
        ..GeminiConfig::new(KEY)
    };
    GeminiProvider::new(crate::client_builder().no_proxy().build().unwrap(), config)
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

fn model(
    id: &str,
    name: &str,
    limits: (u32, u32),
    price: ModelPrice,
    reasoning: Support,
) -> ModelInfo {
    ModelInfo {
        provider: Provider::Gemini,
        id: id.to_owned(),
        display_name: name.to_owned(),
        context_window: Some(limits.0),
        max_output_tokens: Some(limits.1),
        price: Some(price),
        caps: ModelCaps {
            tools: Support::Unknown,
            images: Support::Unknown,
            reasoning,
        },
    }
}

fn expected() -> Vec<ModelInfo> {
    vec![
        model(
            "gemini-4-neon",
            "Gemini 4 Neon",
            (1_048_576, 65_536),
            price(3_000_000, 15_000_000, 150_000),
            Support::Yes,
        ),
        model(
            "gemini-2.5-flash-lite",
            "Gemini 2.5 Flash-Lite",
            (1_048_576, 65_536),
            price(100_000, 400_000, 10_000),
            Support::No,
        ),
        model(
            "gemini-3.8-flash",
            "Gemini 3.8 Flash",
            (1_048_576, 65_536),
            price(750_000, 3_750_000, 75_000),
            Support::Unknown,
        ),
    ]
}

#[tokio::test]
async fn lists_generation_models_in_page_order() {
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
    assert_eq!(
        seen[0].request_line,
        "GET /v1beta/models?pageSize=1000 HTTP/1.1"
    );
    assert_eq!(
        seen[1].request_line,
        "GET /v1beta/models?pageSize=1000&pageToken=page-2-token HTTP/1.1"
    );
    assert_eq!(seen[0].header("x-goog-api-key"), Some(KEY));
    assert!(!seen[0].request_line.contains(KEY));
}

#[tokio::test]
async fn percent_encodes_page_token_and_stops_at_ten_pages() {
    let page = r#"{"models":[],"nextPageToken":"a+b/c=d&e"}"#;
    let replies = (0..11).map(|_| Reply::json("200 OK", &[], page)).collect();
    let server = start_many(replies).await;
    let provider = provider(server.base_url);
    assert_eq!(
        provider
            .list_models(CancellationToken::new())
            .await
            .unwrap(),
        Vec::<ModelInfo>::new()
    );

    assert_eq!(server.requests.lock().unwrap().len(), 10);
    assert_eq!(
        server.requests.lock().unwrap()[1].request_line,
        "GET /v1beta/models?pageSize=1000&pageToken=a%2Bb%2Fc%3Dd%26e HTTP/1.1"
    );
    server.seen.abort();
}

#[test]
fn list_url_is_the_exact_first_page() {
    let client = crate::client_builder().build().unwrap();
    let provider = GeminiProvider::new(client, GeminiConfig::new(KEY));
    assert_eq!(
        provider.list_url(),
        "https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000"
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
    assert_eq!(
        seen[0].request_line,
        "GET /v1beta/models?pageSize=1000 HTTP/1.1"
    );
}
