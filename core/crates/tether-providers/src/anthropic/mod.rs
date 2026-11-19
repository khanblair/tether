//! The Anthropic Messages API adapter.
//!
//! It posts to `/v1/messages` with `stream: true` and maps the server-sent events to
//! [`StreamEvent`](crate::StreamEvent) values. The system prompt and the last tool carry
//! `cache_control` markers, so the stable prefix of a request is cached.

mod http;
mod messages;
mod models;
mod request;
mod stream;
mod wire;

use std::fmt;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{BoxFuture, ChatProvider, ChatRequest, ProviderError, StreamEvent};

/// The address of Anthropic's API.
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// What the adapter needs to reach Anthropic.
///
/// `Debug` hides the key.
#[derive(Clone)]
pub struct AnthropicConfig {
    /// The API key, sent in the `x-api-key` header.
    pub api_key: String,
    /// The API address without a trailing path. Tests point it at a local server.
    pub base_url: String,
}

impl AnthropicConfig {
    /// Uses `api_key` with Anthropic's own address.
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_owned(),
        }
    }
}

impl fmt::Debug for AnthropicConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnthropicConfig")
            .field("api_key", &"<hidden>")
            .field("base_url", &self.base_url)
            .finish()
    }
}

/// Streams chat responses from Anthropic.
///
/// The caller owns the HTTP client and shares it with other providers, so connections are
/// reused. `Debug` hides the key.
pub struct AnthropicProvider {
    client: reqwest::Client,
    config: AnthropicConfig,
}

impl AnthropicProvider {
    /// Creates a provider that sends its requests through `client`.
    #[must_use]
    pub fn new(client: reqwest::Client, config: AnthropicConfig) -> Self {
        Self { client, config }
    }
}

impl fmt::Debug for AnthropicProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnthropicProvider")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl ChatProvider for AnthropicProvider {
    fn stream_chat<'a>(
        &'a self,
        request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(http::stream_chat(
            &self.client,
            &self.config,
            request,
            events,
            cancel,
        ))
    }
}

impl crate::ModelCatalog for AnthropicProvider {
    fn list_models(
        &self,
        cancel: CancellationToken,
    ) -> BoxFuture<'_, Result<Vec<tether_types::ModelInfo>, ProviderError>> {
        Box::pin(models::list(&self.client, &self.config, cancel))
    }
}

#[cfg(test)]
mod tests;
