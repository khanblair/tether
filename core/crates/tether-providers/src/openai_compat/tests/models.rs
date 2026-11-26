use super::KEY;
use crate::test_support::{Reply, request_line_for, start};
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
async fn deepseek_reasoning_comes_from_bundled_data_without_live_effort() {
    let live = r#"{"object":"list","data":[{"id":"deepseek-flash","object":"model","owned_by":"deepseek"},{"id":"deepseek-v4-pro","object":"model","owned_by":"deepseek"}]}"#;
    let server = start(Reply::json("200 OK", &[], live)).await;
    let actual = provider(server.base_url, Dialect::DeepSeek)
        .list_models(CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(actual.len(), 2);
    for model in actual {
        assert_eq!(model.caps.reasoning, Support::Yes, "{}", model.id);
    }
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

#[test]
fn list_url_is_the_exact_model_list_for_each_catalog() {
    let deepseek = OpenAiCompatProvider::new(client(), OpenAiCompatConfig::deepseek(KEY));
    assert_eq!(deepseek.list_url(), "https://api.deepseek.com/models");
    assert_eq!(deepseek.verify_url(), None);
    let openrouter = OpenAiCompatProvider::new(client(), OpenAiCompatConfig::openrouter(KEY));
    assert_eq!(openrouter.list_url(), "https://openrouter.ai/api/v1/models");
    assert_eq!(
        openrouter.verify_url(),
        Some("https://openrouter.ai/api/v1/key".to_owned())
    );
}

#[tokio::test]
async fn list_url_is_the_request_the_server_sees_for_each_catalog() {
    for dialect in [Dialect::DeepSeek, Dialect::OpenRouter] {
        let server = start(Reply::json("200 OK", &[], "{}")).await;
        let provider = provider(server.base_url, dialect);
        let list_url = provider.list_url();
        let models = provider.list_models(CancellationToken::new()).await;
        let seen = server.seen.await.unwrap().unwrap();
        assert_eq!(models, Ok(Vec::new()), "{dialect:?}");
        assert_eq!(
            seen.request_line,
            request_line_for(&list_url),
            "{dialect:?}"
        );
        assert_eq!(seen.request_line, "GET /models HTTP/1.1", "{dialect:?}");
    }
}

async fn verify_with(reply: Reply) -> Result<(), ProviderError> {
    let server = start(reply).await;
    let config = OpenAiCompatConfig {
        base_url: server.base_url,
        ..OpenAiCompatConfig::openrouter(KEY)
    };
    OpenAiCompatProvider::new(client(), config)
        .verify_key(CancellationToken::new())
        .await
}

#[tokio::test]
async fn openrouter_verify_key_authenticates_and_uses_its_url() {
    let server = start(Reply::json("200 OK", &[], "{}")).await;
    let config = OpenAiCompatConfig {
        base_url: server.base_url,
        ..OpenAiCompatConfig::openrouter(KEY)
    };
    let provider = OpenAiCompatProvider::new(client(), config);
    let verify_url = provider.verify_url().unwrap();

    assert_eq!(provider.verify_key(CancellationToken::new()).await, Ok(()));

    let seen = server.seen.await.unwrap().unwrap();
    assert_eq!(seen.request_line, request_line_for(&verify_url));
    assert_eq!(seen.request_line, "GET /key HTTP/1.1");
    assert_eq!(seen.header("authorization"), Some("Bearer test-key-123"));
    assert_eq!(
        seen.header("http-referer"),
        Some("https://github.com/khanblair/tether")
    );
    assert_eq!(seen.header("x-title"), Some("Tether"));
}

#[tokio::test]
async fn openrouter_verify_key_maps_auth_and_rate_limit_statuses() {
    let body = r#"{"error":{"message":"bad","type":"x","code":"x"}}"#;
    assert_eq!(
        verify_with(Reply::json("401 Unauthorized", &[], body)).await,
        Err(ProviderError::Auth {
            message: "bad".to_owned()
        })
    );
    assert_eq!(
        verify_with(Reply::json("403 Forbidden", &[], body)).await,
        Err(ProviderError::Auth {
            message: "bad".to_owned()
        })
    );
    let rate = r#"{"error":{"message":"slow","type":"x","code":429}}"#;
    assert_eq!(
        verify_with(Reply::json(
            "429 Too Many Requests",
            &["retry-after: 7"],
            rate
        ))
        .await,
        Err(ProviderError::RateLimited {
            message: "slow".to_owned(),
            retry_after_secs: Some(7)
        })
    );
}

#[tokio::test]
async fn openrouter_verify_key_sends_extra_headers_and_rejects_authorization_override() {
    let server = start(Reply::json("200 OK", &[], "{}")).await;
    let mut config = OpenAiCompatConfig::openrouter(KEY);
    config.base_url = server.base_url;
    let provider = OpenAiCompatProvider::new(client(), config);
    assert_eq!(provider.verify_key(CancellationToken::new()).await, Ok(()));
    let seen = server.seen.await.unwrap().unwrap();
    assert_eq!(seen.header("x-title"), Some("Tether"));
    assert_eq!(seen.header("authorization"), Some("Bearer test-key-123"));

    let mut config = OpenAiCompatConfig::openrouter(KEY);
    config.base_url = "http://127.0.0.1:9".to_owned();
    config
        .extra_headers
        .push(("Authorization".to_owned(), "Bearer other".to_owned()));
    let provider = OpenAiCompatProvider::new(client(), config);
    assert_eq!(
        provider.verify_key(CancellationToken::new()).await,
        Err(ProviderError::InvalidRequest {
            message: "an extra header cannot replace the Authorization header".to_owned()
        })
    );
}
