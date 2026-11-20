//! Memory: capped agent and user notes, a frozen snapshot, and an Obsidian-compatible vault.

mod files;
mod snapshot;
pub mod vault;

pub use files::{MemoryAction, MemoryError, MemoryResult, MemoryStore, MemoryTarget};
pub use snapshot::FrozenMemorySnapshot;
