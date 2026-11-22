//! Pending question values exposed to the platform.

/// A question waiting for an answer from the platform.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PendingQuestionView {
    /// The provider call waiting for an answer.
    pub tool_call_id: String,
    /// Question text.
    pub question: String,
    /// Tappable answer options.
    pub options: Vec<String>,
    /// Whether typed answers are accepted.
    pub allow_free_text: bool,
}
