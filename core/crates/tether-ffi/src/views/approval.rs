//! Approval and permission rule views for the platform.

use tether_types::PermissionRule;

use super::ExecutorView;

/// One approval waiting for the user.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PendingApprovalView {
    /// Provider call identifier.
    pub tool_call_id: String,
    /// Tool name.
    pub tool: String,
    /// Guarded workspace-relative target.
    pub target: String,
}

/// One project-scoped Always allow rule.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PermissionRuleView {
    /// Stable rule id.
    pub id: String,
    /// Tool name.
    pub tool: String,
    /// Exact target or conservative path prefix.
    pub pattern: String,
    /// Executor covered by the rule.
    pub executor: Option<ExecutorView>,
}

impl From<PermissionRule> for PermissionRuleView {
    fn from(rule: PermissionRule) -> Self {
        Self {
            id: rule.id,
            tool: rule.tool,
            pattern: rule.pattern,
            executor: rule.executor.map(Into::into),
        }
    }
}
