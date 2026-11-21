//! Engine types converted to FFI view types.
//!
//! Views are what the platform layer sees: ids become plain strings and every
//! type derives UniFFI. Engine types never cross the boundary.

mod checkpoint;
mod error;
mod event;
mod project;
mod session;
mod types;

pub use checkpoint::{CheckpointView, RewindScopeView};
pub use error::{CoreErrorView, ErrorKindView};
pub use event::CoreEventView;
pub use project::ProjectView;
pub use session::SessionRecordView;
pub use types::{
    ConnectionStatusView, ExecutorView, MessagePartView, MessageView, ModelRefView,
    PermissionModeView, ProviderView, RoleView, SessionStatusView, ToolCallStatusView,
    UsageSnapshotView,
};
