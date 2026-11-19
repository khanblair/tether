//! The OpenAI Responses API adapter.

mod http;
mod messages;
mod request;
mod stream;
mod wire;

use std::fmt;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{BoxFuture, ChatProvider, ChatRequest, ProviderError, StreamEvent};

const DEFAULT_BASE_URL: &str = "https://api.openai.com";

/// Configuration for the OpenAI Responses API.
#[derive(Clone)]
pub struct OpenAiConfig {
    /// The bearer token sent to OpenAI.
    pub api_key: String,
    /// The API address without a trailing path.
    pub base_url: String,
    /// Ask for a reasoning summary whenever a reasoning effort is set. OpenAI sends summary
    /// deltas only on request. Turn it off for an organization that OpenAI has not cleared to
    /// receive summaries.
    pub reasoning_summary: bool,
}

impl OpenAiConfig {
    /// Creates a configuration using OpenAI's public API address.
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_owned(),
            reasoning_summary: true,
        }
    }
}

impl fmt::Debug for OpenAiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiConfig")
            .field("api_key", &"<hidden>")
            .field("base_url", &self.base_url)
            .field("reasoning_summary", &self.reasoning_summary)
            .finish()
    }
}

/// Streams responses from OpenAI.
pub struct OpenAiProvider {
    client: reqwest::Client,
    config: OpenAiConfig,
}

impl OpenAiProvider {
    /// Creates a provider that sends requests through `client`.
    #[must_use]
    pub fn new(client: reqwest::Client, config: OpenAiConfig) -> Self {
        Self { client, config }
    }
}

impl fmt::Debug for OpenAiProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiProvider")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl ChatProvider for OpenAiProvider {
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
