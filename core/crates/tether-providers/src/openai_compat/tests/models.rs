use super::KEY;
use crate::test_support::{Reply, start};
use crate::{Dialect, ModelCatalog, OpenAiCompatConfig, OpenAiCompatProvider, ProviderError};
use tether_types::{ModelCaps, ModelInfo, ModelPrice, PriceSource, Provider, Support};
use tokio_util::sync::CancellationToken;

const DEEPSEEK: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/model-lists/deepseek.json"
));
const OPENROUTER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/model-lists/openrouter.json"
));

fn client() -> reqwest::Client {
    crate::client_builder().no_proxy().build().unwrap()
}
fn provider(base_url: String, dialect: Dialect) -> OpenAiCompatProvider {
    let config = OpenAiCompatConfig {
        base_url,
        dialect,
        ..OpenAiCompatConfig::custom(KEY, "http://unused")
    };
    OpenAiCompatProvider::new(client(), config)
}
fn price(
    input: u64,
    output: u64,
    read: Option<u64>,
    write: Option<u64>,
    source: PriceSource,
) -> ModelPrice {
    ModelPrice {
        input_micros: input,
        output_micros: output,
        cache_read_micros: read,
        cache_write_micros: write,
        source,
    }
}
fn model(
    id: &str,
    name: &str,
    limits: (Option<u32>, Option<u32>),
    price: Option<ModelPrice>,
    caps: ModelCaps,
) -> ModelInfo {
    let provider = if id.starts_with("deepseek") {
        Provider::DeepSeek
    } else {
        Provider::OpenRouter
    };
    ModelInfo {
        provider,
        id: id.to_owned(),
        display_name: name.to_owned(),
        context_window: limits.0,
        max_output_tokens: limits.1,
        price,
        caps,
    }
}
fn deepseek_expected() -> Vec<ModelInfo> {
    vec![deepseek_flash(), deepseek_pro(), deepseek_legacy()]
}
fn deepseek_flash() -> ModelInfo {
    model(
        "deepseek-flash",
        "DeepSeek-V4.1-Flash",
        (Some(1_048_576), Some(393_216)),
        Some(price(
            300_000,
            1_200_000,
            Some(6_000),
            None,
            PriceSource::Bundled,
        )),
        ModelCaps {
            tools: Support::Yes,
            images: Support::Yes,
            reasoning: Support::Yes,
        },
    )
}
fn deepseek_pro() -> ModelInfo {
    model(
        "deepseek-v4-pro",
        "DeepSeek-V4-Pro",
        (Some(1_048_576), Some(393_216)),
        Some(price(
            1_320_000,
            3_960_000,
            Some(44_000),
            None,
            PriceSource::Bundled,
        )),
        ModelCaps {
            tools: Support::Yes,
            images: Support::No,
            reasoning: Support::Yes,
        },
    )
}
fn deepseek_legacy() -> ModelInfo {
    model(
        "deepseek-legacy",
        "deepseek-legacy",
        (None, None),
        None,
        ModelCaps {
            tools: Support::Yes,
            images: Support::Unknown,
            reasoning: Support::Unknown,
        },
    )
}
fn openrouter_expected() -> Vec<ModelInfo> {
    vec![
        router_neon(),
        router_schematron(),
        router_auto(),
        router_qwen(),
    ]
}
fn router_neon() -> ModelInfo {
    model(
        "google/gemini-4-neon",
        "Google: Gemini 4 Neon",
        (Some(1_048_576), Some(943_718)),
        Some(price(
            3_000_000,
            15_000_000,
            Some(150_000),
            Some(83_333),
            PriceSource::Provider,
        )),
        ModelCaps {
            tools: Support::Yes,
            images: Support::Yes,
            reasoning: Support::Yes,
        },
    )
}
fn router_schematron() -> ModelInfo {
    model(
        "inference-net/schematron-v2-turbo",
        "Inference.net: Schematron V2 Turbo",
        (Some(128_000), Some(8_192)),
        Some(price(
            30_000,
            150_000,
            Some(30_000),
            None,
            PriceSource::Provider,
        )),
        ModelCaps {
            tools: Support::No,
            images: Support::No,
            reasoning: Support::No,
        },
    )
}
fn router_auto() -> ModelInfo {
    model(
        "openrouter/auto",
        "Auto Router",
        (Some(2_000_000), None),
        None,
        ModelCaps {
            tools: Support::Yes,
            images: Support::Yes,
            reasoning: Support::Yes,
        },
    )
}
fn router_qwen() -> ModelInfo {
    model(
        "qwen/qwen3-coder-flash",
        "Qwen: Qwen3 Coder Flash",
        (Some(1_000_000), Some(65_536)),
        Some(price(
            195_000,
            975_000,
            Some(39_000),
            Some(243_750),
            PriceSource::Provider,
        )),
        ModelCaps {
            tools: Support::Yes,
            images: Support::No,
            reasoning: Support::No,
        },
    )
}

#[tokio::test]
async fn lists_deepseek_models_with_live_details() {
    let server = start(Reply::json("200 OK", &[], DEEPSEEK)).await;
    let actual = provider(server.base_url, Dialect::DeepSeek)
        .list_models(CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(actual, deepseek_expected());
    let seen = server.seen.await.unwrap().unwrap();
    assert_eq!(seen.request_line, "GET /models HTTP/1.1");
    assert_eq!(seen.header("authorization"), Some("Bearer test-key-123"));
}

#[tokio::test]
async fn lists_openrouter_models_with_provider_prices() {
    let server = start(Reply::json("200 OK", &[], OPENROUTER)).await;
    let actual = provider(server.base_url, Dialect::OpenRouter)
        .list_models(CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(actual, openrouter_expected());
    let seen = server.seen.await.unwrap().unwrap();
    assert_eq!(seen.request_line, "GET /models HTTP/1.1");
}

#[tokio::test]
async fn plain_catalog_is_rejected() {
    let provider = provider("http://127.0.0.1:1".to_owned(), Dialect::Plain);
    assert_eq!(
        provider.list_models(CancellationToken::new()).await,
        Err(ProviderError::InvalidRequest {
            message: "custom endpoints have no model catalog yet".to_owned()
        })
    );
}

#[tokio::test]
async fn extra_authorization_header_is_rejected_for_both_catalogs() {
    for dialect in [Dialect::DeepSeek, Dialect::OpenRouter] {
        let mut config = OpenAiCompatConfig::custom(KEY, "http://127.0.0.1:9");
        config.dialect = dialect;
        config.extra_headers = vec![("Authorization".to_owned(), "Bearer other".to_owned())];
        let provider = OpenAiCompatProvider::new(client(), config);

        assert_eq!(
            provider.list_models(CancellationToken::new()).await,
            Err(ProviderError::InvalidRequest {
                message: "an extra header cannot replace the Authorization header".to_owned(),
            })
        );
    }
}
