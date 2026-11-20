//! Copy-based sync contract for a private vault and a platform shared folder.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Component, Path};

/// Folder access supplied by the platform, including Android's document tree adapter.
pub trait VaultFolder {
    /// Lists relative Markdown paths.
    ///
    /// # Errors
    /// Returns the platform error when the folder cannot be listed.
    fn list(&self) -> io::Result<Vec<String>>;
    /// Reads a file, or returns `None` when it is absent.
    ///
    /// # Errors
    /// Returns the platform error when the file exists but cannot be read.
    fn read(&self, path: &str) -> io::Result<Option<Vec<u8>>>;
    /// Writes a complete file.
    ///
    /// # Errors
    /// Returns the platform error when the file cannot be written.
    fn write(&self, path: &str, bytes: &[u8]) -> io::Result<()>;
}

/// The bytes seen on the private and shared side for one path. `None` means absent.
type Observation = (Option<Vec<u8>>, Option<Vec<u8>>);

/// Last observed contents on both sides, retained between sync calls.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncManifest {
    observed: BTreeMap<String, Observation>,
}

/// Changes made by one sync call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Paths copied into the private vault.
    pub copied_to_private: Vec<String>,
    /// Paths copied into the shared folder.
    pub copied_to_shared: Vec<String>,
    /// Original paths with two different edits.
    pub conflicts: Vec<String>,
}

/// Fails when a folder reports an unsafe path or I/O fails.
#[derive(Debug)]
pub enum SyncError {
    /// A platform read or write failed.
    Io(io::Error),
    /// A listed path could escape the vault or touch a reserved folder.
    InvalidPath(String),
}

impl From<io::Error> for SyncError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Copies one-sided edits and keeps both versions of a conflict.
///
/// # Errors
/// Returns the first platform error or unsafe path. No deletion is propagated.
pub fn sync_folders(
    private: &dyn VaultFolder,
    shared: &dyn VaultFolder,
    manifest: &mut SyncManifest,
) -> Result<SyncReport, SyncError> {
    let mut paths = BTreeSet::new();
    for path in private.list()?.into_iter().chain(shared.list()?) {
        validate_path(&path)?;
        paths.insert(path);
    }
    let mut report = SyncReport::default();
    for path in paths {
        sync_path(private, shared, manifest, &mut report, path)?;
    }
    Ok(report)
}

fn sync_path(
    private: &dyn VaultFolder,
    shared: &dyn VaultFolder,
    manifest: &mut SyncManifest,
    report: &mut SyncReport,
    path: String,
) -> Result<(), SyncError> {
    let left = private.read(&path)?;
    let right = shared.read(&path)?;
    let previous = manifest.observed.get(&path);
    if left == right {
        manifest.observed.insert(path, (left, right));
        return Ok(());
    }
    let (left_changed, right_changed) = previous.map_or((true, true), |(old_left, old_right)| {
        (left != *old_left, right != *old_right)
    });
    match (&left, &right) {
        (Some(bytes), None) => {
            shared.write(&path, bytes)?;
            report.copied_to_shared.push(path.clone());
            manifest.observed.insert(path, (left.clone(), left));
        }
        (None, Some(bytes)) => {
            private.write(&path, bytes)?;
            report.copied_to_private.push(path.clone());
            manifest.observed.insert(path, (right.clone(), right));
        }
        (Some(left_bytes), Some(_right_bytes)) if left_changed && !right_changed => {
            shared.write(&path, left_bytes)?;
            report.copied_to_shared.push(path.clone());
            manifest.observed.insert(path, (left.clone(), left));
        }
        (Some(_left_bytes), Some(right_bytes)) if right_changed && !left_changed => {
            private.write(&path, right_bytes)?;
            report.copied_to_private.push(path.clone());
            manifest.observed.insert(path, (right.clone(), right));
        }
        (Some(left_bytes), Some(right_bytes)) if left_changed && right_changed => {
            let local_copy = conflict_path(&path, "private");
            let shared_copy = conflict_path(&path, "shared");
            shared.write(&unique_path(shared, &local_copy)?, left_bytes)?;
            private.write(&unique_path(private, &shared_copy)?, right_bytes)?;
            report.conflicts.push(path.clone());
            manifest.observed.insert(path, (left, right));
        }
        _ => {
            manifest.observed.insert(path, (left, right));
        }
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<(), SyncError> {
    let value = Path::new(path);
    if value.extension().and_then(|ext| ext.to_str()) != Some("md")
        || path.starts_with('.')
        || path.contains("/.obsidian/")
        || !value
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(SyncError::InvalidPath(path.to_owned()));
    }
    Ok(())
}

fn conflict_path(path: &str, source: &str) -> String {
    format!("{}.conflict-{source}.md", path.trim_end_matches(".md"))
}

fn unique_path(folder: &dyn VaultFolder, base: &str) -> Result<String, SyncError> {
    if folder.read(base)?.is_none() {
        return Ok(base.to_owned());
    }
    for number in 2..=100 {
        let candidate = format!(
            "{}.md",
            base.trim_end_matches(".md").to_owned() + &number.to_string()
        );
        if folder.read(&candidate)?.is_none() {
            return Ok(candidate);
        }
    }
    Err(SyncError::Io(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "conflict copy limit",
    )))
}

#[cfg(test)]
mod tests;
