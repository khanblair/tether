//! Shared model catalog contract and mapping helpers.

use tether_types::{ModelCaps, ModelInfo, ModelPrice, Provider, Support};
use tokio_util::sync::CancellationToken;

use crate::{BoxFuture, ProviderError};

/// Lists the chat models served by a provider.
pub trait ModelCatalog: Send + Sync {
    /// The URL of the first model-list request, exactly as it is sent. It never holds the key.
    /// It is empty when the base URL is not a valid URL, and `list_models` then fails.
    fn list_url(&self) -> String;

    /// The URL of a separate authenticated key check, when the list endpoint is public.
    fn verify_url(&self) -> Option<String> {
        None
    }

    /// Verifies the key before the model list request.
    fn verify_key(&self, _cancel: CancellationToken) -> BoxFuture<'_, Result<(), ProviderError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    /// Lists models with context, price, and capability badges.
    fn list_models(
        &self,
        cancel: CancellationToken,
    ) -> BoxFuture<'_, Result<Vec<ModelInfo>, ProviderError>>;
}

pub(crate) fn token_limit(value: Option<&serde_json::Value>) -> Option<u32> {
    let number = value?.as_u64()?;
    (number > 0).then(|| u32::try_from(number).ok()).flatten()
}

pub(crate) fn support(value: Option<bool>) -> Support {
    value.map_or(Support::Unknown, |value| {
        if value { Support::Yes } else { Support::No }
    })
}

pub(crate) fn display_name(name: Option<&str>, id: &str) -> String {
    name.filter(|name| !name.is_empty())
        .unwrap_or(id)
        .to_owned()
}

#[derive(Clone, Copy)]
pub(crate) struct LiveDetails<'a> {
    /// Provider display name, when present.
    pub(crate) name: Option<&'a str>,
    /// Live context window.
    pub(crate) context: Option<u32>,
    /// Live output limit.
    pub(crate) output: Option<u32>,
    /// Live capability values.
    pub(crate) caps: ModelCaps,
}

pub(crate) fn details(
    provider: Provider,
    id: &str,
    live: LiveDetails<'_>,
    price: Option<ModelPrice>,
) -> ModelInfo {
    let bundled = crate::pricing::bundled_details(provider, id);
    ModelInfo {
        provider,
        id: id.to_owned(),
        display_name: display_name(live.name, id),
        context_window: live
            .context
            .or_else(|| bundled.as_ref().and_then(|value| value.context_window)),
        max_output_tokens: live
            .output
            .or_else(|| bundled.as_ref().and_then(|value| value.max_output_tokens)),
        price: price.or_else(|| crate::pricing::bundled_price(provider, id)),
        caps: merge_caps(
            live.caps,
            bundled.map_or(ModelCaps::UNKNOWN, |value| value.caps),
        ),
    }
}

fn merge_caps(live: ModelCaps, bundled: ModelCaps) -> ModelCaps {
    ModelCaps {
        tools: merge_support(live.tools, bundled.tools),
        images: merge_support(live.images, bundled.images),
        reasoning: merge_support(live.reasoning, bundled.reasoning),
    }
}

fn merge_support(live: Support, bundled: Support) -> Support {
    if live == Support::Unknown {
        bundled
    } else {
        live
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AnthropicConfig, AnthropicProvider, DeepSeekProvider, GeminiConfig, GeminiProvider,
        OpenAiCompatConfig, OpenAiCompatProvider, OpenAiConfig, OpenAiProvider,
    };

    fn client() -> reqwest::Client {
        crate::client_builder().no_proxy().build().unwrap()
    }

    #[tokio::test]
    async fn default_key_verification_is_immediate_for_catalogs_without_a_verify_url() {
        let closed = "http://127.0.0.1:9".to_owned();
        let catalogs: Vec<Box<dyn ModelCatalog>> = vec![
            Box::new(AnthropicProvider::new(
                client(),
                AnthropicConfig {
                    api_key: "test".to_owned(),
                    base_url: closed.clone(),
                },
            )),
            Box::new(OpenAiProvider::new(
                client(),
                OpenAiConfig {
                    api_key: "test".to_owned(),
                    base_url: closed.clone(),
                    reasoning_summary: true,
                },
            )),
            Box::new(GeminiProvider::new(
                client(),
                GeminiConfig {
                    api_key: "test".to_owned(),
                    base_url: closed.clone(),
                },
            )),
            Box::new(DeepSeekProvider::new(
                client(),
                OpenAiCompatConfig {
                    base_url: closed,
                    ..OpenAiCompatConfig::deepseek("test")
                },
            )),
        ];

        let mut catalogs = catalogs.into_iter();
        assert_default_verification(catalogs.next().unwrap()).await;
        assert_default_verification(catalogs.next().unwrap()).await;
        assert_default_verification(catalogs.next().unwrap()).await;
        assert_default_verification(catalogs.next().unwrap()).await;
    }

    async fn assert_default_verification(catalog: Box<dyn ModelCatalog>) {
        assert_eq!(catalog.verify_url(), None);
        assert_eq!(catalog.verify_key(CancellationToken::new()).await, Ok(()));
    }

    #[test]
    fn openrouter_has_an_authenticated_verify_url() {
        let provider = OpenAiCompatProvider::new(client(), OpenAiCompatConfig::openrouter("test"));
        assert_eq!(
            provider.verify_url(),
            Some("https://openrouter.ai/api/v1/key".to_owned())
        );
    }
}
