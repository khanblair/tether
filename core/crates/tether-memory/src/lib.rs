//! Capped, file-backed agent and user memory.

mod files;

pub use files::{MemoryAction, MemoryError, MemoryResult, MemoryStore, MemoryTarget};
