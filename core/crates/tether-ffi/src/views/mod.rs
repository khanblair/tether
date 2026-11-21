//! Engine types converted to FFI view types.
//!
//! Views are what the platform layer sees: ids become plain strings and every
//! type derives UniFFI. Engine types never cross the boundary.

mod approval;
mod checkpoint;
mod error;
mod event;
mod project;
mod session;
mod types;

pub use approval::{PendingApprovalView, PermissionRuleView};
pub use checkpoint::{CheckpointView, RewindScopeView};
pub use error::{CoreErrorView, ErrorKindView};
pub use event::CoreEventView;
pub use project::ProjectView;
pub use session::{ModelView, SessionRecordView, SessionUsageView, SupportView, TranscriptHitView};
pub use types::{
    ConnectionStatusView, ExecutorView, MessagePartView, MessageView, ModelRefView,
    PermissionModeView, ProviderView, RoleView, SessionStatusView, ToolCallStatusView,
    UsageSnapshotView,
};
