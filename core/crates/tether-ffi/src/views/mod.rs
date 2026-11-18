//! Engine types converted to FFI view types.
//!
//! Views are what the platform layer sees: ids become plain strings and every
//! type derives UniFFI. Engine types never cross the boundary.

mod error;
mod event;
mod types;

pub use error::{CoreErrorView, ErrorKindView};
pub use event::CoreEventView;
pub use types::{
    ConnectionStatusView, ExecutorView, MessagePartView, MessageView, ModelRefView,
    PermissionModeView, ProviderView, RoleView, SessionStatusView, ToolCallStatusView,
    UsageSnapshotView,
};
