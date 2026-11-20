//! Project workspace: path guard, file index, search, watcher.

mod file_index;
mod path_guard;

pub use file_index::{FileIndex, IndexedFile, SearchHit};
pub use path_guard::PathGuard;

#[cfg(test)]
mod file_index_tests;
#[cfg(test)]
mod path_guard_tests;
