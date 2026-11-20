//! Project workspace: path guard, file index, search, watcher.

mod path_guard;
mod file_index;

pub use path_guard::PathGuard;
pub use file_index::{FileIndex, IndexedFile, SearchHit};

#[cfg(test)]
mod path_guard_tests;
#[cfg(test)]
mod file_index_tests;
