//! Context and interruption views for the platform.

use tether_engine::context::{ContextBreakdown, ContextSection};
use tether_store::CompactionRecord;

/// One context section and its approximate size.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ContextSectionView {
    /// Section name.
    pub name: String,
    /// Approximate token count.
    pub approx_tokens: u32,
}

impl From<ContextSection> for ContextSectionView {
    fn from(section: ContextSection) -> Self {
        Self {
            name: section.name,
            approx_tokens: section.approx_tokens,
        }
    }
}

/// Current context use for one session.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ContextBreakdownView {
    /// Context capacity when known.
    pub window: Option<u32>,
    /// Last reported or estimated input tokens.
    pub used_tokens: u32,
    /// Context percentage when the window is known.
    pub percent: Option<u8>,
    /// Prompt and conversation sections.
    pub sections: Vec<ContextSectionView>,
}

impl From<ContextBreakdown> for ContextBreakdownView {
    fn from(value: ContextBreakdown) -> Self {
        Self {
            window: value.window,
            used_tokens: value.used_tokens,
            percent: value.percent,
            sections: value.sections.into_iter().map(Into::into).collect(),
        }
    }
}

/// A compaction marker in the transcript.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CompactionView {
    /// Marker id.
    pub id: String,
    /// Summary message id.
    pub summary_message_id: String,
    /// First replaced message id.
    pub first_message_id: String,
    /// Last replaced message id.
    pub last_message_id: String,
    /// Tokens before compaction.
    pub tokens_before: u32,
    /// Tokens after compaction.
    pub tokens_after: u32,
    /// Creation time in Unix milliseconds.
    pub created_at_ms: i64,
    /// Optional focus supplied by the user.
    pub focus: Option<String>,
}

impl From<CompactionRecord> for CompactionView {
    fn from(value: CompactionRecord) -> Self {
        Self {
            id: value.id,
            summary_message_id: value.summary_message_id.into(),
            first_message_id: value.first_message_id.into(),
            last_message_id: value.last_message_id.into(),
            tokens_before: value.tokens_before,
            tokens_after: value.tokens_after,
            created_at_ms: value.created_at,
            focus: value.focus,
        }
    }
}

/// The operation a killed session was performing.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct InterruptionView {
    /// Human-readable step.
    pub step: String,
    /// Time of the saved session update.
    pub at_ms: i64,
}
