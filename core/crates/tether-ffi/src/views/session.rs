//! Saved session settings for the platform.

use tether_store::SessionRecord;

use crate::{ProviderView, SessionStatusView};

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
    /// Saved status.
    pub status: SessionStatusView,
}

impl From<SessionRecord> for SessionRecordView {
    fn from(session: SessionRecord) -> Self {
        Self {
            id: session.id.into(),
            project_id: session.project_id.into(),
            title: session.title,
            provider: session.provider.into(),
            model_id: session.model_id,
            status: session.status.into(),
        }
    }
}
