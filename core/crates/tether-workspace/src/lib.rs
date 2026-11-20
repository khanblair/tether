//! Project workspace: path guard, file index, search, watcher.

mod path_guard;

pub use path_guard::PathGuard;

#[cfg(test)]
mod path_guard_tests;
