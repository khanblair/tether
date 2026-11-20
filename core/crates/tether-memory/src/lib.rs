//! Capped, file-backed agent and user memory.

mod files;
mod snapshot;

pub use files::{MemoryAction, MemoryError, MemoryResult, MemoryStore, MemoryTarget};
pub use snapshot::FrozenMemorySnapshot;
