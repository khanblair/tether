//! Shared model catalog contract and mapping helpers.

use tether_types::{ModelCaps, ModelInfo, ModelPrice, Provider, Support};
use tokio_util::sync::CancellationToken;

use crate::{BoxFuture, ProviderError};

/// Lists the chat models served by a provider.
pub trait ModelCatalog: Send + Sync {
    /// The URL of the first model-list request, exactly as it is sent. It never holds the key.
    /// It is empty when the base URL is not a valid URL, and `list_models` then fails.
    fn list_url(&self) -> String;

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
