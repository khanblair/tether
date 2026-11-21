//! Saved plan values.

/// A plan proposed by the model for the user to review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The short plan summary.
    pub summary: String,
    /// Ordered work steps.
    pub steps: Vec<String>,
    /// Workspace-relative files the plan expects to touch.
    pub files: Vec<String>,
    /// Commands the plan expects to run.
    pub commands: Vec<String>,
    /// Questions still open before the work starts.
    pub open_questions: Vec<String>,
}
