//! Shared types: ids, messages, tool calls, events, permissions, errors.
//!
//! P0 holds only the event type needed to prove the Rust to Kotlin event path.

mod event;

pub use event::CoreEvent;
