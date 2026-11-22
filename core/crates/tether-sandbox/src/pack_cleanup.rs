//! Cleanup helpers for failed pack replacement.

use std::fs;
use std::path::Path;

pub(crate) fn remove_empty_directory(path: &Path) {
    if fs::read_dir(path).is_ok_and(|mut entries| entries.next().is_none()) {
        drop(fs::remove_dir(path));
    }
}
