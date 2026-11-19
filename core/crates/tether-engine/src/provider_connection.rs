//! A low-cost authenticated catalog request checks each provider key.

use std::time::Duration;

use tether_providers::{
    AnthropicConfig, AnthropicProvider, DeepSeekConfig, DeepSeekProvider, GeminiConfig,
    GeminiProvider, ModelCatalog, OpenAiConfig, OpenAiProvider, OpenRouterConfig,
    OpenRouterProvider, ProviderError, http_client,
};
use tether_types::{ConnectionStatus, ModelInfo, Provider};
use tokio_util::sync::CancellationToken;

/// The longest a key check may take before it reports a network error.
const CHECK_LIMIT: Duration = Duration::from_secs(20);

/// What a key check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCheck {
    /// `Ok`, `BadKey`, `NetworkError`, or `Blocked`. Never `Unset` or `Testing`.
    pub status: ConnectionStatus,
    /// The catalog when `status` is `Ok`, otherwise empty.
    pub models: Vec<ModelInfo>,
}

impl ProviderCheck {
    fn with_status(status: ConnectionStatus) -> Self {
        Self {
            status,
            models: Vec::new(),
        }
    }
}

/// Checks a provider key with its model-list endpoint, without generating paid tokens.
///
/// A blank key returns `BadKey` without a network call. The whole check is limited to 20 seconds.
pub async fn check_provider(provider: Provider, key: &str) -> ProviderCheck {
    check_key(key, CHECK_LIMIT, || catalog_for(provider, key)).await
}

/// The URL that a key check sends its first request to. It never holds a key.
///
/// Returns an empty string when the shared HTTP client cannot start.
#[must_use]
pub fn provider_test_target(provider: Provider) -> String {
    catalog_for(provider, "")
        .map(|catalog| catalog.list_url())
        .unwrap_or_default()
}

/// Rejects a blank key, then builds the catalog and checks it. The catalog is built only
/// for a non-blank key.
async fn check_key<F>(key: &str, limit: Duration, catalog: F) -> ProviderCheck
where
    F: FnOnce() -> Result<Box<dyn ModelCatalog>, ProviderError>,
{
    if key.trim().is_empty() {
        return ProviderCheck::with_status(ConnectionStatus::BadKey);
    }
    match catalog() {
        Ok(catalog) => check(catalog.as_ref(), limit).await,
        Err(_) => ProviderCheck::with_status(ConnectionStatus::NetworkError),
    }
}

/// Lists models through the catalog, and gives up after `limit`.
async fn check(catalog: &dyn ModelCatalog, limit: Duration) -> ProviderCheck {
    let request = catalog.list_models(CancellationToken::new());
    match tokio::time::timeout(limit, request).await {
        Ok(result) => status_for_result(result),
        Err(_elapsed) => ProviderCheck::with_status(ConnectionStatus::NetworkError),
    }
}

fn status_for_result(result: Result<Vec<ModelInfo>, ProviderError>) -> ProviderCheck {
    match result {
        Ok(models) => ProviderCheck {
            status: ConnectionStatus::Ok,
            models,
        },
        Err(ProviderError::Auth { .. }) => ProviderCheck::with_status(ConnectionStatus::BadKey),
        Err(ProviderError::Blocked { .. }) => ProviderCheck::with_status(ConnectionStatus::Blocked),
        Err(_) => ProviderCheck::with_status(ConnectionStatus::NetworkError),
    }
}

fn catalog_for(provider: Provider, key: &str) -> Result<Box<dyn ModelCatalog>, ProviderError> {
    let client = http_client()?;
    let catalog: Box<dyn ModelCatalog> = match provider {
        Provider::Anthropic => Box::new(AnthropicProvider::new(client, AnthropicConfig::new(key))),
        Provider::OpenAi => Box::new(OpenAiProvider::new(client, OpenAiConfig::new(key))),
        Provider::Gemini => Box::new(GeminiProvider::new(client, GeminiConfig::new(key))),
        Provider::DeepSeek => {
            Box::new(DeepSeekProvider::new(client, DeepSeekConfig::deepseek(key)))
        }
        Provider::OpenRouter => Box::new(OpenRouterProvider::new(
            client,
            OpenRouterConfig::openrouter(key),
        )),
    };
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use std::future::pending;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tether_providers::BoxFuture;
    use tether_types::ModelCaps;

    use super::*;

    /// A catalog that replies with a fixed result.
    struct Fixed(Result<Vec<ModelInfo>, ProviderError>);

    impl ModelCatalog for Fixed {
        fn list_url(&self) -> String {
            String::new()
        }

        fn list_models(
            &self,
            _cancel: CancellationToken,
        ) -> BoxFuture<'_, Result<Vec<ModelInfo>, ProviderError>> {
            Box::pin(std::future::ready(self.0.clone()))
        }
    }

    /// A catalog that never answers.
    struct Silent;

    impl ModelCatalog for Silent {
        fn list_url(&self) -> String {
            String::new()
        }

        fn list_models(
            &self,
            _cancel: CancellationToken,
        ) -> BoxFuture<'_, Result<Vec<ModelInfo>, ProviderError>> {
            Box::pin(pending())
        }
    }

    fn model(id: &str) -> ModelInfo {
        ModelInfo {
            provider: Provider::OpenAi,
            id: id.to_owned(),
            display_name: id.to_owned(),
            context_window: Some(128_000),
            max_output_tokens: None,
            price: None,
            caps: ModelCaps::UNKNOWN,
        }
    }

    /// Checks that one provider error gives the expected status and no models.
    async fn expect_status(error: ProviderError, expected: ConnectionStatus) {
        let fixed = Fixed(Err(error.clone()));
        let check = check(&fixed, CHECK_LIMIT).await;
        assert_eq!(check.status, expected, "{error:?}");
        assert!(check.models.is_empty(), "{error:?}");
    }

    #[tokio::test]
    async fn every_provider_error_maps_to_one_status() {
        let message = String::new;
        expect_status(
            ProviderError::Auth { message: message() },
            ConnectionStatus::BadKey,
        )
        .await;
        expect_status(
            ProviderError::Blocked { message: message() },
            ConnectionStatus::Blocked,
        )
        .await;
        expect_status(
            ProviderError::RateLimited {
                message: message(),
                retry_after_secs: Some(5),
            },
            ConnectionStatus::NetworkError,
        )
        .await;
        expect_status(
            ProviderError::InvalidRequest { message: message() },
            ConnectionStatus::NetworkError,
        )
        .await;
        expect_status(
            ProviderError::Server {
                status: 503,
                message: message(),
            },
            ConnectionStatus::NetworkError,
        )
        .await;
        expect_status(
            ProviderError::Network { message: message() },
            ConnectionStatus::NetworkError,
        )
        .await;
        expect_status(
            ProviderError::MalformedStream { message: message() },
            ConnectionStatus::NetworkError,
        )
        .await;
        expect_status(ProviderError::Cancelled, ConnectionStatus::NetworkError).await;
    }

    #[tokio::test]
    async fn ok_returns_the_same_models_in_the_same_order() {
        let models = vec![model("gpt-b"), model("gpt-a"), model("gpt-c")];
        let check = check(&Fixed(Ok(models.clone())), CHECK_LIMIT).await;
        assert_eq!(
            check,
            ProviderCheck {
                status: ConnectionStatus::Ok,
                models,
            }
        );
    }

    #[tokio::test]
    async fn a_catalog_that_never_answers_is_a_network_error() {
        let check = check(&Silent, Duration::from_millis(10)).await;
        assert_eq!(
            check,
            ProviderCheck {
                status: ConnectionStatus::NetworkError,
                models: Vec::new(),
            }
        );
    }

    /// Counts a build, then returns a catalog that lists one model.
    fn counted_catalog(builds: &AtomicUsize) -> Box<dyn ModelCatalog> {
        builds.fetch_add(1, Ordering::SeqCst);
        Box::new(Fixed(Ok(vec![model("gpt-a")])))
    }

    /// Checks that a blank key gives `BadKey` and builds no catalog.
    async fn expect_blank_key(key: &str, builds: &AtomicUsize) {
        let check = check_key(key, CHECK_LIMIT, || Ok(counted_catalog(builds))).await;
        assert_eq!(
            check,
            ProviderCheck::with_status(ConnectionStatus::BadKey),
            "{key:?}"
        );
    }

    #[tokio::test]
    async fn a_blank_key_never_builds_or_calls_the_catalog() {
        let builds = AtomicUsize::new(0);
        expect_blank_key("", &builds).await;
        expect_blank_key("   ", &builds).await;
        expect_blank_key("\t\n", &builds).await;
        assert_eq!(builds.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_real_key_builds_the_catalog_once() {
        let builds = AtomicUsize::new(0);
        let check = check_key("sk-test", CHECK_LIMIT, || Ok(counted_catalog(&builds))).await;
        assert_eq!(check.status, ConnectionStatus::Ok);
        assert_eq!(check.models, vec![model("gpt-a")]);
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn key_checks_are_limited_to_twenty_seconds() {
        assert_eq!(CHECK_LIMIT, Duration::from_secs(20));
    }

    #[test]
    fn provider_test_target_is_the_model_list_url_of_each_provider() {
        assert_eq!(
            provider_test_target(Provider::Anthropic),
            "https://api.anthropic.com/v1/models?limit=1000"
        );
        assert_eq!(
            provider_test_target(Provider::OpenAi),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            provider_test_target(Provider::Gemini),
            "https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000"
        );
        assert_eq!(
            provider_test_target(Provider::DeepSeek),
            "https://api.deepseek.com/models"
        );
        assert_eq!(
            provider_test_target(Provider::OpenRouter),
            "https://openrouter.ai/api/v1/models"
        );
    }
}
