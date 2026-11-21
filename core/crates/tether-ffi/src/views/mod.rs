//! Engine types converted to FFI view types.
//!
//! Views are what the platform layer sees: ids become plain strings and every
//! type derives UniFFI. Engine types never cross the boundary.

mod approval;
mod checkpoint;
mod context;
mod error;
mod event;
mod plan;
mod project;
mod question;
mod session;
mod todo;
mod types;
mod usage;

pub use approval::{PendingApprovalView, PermissionRuleView};
pub use checkpoint::{CheckpointView, RewindScopeView};
pub use context::{CompactionView, ContextBreakdownView, ContextSectionView, InterruptionView};
pub use error::{CoreErrorView, ErrorKindView};
pub use event::CoreEventView;
pub use plan::PlanView;
pub use project::ProjectView;
pub use question::PendingQuestionView;
pub use session::{
    ModelView, PromptSectionView, SessionRecordView, SessionUsageView, SupportView,
    TranscriptHitView,
};
pub use todo::{TodoItemView, TodoStatusView};
pub use types::{
    BudgetScopeView, ConnectionStatusView, ExecutorView, MessagePartView, MessageView,
    ModelRefView, PermissionModeView, ProviderView, RoleView, SessionStatusView,
    ToolCallStatusView, UsageSnapshotView,
};
pub use usage::{
    BudgetView, DayUsageView, ModelUsageView, ProjectUsageView, UsageSummaryView, UsageTotalsView,
};
