//! Saved session settings for the platform.

use tether_store::{SessionRecord, SessionSearchHit};
use tether_types::{ModelInfo, ReasoningEffort, Support, UsageSnapshot};

use crate::{PermissionModeView, ProviderView, SessionStatusView};

/// Whether the saved catalog reports a model capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SupportView {
    /// Supported.
    Yes,
    /// Unsupported.
    No,
    /// The provider did not say.
    Unknown,
}

impl From<Support> for SupportView {
    fn from(value: Support) -> Self {
        match value {
            Support::Yes => Self::Yes,
            Support::No => Self::No,
            Support::Unknown => Self::Unknown,
        }
    }
}

/// One model from the saved provider catalog.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ModelView {
    /// Provider model id.
    pub id: String,
    /// Picker label.
    pub display_name: String,
    /// Context capacity in tokens, when known.
    pub context_window: Option<u32>,
    /// Output capacity in tokens, when known.
    pub max_output_tokens: Option<u32>,
    /// Input price in USD micros per million tokens, when known.
    pub input_price_micros_per_million: Option<u64>,
    /// Output price in USD micros per million tokens, when known.
    pub output_price_micros_per_million: Option<u64>,
    /// Tool support.
    pub tools: SupportView,
    /// Image input support.
    pub images: SupportView,
    /// Reasoning support.
    pub reasoning: SupportView,
}

impl From<ModelInfo> for ModelView {
    fn from(model: ModelInfo) -> Self {
        Self {
            id: model.id,
            display_name: model.display_name,
            context_window: model.context_window,
            max_output_tokens: model.max_output_tokens,
            input_price_micros_per_million: model.price.map(|price| price.input_micros),
            output_price_micros_per_million: model.price.map(|price| price.output_micros),
            tools: model.caps.tools.into(),
            images: model.caps.images.into(),
            reasoning: model.caps.reasoning.into(),
        }
    }
}

/// Accumulated usage for one session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct SessionUsageView {
    /// Input tokens.
    pub tokens_in: u64,
    /// Output tokens.
    pub tokens_out: u64,
    /// Cost in USD micros.
    pub cost_micros: u64,
    /// False when at least one response had no known model price.
    pub price_known: bool,
}

/// One section of the current system prompt.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PromptSectionView {
    /// Stable section name.
    pub name: String,
    /// Character count.
    pub chars: u32,
    /// Approximate token count.
    pub approx_tokens: u32,
}

impl From<UsageSnapshot> for SessionUsageView {
    fn from(usage: UsageSnapshot) -> Self {
        Self {
            tokens_in: usage.tokens_in,
            tokens_out: usage.tokens_out,
            cost_micros: usage.cost_micros,
            price_known: true,
        }
    }
}

/// A saved text part matching transcript search.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TranscriptHitView {
    /// Message id.
    pub message_id: String,
    /// Session id.
    pub session_id: String,
    /// Session title.
    pub session_title: String,
    /// Matched text.
    pub text: String,
}

impl From<SessionSearchHit> for TranscriptHitView {
    fn from(hit: SessionSearchHit) -> Self {
        Self {
            message_id: hit.message_id.into(),
            session_id: hit.session_id.into(),
            session_title: hit.session_title,
            text: hit.text,
        }
    }
}

/// FFI view of [`ReasoningEffort`]. `None` on a session means the model default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReasoningEffortView {
    /// Little reasoning, fast.
    Low,
    /// The default balance.
    Medium,
    /// Deep reasoning, slower and dearer.
    High,
}

impl From<ReasoningEffort> for ReasoningEffortView {
    fn from(value: ReasoningEffort) -> Self {
        match value {
            ReasoningEffort::Low => Self::Low,
            ReasoningEffort::Medium => Self::Medium,
            ReasoningEffort::High => Self::High,
        }
    }
}

impl From<ReasoningEffortView> for ReasoningEffort {
    fn from(view: ReasoningEffortView) -> Self {
        match view {
            ReasoningEffortView::Low => Self::Low,
            ReasoningEffortView::Medium => Self::Medium,
            ReasoningEffortView::High => Self::High,
        }
    }
}

/// Saved session settings and status for the platform.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SessionRecordView {
    /// Session id.
    pub id: String,
    /// Project id.
    pub project_id: String,
    /// Display title.
    pub title: String,
    /// Provider.
    pub provider: ProviderView,
    /// Provider model id.
    pub model_id: String,
    /// Saved permission mode.
    pub mode: PermissionModeView,
    /// Saved status.
    pub status: SessionStatusView,
    /// Creation time in Unix milliseconds.
    pub created_at_ms: i64,
    /// Time of the last change, in Unix milliseconds.
    pub updated_at_ms: i64,
    /// Saved reasoning effort. `None` means the model default.
    pub reasoning_effort: Option<ReasoningEffortView>,
}

impl From<SessionRecord> for SessionRecordView {
    fn from(session: SessionRecord) -> Self {
        Self {
            id: session.id.into(),
            project_id: session.project_id.into(),
            title: session.title,
            provider: session.provider.into(),
            model_id: session.model_id,
            mode: session.mode.into(),
            status: session.status.into(),
            created_at_ms: session.created_at,
            updated_at_ms: session.updated_at,
            reasoning_effort: session.reasoning_effort.map(ReasoningEffortView::from),
        }
    }
}
