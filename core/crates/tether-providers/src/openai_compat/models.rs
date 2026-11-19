use std::collections::HashSet;

use serde::Deserialize;
use tether_types::{ModelCaps, ModelInfo, ModelPrice, PriceSource, Provider, Support};
use tokio_util::sync::CancellationToken;

use super::{Dialect, OpenAiCompatConfig, http};
use crate::catalog::{LiveDetails, details, support, token_limit};
use crate::pricing::parse_usd_micros;
use crate::transport::{GetCall, get_json, get_status};
use crate::{BoxFuture, ProviderError};

#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    data: Vec<WireModel>,
}

#[derive(Deserialize)]
struct WireModel {
    id: String,
    name: Option<String>,
    context_window: Option<serde_json::Value>,
    max_output_tokens: Option<serde_json::Value>,
    context_length: Option<serde_json::Value>,
    pricing: Option<Pricing>,
    architecture: Option<Architecture>,
    supported_parameters: Option<Vec<String>>,
    top_provider: Option<TopProvider>,
    input_modalities: Option<Vec<String>>,
    effort: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct Pricing {
    prompt: Option<String>,
    completion: Option<String>,
    input_cache_read: Option<String>,
    input_cache_write: Option<String>,
}
#[derive(Deserialize)]
struct Architecture {
    input_modalities: Option<Vec<String>>,
    #[serde(default)]
    output_modalities: Option<Vec<String>>,
}
#[derive(Deserialize)]
struct TopProvider {
    max_completion_tokens: Option<serde_json::Value>,
}

pub(super) async fn list(
    client: &reqwest::Client,
    config: &OpenAiCompatConfig,
    cancel: CancellationToken,
) -> Result<Vec<ModelInfo>, ProviderError> {
    if config.dialect == Dialect::Plain {
        return Err(ProviderError::InvalidRequest {
            message: "custom endpoints have no model catalog yet".to_owned(),
        });
    }
    let page = get_page(client, config, &cancel).await?;
    let provider = match config.dialect {
        Dialect::DeepSeek => Provider::DeepSeek,
        Dialect::OpenRouter => Provider::OpenRouter,
        Dialect::Plain => {
            return Err(ProviderError::InvalidRequest {
                message: "custom endpoints have no model catalog yet".to_owned(),
            });
        }
    };
    let mut seen = HashSet::new();
    Ok(page
        .data
        .into_iter()
        .filter_map(|model| {
            if config.dialect == Dialect::OpenRouter
                && model
                    .architecture
                    .as_ref()
                    .and_then(|architecture| architecture.output_modalities.as_ref())
                    .is_some_and(|values| !values.iter().any(|value| value == "text"))
            {
                return None;
            }
            if !seen.insert(model.id.clone()) {
                return None;
            }
            Some(map_model(provider, config.dialect, &model))
        })
        .collect())
}

/// The URL of the model list. It is the only page, so `list_url` reports it too.
pub(super) fn list_url(config: &OpenAiCompatConfig) -> String {
    format!("{}/models", config.base_url.trim_end_matches('/'))
}

fn key_url(config: &OpenAiCompatConfig) -> String {
    format!("{}/key", config.base_url.trim_end_matches('/'))
}

async fn verify(
    client: &reqwest::Client,
    config: &OpenAiCompatConfig,
    cancel: CancellationToken,
) -> Result<(), ProviderError> {
    if config.dialect != Dialect::OpenRouter {
        return Ok(());
    }
    let headers = http::build_headers(config)?;
    get_status(
        GetCall {
            client,
            url: key_url(config),
            headers,
            errors: &http::CompatErrors,
        },
        &cancel,
    )
    .await
}

async fn get_page(
    client: &reqwest::Client,
    config: &OpenAiCompatConfig,
    cancel: &CancellationToken,
) -> Result<Page, ProviderError> {
    let headers = http::build_headers(config)?;
    get_json(
        GetCall {
            client,
            url: list_url(config),
            headers,
            errors: &http::CompatErrors,
        },
        cancel,
    )
    .await
}

fn map_model(provider: Provider, dialect: Dialect, model: &WireModel) -> ModelInfo {
    let live_caps = match dialect {
        Dialect::DeepSeek => deepseek_caps(model),
        Dialect::OpenRouter => router_caps(model),
        Dialect::Plain => ModelCaps::UNKNOWN,
    };
    let price = (dialect == Dialect::OpenRouter).then(|| router_price(model.pricing.as_ref()));
    let price = price.flatten();
    let context = model
        .context_window
        .as_ref()
        .and_then(|value| token_limit(Some(value)))
        .or_else(|| {
            model
                .context_length
                .as_ref()
                .and_then(|value| token_limit(Some(value)))
        });
    let output = model
        .max_output_tokens
        .as_ref()
        .and_then(|value| token_limit(Some(value)))
        .or_else(|| {
            model
                .top_provider
                .as_ref()
                .and_then(|provider| provider.max_completion_tokens.as_ref())
                .and_then(|value| token_limit(Some(value)))
        });
    details(
        provider,
        &model.id,
        LiveDetails {
            name: model.name.as_deref(),
            context,
            output,
            caps: live_caps,
        },
        price,
    )
}

fn deepseek_caps(model: &WireModel) -> ModelCaps {
    let images = model
        .input_modalities
        .as_ref()
        .map_or(Support::Unknown, |values| {
            if values.iter().any(|value| value == "image") {
                Support::Yes
            } else {
                Support::No
            }
        });
    ModelCaps {
        tools: Support::Yes,
        images,
        reasoning: support(model.effort.as_ref().map(|_| true)),
    }
}

fn router_caps(model: &WireModel) -> ModelCaps {
    let parameters = model.supported_parameters.as_ref();
    let tools = listed_support(parameters, &["tools"]);
    let reasoning = listed_support(
        parameters,
        &["reasoning", "include_reasoning", "reasoning_effort"],
    );
    let images = model
        .architecture
        .as_ref()
        .map(|architecture| listed_modalities(architecture.input_modalities.as_ref()));
    ModelCaps {
        tools,
        images: images.unwrap_or(Support::Unknown),
        reasoning,
    }
}

fn listed_support(values: Option<&Vec<String>>, wanted: &[&str]) -> Support {
    values.map_or(Support::Unknown, |values| {
        if wanted
            .iter()
            .any(|wanted| values.iter().any(|value| value == wanted))
        {
            Support::Yes
        } else {
            Support::No
        }
    })
}

fn listed_modalities(values: Option<&Vec<String>>) -> Support {
    values.map_or(Support::Unknown, |values| {
        if values.iter().any(|value| value == "image") {
            Support::Yes
        } else {
            Support::No
        }
    })
}

fn router_price(pricing: Option<&Pricing>) -> Option<ModelPrice> {
    let pricing = pricing?;
    Some(ModelPrice {
        input_micros: parse_usd_micros(pricing.prompt.as_deref()?, true)?,
        output_micros: parse_usd_micros(pricing.completion.as_deref()?, true)?,
        cache_read_micros: pricing
            .input_cache_read
            .as_deref()
            .and_then(|value| parse_usd_micros(value, true)),
        cache_write_micros: pricing
            .input_cache_write
            .as_deref()
            .and_then(|value| parse_usd_micros(value, true)),
        source: PriceSource::Provider,
    })
}

impl crate::ModelCatalog for super::OpenAiCompatProvider {
    fn list_url(&self) -> String {
        list_url(&self.config)
    }

    fn verify_url(&self) -> Option<String> {
        (self.config.dialect == Dialect::OpenRouter).then(|| key_url(&self.config))
    }

    fn verify_key(&self, cancel: CancellationToken) -> BoxFuture<'_, Result<(), ProviderError>> {
        Box::pin(verify(&self.client, &self.config, cancel))
    }

    fn list_models(
        &self,
        cancel: CancellationToken,
    ) -> BoxFuture<'_, Result<Vec<ModelInfo>, ProviderError>> {
        Box::pin(list(&self.client, &self.config, cancel))
    }
}
