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

/// The URLs that a key check sends, in request order. They never hold a key.
///
/// Returns an empty list when the shared HTTP client cannot start.
#[must_use]
pub fn provider_test_targets(provider: Provider) -> Vec<String> {
    catalog_for(provider, "")
        .map(|catalog| {
            catalog
                .verify_url()
                .into_iter()
                .chain(std::iter::once(catalog.list_url()))
                .collect()
        })
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
    let cancel = CancellationToken::new();
    let request = async {
        if let Err(error) = catalog.verify_key(cancel.clone()).await {
            return status_for_error(&error);
        }
        status_for_result(catalog.list_models(cancel).await)
    };
    match tokio::time::timeout(limit, request).await {
        Ok(result) => result,
        Err(_elapsed) => ProviderCheck::with_status(ConnectionStatus::NetworkError),
    }
}

fn status_for_error(error: &ProviderError) -> ProviderCheck {
    match error {
        ProviderError::Auth { .. } => ProviderCheck::with_status(ConnectionStatus::BadKey),
        ProviderError::Blocked { .. } => ProviderCheck::with_status(ConnectionStatus::Blocked),
        _ => ProviderCheck::with_status(ConnectionStatus::NetworkError),
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
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

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

    struct Scripted {
        verify: Option<Result<(), ProviderError>>,
        list: Result<Vec<ModelInfo>, ProviderError>,
        verify_calls: Arc<AtomicUsize>,
        list_calls: Arc<AtomicUsize>,
    }

    impl ModelCatalog for Scripted {
        fn list_url(&self) -> String {
            String::new()
        }

        fn verify_key(
            &self,
            _cancel: CancellationToken,
        ) -> BoxFuture<'_, Result<(), ProviderError>> {
            self.verify_calls.fetch_add(1, Ordering::SeqCst);
            match &self.verify {
                Some(result) => Box::pin(std::future::ready(result.clone())),
                None => Box::pin(pending()),
            }
        }

        fn list_models(
            &self,
            _cancel: CancellationToken,
        ) -> BoxFuture<'_, Result<Vec<ModelInfo>, ProviderError>> {
            self.list_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(std::future::ready(self.list.clone()))
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

    fn scripted_catalog(
        verify: Option<Result<(), ProviderError>>,
        list: Result<Vec<ModelInfo>, ProviderError>,
    ) -> (Scripted, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let verify_calls = Arc::new(AtomicUsize::new(0));
        let list_calls = Arc::new(AtomicUsize::new(0));
        (
            Scripted {
                verify,
                list,
                verify_calls: Arc::clone(&verify_calls),
                list_calls: Arc::clone(&list_calls),
            },
            verify_calls,
            list_calls,
        )
    }

    #[tokio::test]
    async fn a_failed_key_verification_maps_and_skips_model_listing() {
        let message = String::new;
        let cases = [
            (
                ProviderError::Auth { message: message() },
                ConnectionStatus::BadKey,
            ),
            (
                ProviderError::Blocked { message: message() },
                ConnectionStatus::Blocked,
            ),
            (
                ProviderError::RateLimited {
                    message: message(),
                    retry_after_secs: Some(7),
                },
                ConnectionStatus::NetworkError,
            ),
            (
                ProviderError::InvalidRequest { message: message() },
                ConnectionStatus::NetworkError,
            ),
            (
                ProviderError::Server {
                    status: 503,
                    message: message(),
                },
                ConnectionStatus::NetworkError,
            ),
            (
                ProviderError::Network { message: message() },
                ConnectionStatus::NetworkError,
            ),
            (
                ProviderError::MalformedStream { message: message() },
                ConnectionStatus::NetworkError,
            ),
            (ProviderError::Cancelled, ConnectionStatus::NetworkError),
        ];
        assert_failed_verification(cases[0].0.clone(), cases[0].1).await;
        assert_failed_verification(cases[1].0.clone(), cases[1].1).await;
        assert_failed_verification(cases[2].0.clone(), cases[2].1).await;
        assert_failed_verification(cases[3].0.clone(), cases[3].1).await;
        assert_failed_verification(cases[4].0.clone(), cases[4].1).await;
        assert_failed_verification(cases[5].0.clone(), cases[5].1).await;
        assert_failed_verification(cases[6].0.clone(), cases[6].1).await;
        assert_failed_verification(cases[7].0.clone(), cases[7].1).await;
    }

    async fn assert_failed_verification(error: ProviderError, expected: ConnectionStatus) {
        let (catalog, verify_calls, list_calls) =
            scripted_catalog(Some(Err(error)), Ok(vec![model("not-called")]));
        let check = check(&catalog, CHECK_LIMIT).await;

        assert_eq!(check.status, expected);
        assert_eq!(check.models, Vec::<ModelInfo>::new());
        assert_eq!(verify_calls.load(Ordering::SeqCst), 1);
        assert_eq!(list_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn successful_key_verification_then_lists_models() {
        let models = vec![model("gpt-a")];
        let (catalog, verify_calls, list_calls) =
            scripted_catalog(Some(Ok(())), Ok(models.clone()));

        assert_eq!(check(&catalog, CHECK_LIMIT).await.models, models);
        assert_eq!(verify_calls.load(Ordering::SeqCst), 1);
        assert_eq!(list_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_key_verification_that_never_answers_is_a_network_error() {
        let (catalog, verify_calls, list_calls) = scripted_catalog(None, Ok(Vec::new()));
        let check = check(&catalog, Duration::from_millis(10)).await;

        assert_eq!(check.status, ConnectionStatus::NetworkError);
        assert_eq!(verify_calls.load(Ordering::SeqCst), 1);
        assert_eq!(list_calls.load(Ordering::SeqCst), 0);
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
    fn provider_test_targets_are_exact_and_ordered_for_each_provider() {
        assert_eq!(
            provider_test_targets(Provider::Anthropic),
            vec!["https://api.anthropic.com/v1/models?limit=1000"]
        );
        assert_eq!(
            provider_test_targets(Provider::OpenAi),
            vec!["https://api.openai.com/v1/models"]
        );
        assert_eq!(
            provider_test_targets(Provider::Gemini),
            vec!["https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000"]
        );
        assert_eq!(
            provider_test_targets(Provider::DeepSeek),
            vec!["https://api.deepseek.com/models"]
        );
        assert_eq!(
            provider_test_targets(Provider::OpenRouter),
            vec![
                "https://openrouter.ai/api/v1/key",
                "https://openrouter.ai/api/v1/models"
            ]
        );
    }
}
