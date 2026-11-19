//! OpenAI-compatible Chat Completions adapters.

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

const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com";
const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
const REFERER: &str = "https://github.com/khanblair/tether";

/// The wire dialect used by an OpenAI-compatible endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// The common Chat Completions format.
    Plain,
    /// DeepSeek's reasoning and cache fields.
    DeepSeek,
    /// OpenRouter's reasoning and usage fields.
    OpenRouter,
}

/// Configuration for an OpenAI-compatible Chat Completions endpoint.
#[derive(Clone)]
pub struct OpenAiCompatConfig {
    /// The bearer token sent to the endpoint.
    pub api_key: String,
    /// The endpoint base URL without a trailing slash.
    pub base_url: String,
    /// Non-secret headers sent with each request.
    pub extra_headers: Vec<(String, String)>,
    /// The provider-specific wire dialect.
    pub dialect: Dialect,
}

impl OpenAiCompatConfig {
    /// Creates the documented DeepSeek configuration.
    #[must_use]
    pub fn deepseek(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEEPSEEK_BASE_URL.to_owned(),
            extra_headers: Vec::new(),
            dialect: Dialect::DeepSeek,
        }
    }

    /// Creates the documented OpenRouter configuration.
    #[must_use]
    pub fn openrouter(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: OPENROUTER_BASE_URL.to_owned(),
            extra_headers: vec![
                ("HTTP-Referer".to_owned(), REFERER.to_owned()),
                ("X-Title".to_owned(), "Tether".to_owned()),
            ],
            dialect: Dialect::OpenRouter,
        }
    }

    /// Creates a plain configuration for another compatible server.
    #[must_use]
    pub fn custom(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            extra_headers: Vec::new(),
            dialect: Dialect::Plain,
        }
    }
}

impl fmt::Debug for OpenAiCompatConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiCompatConfig")
            .field("api_key", &"<hidden>")
            .field("base_url", &self.base_url)
            .field("extra_headers", &self.extra_headers)
            .field("dialect", &self.dialect)
            .finish()
    }
}

/// Streams Chat Completions responses from an OpenAI-compatible endpoint.
pub struct OpenAiCompatProvider {
    client: reqwest::Client,
    config: OpenAiCompatConfig,
}

impl OpenAiCompatProvider {
    /// Creates a provider that sends requests through `client`.
    #[must_use]
    pub fn new(client: reqwest::Client, config: OpenAiCompatConfig) -> Self {
        Self { client, config }
    }
}

impl fmt::Debug for OpenAiCompatProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiCompatProvider")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl ChatProvider for OpenAiCompatProvider {
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
