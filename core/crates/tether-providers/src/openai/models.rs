use std::collections::HashSet;

use reqwest::header::{AUTHORIZATION, HeaderMap};
use serde::Deserialize;
use tether_types::{ModelCaps, ModelInfo, Provider};
use tokio_util::sync::CancellationToken;

use super::{OpenAiConfig, http};
use crate::catalog::{LiveDetails, details};
use crate::transport::{GetCall, get_json, secret_header};
use crate::{BoxFuture, ProviderError};

#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    data: Vec<Model>,
}

#[derive(Deserialize)]
struct Model {
    id: String,
}

pub(super) async fn list(
    client: &reqwest::Client,
    config: &OpenAiConfig,
    cancel: CancellationToken,
) -> Result<Vec<ModelInfo>, ProviderError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        secret_header(&format!("Bearer {}", config.api_key))?,
    );
    let page: Page = get_json(
        GetCall {
            client,
            url: format!("{}/v1/models", config.base_url.trim_end_matches('/')),
            headers,
            errors: &http::OpenAiErrors,
        },
        &cancel,
    )
    .await?;
    let mut seen = HashSet::new();
    Ok(page
        .data
        .into_iter()
        .filter_map(|model| {
            if !is_chat_model(&model.id) || !seen.insert(model.id.clone()) {
                return None;
            }
            Some(details(
                Provider::OpenAi,
                &model.id,
                LiveDetails {
                    name: None,
                    context: None,
                    output: None,
                    caps: ModelCaps::UNKNOWN,
                },
                None,
            ))
        })
        .collect())
}

fn is_chat_model(id: &str) -> bool {
    const BLOCKED: [&str; 11] = [
        "embedding",
        "whisper",
        "tts",
        "dall-e",
        "moderation",
        "image",
        "audio",
        "realtime",
        "transcribe",
        "instruct",
        "search",
    ];
    let prefix = id.starts_with("gpt-") && !id.starts_with("gpt-image")
        || id.starts_with("chatgpt-")
        || (id.as_bytes().first() == Some(&b'o')
            && id.as_bytes().get(1).is_some_and(u8::is_ascii_digit));
    prefix && BLOCKED.iter().all(|word| !id.contains(word))
}

impl crate::ModelCatalog for super::OpenAiProvider {
    fn list_models(
        &self,
        cancel: CancellationToken,
    ) -> BoxFuture<'_, Result<Vec<ModelInfo>, ProviderError>> {
        Box::pin(list(&self.client, &self.config, cancel))
    }
}
