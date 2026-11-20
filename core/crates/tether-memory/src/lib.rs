//! Memory: capped agent and user notes, a frozen snapshot, and an Obsidian-compatible vault.

mod files;
mod snapshot;
pub mod vault;

pub use files::{MemoryAction, MemoryError, MemoryResult, MemoryStore, MemoryTarget};
pub use snapshot::FrozenMemorySnapshot;

/// A name part that no other call in this process returns twice: the process id, the clock, and a
/// counter. The counter matters because two threads can read the same clock tick.
pub(crate) fn unique_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    format!(
        "{}-{nanos}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
