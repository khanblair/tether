use std::collections::HashSet;

use reqwest::header::HeaderMap;
use serde::Deserialize;
use tether_types::{ModelCaps, ModelInfo, Provider};
use tokio_util::sync::CancellationToken;

use super::{GeminiConfig, http};
use crate::catalog::{LiveDetails, details, support, token_limit};
use crate::transport::{GetCall, get_json, secret_header};
use crate::{BoxFuture, ProviderError};

#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    models: Vec<Model>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct Model {
    name: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "inputTokenLimit")]
    input: Option<serde_json::Value>,
    #[serde(rename = "outputTokenLimit")]
    output: Option<serde_json::Value>,
    #[serde(rename = "supportedGenerationMethods", default)]
    methods: Vec<String>,
    thinking: Option<bool>,
}

pub(super) async fn list(
    client: &reqwest::Client,
    config: &GeminiConfig,
    cancel: CancellationToken,
) -> Result<Vec<ModelInfo>, ProviderError> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    let mut token = None;
    for _ in 0..10 {
        let page = get_page(client, config, token.as_deref(), &cancel).await?;
        for model in &page.models {
            add_model(&mut result, &mut seen, model);
        }
        token = page.next_page_token;
        if token.is_none() {
            break;
        }
    }
    Ok(result)
}

async fn get_page(
    client: &reqwest::Client,
    config: &GeminiConfig,
    token: Option<&str>,
    cancel: &CancellationToken,
) -> Result<Page, ProviderError> {
    let mut headers = HeaderMap::new();
    headers.insert("x-goog-api-key", secret_header(&config.api_key)?);
    let mut url = reqwest::Url::parse(&format!(
        "{}/v1beta/models",
        config.base_url.trim_end_matches('/')
    ))
    .map_err(|_| crate::ProviderError::InvalidRequest {
        message: "the model catalog URL is invalid".to_owned(),
    })?;
    url.query_pairs_mut().append_pair("pageSize", "1000");
    if let Some(token) = token {
        url.query_pairs_mut().append_pair("pageToken", token);
    }
    get_json(
        GetCall {
            client,
            url: url.to_string(),
            headers,
            errors: &http::GeminiErrors,
        },
        cancel,
    )
    .await
}

fn add_model(result: &mut Vec<ModelInfo>, seen: &mut HashSet<String>, model: &Model) {
    let Some(id) = model.name.strip_prefix("models/") else {
        return;
    };
    if !model
        .methods
        .iter()
        .any(|method| method == "generateContent")
        || !seen.insert(id.to_owned())
    {
        return;
    }
    let caps = ModelCaps {
        tools: tether_types::Support::Unknown,
        images: tether_types::Support::Unknown,
        reasoning: support(model.thinking),
    };
    result.push(details(
        Provider::Gemini,
        id,
        LiveDetails {
            name: model.display_name.as_deref(),
            context: token_limit(model.input.as_ref()),
            output: token_limit(model.output.as_ref()),
            caps,
        },
        None,
    ));
}

impl crate::ModelCatalog for super::GeminiProvider {
    fn list_models(
        &self,
        cancel: CancellationToken,
    ) -> BoxFuture<'_, Result<Vec<ModelInfo>, ProviderError>> {
        Box::pin(list(&self.client, &self.config, cancel))
    }
}
