//! FFI views for session-owned background processes.

use super::ProcessStatusView;

/// FFI view of one session-owned background process.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProcessView {
    /// Short process identifier.
    pub id: String,
    /// Original command.
    pub command: String,
    /// Current process state.
    pub status: ProcessStatusView,
    /// Detected listening port, if any.
    pub port: Option<u16>,
    /// Start time in Unix milliseconds.
    pub started_at_ms: u64,
}
