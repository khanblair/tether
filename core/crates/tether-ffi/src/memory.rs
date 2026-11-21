//! Capped memory files and session snapshot for the platform.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use tether_memory::{FrozenMemorySnapshot, MemoryAction, MemoryError, MemoryStore, MemoryTarget};

/// Selects the agent notes or user preferences file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MemoryTargetView {
    /// Agent notes in `MEMORY.md`.
    Memory,
    /// User preferences in `USER.md`.
    User,
}

/// An exact change to one memory file.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum MemoryActionView {
    /// Adds a distinct entry.
    Add {
        /// New entry text.
        entry: String,
    },
    /// Replaces one exact entry.
    Replace {
        /// Existing entry text.
        old: String,
        /// Replacement entry text.
        new: String,
    },
    /// Removes one exact entry.
    Remove {
        /// Existing entry text.
        entry: String,
    },
}

/// The saved content and character meter after an action.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MemoryResultView {
    /// Whether the file changed.
    pub changed: bool,
    /// Complete current content.
    pub content: String,
    /// Number of characters used.
    pub used: u32,
    /// Maximum number of characters.
    pub cap: u32,
}

/// A memory operation failed.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MemoryRuntimeError {
    /// The supplied directory is empty or contains a parent component.
    #[error("invalid memory path")]
    InvalidPath,
    /// The exact entry does not exist.
    #[error("memory entry not found")]
    NotFound,
    /// The result would exceed its character cap.
    #[error("memory exceeds its character cap")]
    OverCap,
    /// The entry has an invalid format.
    #[error("invalid memory entry")]
    InvalidEntry,
    /// The entry has blocked content.
    #[error("unsafe memory entry")]
    UnsafeContent,
    /// Storage cannot be read or written.
    #[error("memory is unavailable")]
    Unavailable,
}

/// Reads and edits the two memory files in one directory.
#[derive(uniffi::Object)]
pub struct MemoryRuntime {
    root: PathBuf,
    store: MemoryStore,
}

#[uniffi::export]
impl MemoryRuntime {
    /// Opens a directory and creates it when missing.
    ///
    /// # Errors
    /// Returns `InvalidPath` for an empty or parent path, or `Unavailable` for I/O failure.
    #[uniffi::constructor]
    pub fn new(memory_dir: &str) -> Result<Arc<Self>, MemoryRuntimeError> {
        let path = checked_directory(memory_dir).ok_or(MemoryRuntimeError::InvalidPath)?;
        std::fs::create_dir_all(path).map_err(|_| MemoryRuntimeError::Unavailable)?;
        Ok(Arc::new(Self {
            root: path.to_path_buf(),
            store: MemoryStore::new(path),
        }))
    }

    /// Reads one memory file. A missing file is empty.
    #[must_use]
    pub fn read(&self, target: MemoryTargetView) -> String {
        if !self.files_safe() {
            return String::new();
        }
        self.store.read(target.into()).unwrap_or_default()
    }

    /// Applies one exact memory change and returns the character meter.
    ///
    /// # Errors
    /// Returns a cap, content, missing entry, or storage error.
    pub fn apply(
        &self,
        target: MemoryTargetView,
        action: MemoryActionView,
    ) -> Result<MemoryResultView, MemoryRuntimeError> {
        if !self.files_safe() {
            return Err(MemoryRuntimeError::InvalidPath);
        }
        let result = self
            .store
            .apply(target.into(), action.into())
            .map_err(|error| map_error(&error))?;
        let cap = match target {
            MemoryTargetView::Memory => 2_200,
            MemoryTargetView::User => 1_375,
        };
        Ok(MemoryResultView {
            changed: result.changed,
            used: u32::try_from(result.content.chars().count()).unwrap_or(u32::MAX),
            content: result.content,
            cap,
        })
    }

    /// Formats the current files as an untrusted session snapshot block.
    #[must_use]
    pub fn frozen_block(&self) -> String {
        if !self.files_safe() {
            return String::new();
        }
        FrozenMemorySnapshot::load(&self.store)
            .map_or_else(|_| String::new(), |snapshot| snapshot.system_block())
    }
}

impl MemoryRuntime {
    fn files_safe(&self) -> bool {
        ["MEMORY.md", "USER.md"].iter().all(|name| {
            !std::fs::symlink_metadata(self.root.join(name))
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
        })
    }
}

impl From<MemoryTargetView> for MemoryTarget {
    fn from(value: MemoryTargetView) -> Self {
        match value {
            MemoryTargetView::Memory => Self::Memory,
            MemoryTargetView::User => Self::User,
        }
    }
}

impl From<MemoryActionView> for MemoryAction {
    fn from(value: MemoryActionView) -> Self {
        match value {
            MemoryActionView::Add { entry } => Self::Add(entry),
            MemoryActionView::Replace { old, new } => Self::Patch { old, new },
            MemoryActionView::Remove { entry } => Self::Remove(entry),
        }
    }
}

fn map_error(error: &MemoryError) -> MemoryRuntimeError {
    match error {
        MemoryError::Io(_) => MemoryRuntimeError::Unavailable,
        MemoryError::MissingEntry => MemoryRuntimeError::NotFound,
        MemoryError::OverCap { .. } => MemoryRuntimeError::OverCap,
        MemoryError::UnsafeContent => MemoryRuntimeError::UnsafeContent,
        MemoryError::InvalidEntry => MemoryRuntimeError::InvalidEntry,
    }
}

/// Accepts a directory without parent components or a symlink at its root.
pub(crate) fn checked_directory(path: &str) -> Option<&Path> {
    let value = Path::new(path);
    let valid = !path.is_empty()
        && !value.components().any(|part| part == Component::ParentDir)
        && !std::fs::symlink_metadata(value)
            .is_ok_and(|metadata| metadata.file_type().is_symlink());
    valid.then_some(value)
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
