//! Shared types: ids, messages, tool calls, events, permissions, errors.
//!
//! These are the engine's own types. They carry no UniFFI code; `tether-ffi`
//! turns the ones the UI sees into view types.

mod error;
mod event;
mod ids;
mod message;
mod model;
mod session;
mod text;
mod usage;

pub use error::CoreError;
pub use event::CoreEvent;
pub use ids::{MessageId, ProjectId, SessionId, ToolCallId};
pub use message::{Message, MessagePart, Role};
pub use model::{ModelCaps, ModelInfo, ModelPrice, PriceSource, Support};
pub use session::{
    ConnectionStatus, Executor, ModelRef, PermissionMode, Provider, SessionStatus, ToolCallStatus,
};
pub use text::UnknownValue;
pub use usage::UsageSnapshot;
