/// One `run` tool call and how it ended.
#[derive(Debug, Clone, uniffi::Record)]
pub struct RunHistoryView {
    /// Provider call identifier.
    pub call_id: String,
    /// Session that owns the call.
    pub session_id: String,
    /// Saved display title of the session.
    pub session_title: String,
    /// The command line, cut to 500 characters on a character boundary.
    pub command: String,
    /// Executor from the result or call arguments.
    pub executor: Option<super::types::ExecutorView>,
    /// How the call ended.
    pub outcome: RunOutcomeView,
    /// Exit code when the outcome is `Succeeded` or `Failed`.
    pub exit_code: Option<i32>,
    /// Foreground duration in milliseconds.
    pub duration_ms: Option<u64>,
    /// Call message creation time in Unix milliseconds.
    pub started_at_ms: i64,
    /// True for a Linux background run.
    pub background: bool,
}

/// The terminal state shown for one saved `run` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RunOutcomeView {
    /// The command exited with code zero.
    Succeeded,
    /// The command exited with a nonzero code.
    Failed,
    /// The executor stopped the command at its time limit.
    TimedOut,
    /// The caller cancelled the command.
    Cancelled,
    /// The command exceeded its memory limit.
    OutOfMemory,
    /// The user denied the call before execution.
    Denied,
    /// Linux started a background process.
    Background,
    /// No known result is available.
    Unknown,
}
