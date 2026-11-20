//! Saved checkpoint and rewind choices exposed to the platform.

use tether_engine::RewindScope;
use tether_store::Checkpoint;

/// One saved edit point in a session transcript.
#[derive(Debug, Clone, uniffi::Record)]
pub struct CheckpointView {
    /// Checkpoint id used for rewind.
    pub id: String,
    /// The assistant message that began the edit batch.
    pub message_id: Option<String>,
    /// Time in Unix milliseconds.
    pub created_at_ms: i64,
    /// Workspace-relative paths captured by this batch.
    pub paths: Vec<String>,
}

impl From<Checkpoint> for CheckpointView {
    fn from(checkpoint: Checkpoint) -> Self {
        Self {
            id: checkpoint.id,
            message_id: checkpoint.message_id.map(Into::into),
            created_at_ms: checkpoint.created_at_ms,
            paths: checkpoint
                .files
                .into_iter()
                .map(|file| file.path.to_string_lossy().into_owned())
                .collect(),
        }
    }
}

/// Part of a session to restore.
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum RewindScopeView {
    /// Workspace files only.
    Files,
    /// Conversation context only.
    Conversation,
    /// Files and conversation together.
    Both,
}

impl From<RewindScopeView> for RewindScope {
    fn from(scope: RewindScopeView) -> Self {
        match scope {
            RewindScopeView::Files => Self::Files,
            RewindScopeView::Conversation => Self::Conversation,
            RewindScopeView::Both => Self::Both,
        }
    }
}
