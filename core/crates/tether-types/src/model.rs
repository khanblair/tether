//! Model catalog values shared by providers, the store, and the app.

use crate::session::Provider;
use crate::text::text_enum;

/// Whether a model has a feature. `Unknown` means the provider's list did not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    /// The model has the feature.
    Yes,
    /// The model does not have the feature.
    No,
    /// The provider did not report whether the model has the feature.
    Unknown,
}

text_enum!(Support {
    Yes => "yes",
    No => "no",
    Unknown => "unknown",
});

/// How much reasoning a model should spend, for models that support the setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningEffort {
    /// Little reasoning, fast.
    Low,
    /// The default balance.
    Medium,
    /// Deep reasoning, slower and dearer.
    High,
}

text_enum!(ReasoningEffort {
    Low => "low",
    Medium => "medium",
    High => "high",
});

/// Where a price came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceSource {
    /// The provider returned the price.
    Provider,
    /// Tether supplied the price from its bundled table.
    Bundled,
}

text_enum!(PriceSource {
    Provider => "provider",
    Bundled => "bundled",
});

/// The features the picker shows as badges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelCaps {
    /// Whether the model can call tools.
    pub tools: Support,
    /// Whether the model accepts images.
    pub images: Support,
    /// Whether the model supports reasoning output.
    pub reasoning: Support,
}

impl ModelCaps {
    /// Capabilities used when the provider did not report any features.
    pub const UNKNOWN: Self = Self {
        tools: Support::Unknown,
        images: Support::Unknown,
        reasoning: Support::Unknown,
    };
}

/// A price in USD micros per million tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelPrice {
    /// Input cost in USD micros per million tokens.
    pub input_micros: u64,
    /// Output cost in USD micros per million tokens.
    pub output_micros: u64,
    /// Cached input read cost in USD micros per million tokens.
    pub cache_read_micros: Option<u64>,
    /// Cached input write cost in USD micros per million tokens.
    pub cache_write_micros: Option<u64>,
    /// The source of this price.
    pub source: PriceSource,
}

/// One model in a provider's catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    /// The provider that serves the model.
    pub provider: Provider,
    /// The provider's model identifier.
    pub id: String,
    /// A name suitable for display in the picker.
    pub display_name: String,
    /// Maximum context size in tokens, when known.
    pub context_window: Option<u32>,
    /// Maximum generated output size in tokens, when known.
    pub max_output_tokens: Option<u32>,
    /// The model's price, when known.
    pub price: Option<ModelPrice>,
    /// Features shown as picker badges.
    pub caps: ModelCaps,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_text_forms_round_trip() {
        for &value in Support::ALL {
            assert_eq!(value.as_str().parse::<Support>().unwrap(), value);
        }
        assert!("maybe".parse::<Support>().is_err());
    }

    #[test]
    fn price_source_text_forms_round_trip() {
        for &value in PriceSource::ALL {
            assert_eq!(value.as_str().parse::<PriceSource>().unwrap(), value);
        }
        assert!("unknown".parse::<PriceSource>().is_err());
    }

    #[test]
    fn text_forms_are_stable() {
        assert_eq!(
            Support::ALL.iter().map(|v| v.as_str()).collect::<Vec<_>>(),
            ["yes", "no", "unknown"]
        );
        assert_eq!(
            PriceSource::ALL
                .iter()
                .map(|v| v.as_str())
                .collect::<Vec<_>>(),
            ["provider", "bundled"]
        );
    }

    #[test]
    fn unknown_capabilities_are_all_unknown() {
        assert_eq!(ModelCaps::UNKNOWN.tools, Support::Unknown);
        assert_eq!(ModelCaps::UNKNOWN.images, Support::Unknown);
        assert_eq!(ModelCaps::UNKNOWN.reasoning, Support::Unknown);
    }

    #[test]
    fn reasoning_effort_text_round_trips() {
        assert_eq!(
            ReasoningEffort::ALL
                .iter()
                .map(|v| v.as_str())
                .collect::<Vec<_>>(),
            ["low", "medium", "high"]
        );
        for &value in ReasoningEffort::ALL {
            assert_eq!(value.as_str().parse::<ReasoningEffort>().unwrap(), value);
        }
    }

    #[test]
    fn reasoning_effort_rejects_unknown_text() {
        let error = "extreme".parse::<ReasoningEffort>().unwrap_err();
        assert_eq!(error.kind, "ReasoningEffort");
        assert_eq!(error.value, "extreme");
        assert!("".parse::<ReasoningEffort>().is_err());
        assert!("High".parse::<ReasoningEffort>().is_err());
    }
}
