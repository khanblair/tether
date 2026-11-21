//! Plan values exposed to the platform.

use tether_types::Plan;

/// The FFI view of a submitted plan.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PlanView {
    /// Short plan summary.
    pub summary: String,
    /// Ordered work steps.
    pub steps: Vec<String>,
    /// Workspace-relative files.
    pub files: Vec<String>,
    /// Expected commands.
    pub commands: Vec<String>,
    /// Questions still open.
    pub open_questions: Vec<String>,
}

impl From<Plan> for PlanView {
    fn from(plan: Plan) -> Self {
        Self {
            summary: plan.summary,
            steps: plan.steps,
            files: plan.files,
            commands: plan.commands,
            open_questions: plan.open_questions,
        }
    }
}
