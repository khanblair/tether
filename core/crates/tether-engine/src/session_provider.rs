//! The one provider factory used by a saved session.

use std::sync::Arc;

use tether_providers::{
    AnthropicConfig, AnthropicProvider, ChatProvider, DeepSeekConfig, DeepSeekProvider,
    GeminiConfig, GeminiProvider, OpenAiConfig, OpenAiProvider, OpenRouterConfig,
    OpenRouterProvider, ProviderError, http_client,
};
use tether_types::Provider;

/// Builds the selected provider without exposing its key to Kotlin.
///
/// # Errors
/// Returns an error when the shared HTTPS client cannot start.
pub fn provider_for(provider: Provider, key: &str) -> Result<Arc<dyn ChatProvider>, ProviderError> {
    let client = http_client()?;
    Ok(match provider {
        Provider::Anthropic => Arc::new(AnthropicProvider::new(client, AnthropicConfig::new(key))),
        Provider::OpenAi => Arc::new(OpenAiProvider::new(client, OpenAiConfig::new(key))),
        Provider::Gemini => Arc::new(GeminiProvider::new(client, GeminiConfig::new(key))),
        Provider::DeepSeek => {
            Arc::new(DeepSeekProvider::new(client, DeepSeekConfig::deepseek(key)))
        }
        Provider::OpenRouter => Arc::new(OpenRouterProvider::new(
            client,
            OpenRouterConfig::openrouter(key),
        )),
    })
}
