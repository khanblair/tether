//! The Gemini generate-content API adapter.

mod http;
mod messages;
mod request;
mod stream;
mod wire;

use std::fmt;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{BoxFuture, ChatProvider, ChatRequest, ProviderError, StreamEvent};

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// What the Gemini adapter needs to reach Google's API.
#[derive(Clone)]
pub struct GeminiConfig {
    /// The API key, sent in the `x-goog-api-key` header.
    pub api_key: String,
    /// The API address without a trailing path.
    pub base_url: String,
}

impl GeminiConfig {
    /// Uses `api_key` with Google's Gemini API address.
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_owned(),
        }
    }
}

impl fmt::Debug for GeminiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GeminiConfig")
            .field("api_key", &"<hidden>")
            .field("base_url", &self.base_url)
            .finish()
    }
}

/// Streams chat responses from Gemini.
pub struct GeminiProvider {
    client: reqwest::Client,
    config: GeminiConfig,
}

impl GeminiProvider {
    /// Creates a provider that sends requests through `client`.
    #[must_use]
    pub fn new(client: reqwest::Client, config: GeminiConfig) -> Self {
        Self { client, config }
    }
}

impl fmt::Debug for GeminiProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GeminiProvider")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl ChatProvider for GeminiProvider {
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

#[cfg(test)]
mod tests;
