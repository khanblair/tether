use std::collections::HashSet;

use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use tether_types::{ModelCaps, ModelInfo, Provider};
use tokio_util::sync::CancellationToken;

use super::{AnthropicConfig, http};
use crate::ProviderError;
use crate::catalog::{LiveDetails, details, support, token_limit};
use crate::transport::{GetCall, get_json, secret_header};

#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    data: Vec<Model>,
    #[serde(default)]
    has_more: bool,
    last_id: Option<String>,
}

#[derive(Deserialize)]
struct Model {
    id: String,
    display_name: Option<String>,
    max_input_tokens: Option<serde_json::Value>,
    max_tokens: Option<serde_json::Value>,
    lifecycle: Option<String>,
    capabilities: Option<Capabilities>,
}

#[derive(Deserialize)]
struct Capabilities {
    image_input: Option<Flag>,
    thinking: Option<Flag>,
}

#[derive(Deserialize)]
struct Flag {
    supported: Option<bool>,
}

pub(super) async fn list(
    client: &reqwest::Client,
    config: &AnthropicConfig,
    cancel: CancellationToken,
) -> Result<Vec<ModelInfo>, ProviderError> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    let mut after = None;
    for _ in 0..10 {
        let page = get_page(client, config, after.as_deref(), &cancel).await?;
        for model in &page.data {
            add_model(&mut result, &mut seen, model);
        }
        if !page.has_more {
            break;
        }
        after = page.last_id;
        if after.is_none() {
            break;
        }
    }
    Ok(result)
}

async fn get_page(
    client: &reqwest::Client,
    config: &AnthropicConfig,
    after: Option<&str>,
    cancel: &CancellationToken,
) -> Result<Page, ProviderError> {
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", secret_header(&config.api_key)?);
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    let mut url = reqwest::Url::parse(&format!(
        "{}/v1/models",
        config.base_url.trim_end_matches('/')
    ))
    .map_err(|_| ProviderError::InvalidRequest {
        message: "the model catalog URL is invalid".to_owned(),
    })?;
    url.query_pairs_mut().append_pair("limit", "1000");
    if let Some(id) = after {
        url.query_pairs_mut().append_pair("after_id", id);
    }
    get_json(
        GetCall {
            client,
            url: url.to_string(),
            headers,
            errors: &http::AnthropicErrors,
        },
        cancel,
    )
    .await
}

fn add_model(result: &mut Vec<ModelInfo>, seen: &mut HashSet<String>, model: &Model) {
    if model.lifecycle.as_deref() == Some("retired") || !seen.insert(model.id.clone()) {
        return;
    }
    let caps = ModelCaps {
        tools: tether_types::Support::Yes,
        images: support(
            model
                .capabilities
                .as_ref()
                .and_then(|c| c.image_input.as_ref())
                .and_then(|f| f.supported),
        ),
        reasoning: support(
            model
                .capabilities
                .as_ref()
                .and_then(|c| c.thinking.as_ref())
                .and_then(|f| f.supported),
        ),
    };
    result.push(details(
        Provider::Anthropic,
        &model.id,
        LiveDetails {
            name: model.display_name.as_deref(),
            context: token_limit(model.max_input_tokens.as_ref()),
            output: token_limit(model.max_tokens.as_ref()),
            caps,
        },
        None,
    ));
}
