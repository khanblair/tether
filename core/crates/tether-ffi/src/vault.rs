//! Private Markdown vault operations for the platform.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use tether_memory::vault::{NoteLinks, VaultError, VaultLink, VaultNote, VaultStore};

use crate::memory::checked_directory;
use crate::{vault_archive, vault_sync};

/// Controls whether vault import merges into or replaces an empty vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ImportMode {
    /// Keep existing files and save changed incoming notes as conflict copies.
    Merge,
    /// Import only when the vault contains no notes or attachments.
    ReplaceEmpty,
}

/// Counts the files handled by a vault import.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Record)]
pub struct VaultImportReportView {
    /// Number of new files written.
    pub imported: u64,
    /// Number of identical or unsupported files skipped.
    pub skipped: u64,
    /// Number of changed files saved beside an existing file.
    pub conflicts: u64,
}

/// A shared-folder platform operation failed.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum VaultFolderPlatformError {
    /// The platform could not complete the operation.
    #[error("vault folder operation failed")]
    Failed,
}

/// Provides access to a user-selected shared vault folder.
#[uniffi::export(with_foreign)]
pub trait VaultFolderPlatform: Send + Sync {
    /// Lists relative Markdown paths in the shared folder.
    ///
    /// # Errors
    /// Returns a platform error when listing fails.
    fn list(&self) -> Result<Vec<String>, VaultFolderPlatformError>;
    /// Reads a shared Markdown file, or returns `None` when it is absent.
    ///
    /// # Errors
    /// Returns a platform error when reading fails.
    fn read(&self, path: String) -> Result<Option<Vec<u8>>, VaultFolderPlatformError>;
    /// Writes one complete shared Markdown file.
    ///
    /// # Errors
    /// Returns a platform error when writing fails.
    fn write(&self, path: String, bytes: Vec<u8>) -> Result<(), VaultFolderPlatformError>;
}

/// Changes made by a shared-folder sync.
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct VaultSyncReportView {
    /// Paths copied into the private vault.
    pub copied_to_private: Vec<String>,
    /// Paths copied into the shared folder.
    pub copied_to_shared: Vec<String>,
    /// Original paths with two different edits.
    pub conflicts: Vec<String>,
}

/// A parsed vault note and its original Markdown.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VaultNoteView {
    /// Relative Markdown path.
    pub path: String,
    /// Title from frontmatter or the filename.
    pub title: String,
    /// Required description.
    pub description: String,
    /// Optional note category.
    pub note_type: Option<String>,
    /// Tags from frontmatter.
    pub tags: Vec<String>,
    /// Aliases from frontmatter.
    pub aliases: Vec<String>,
    /// Optional creation date.
    pub created: Option<String>,
    /// Optional update date.
    pub updated: Option<String>,
    /// Markdown headings.
    pub headings: Vec<String>,
    /// Complete Markdown source.
    pub source: String,
}

/// A wikilink in a vault note.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VaultLinkView {
    /// Relative source note path.
    pub from: String,
    /// Target text as written in the link.
    pub target: String,
    /// Optional target heading.
    pub heading: Option<String>,
    /// Optional display alias.
    pub alias: Option<String>,
    /// Whether the target exists.
    pub resolved: bool,
    /// Relative path of the target note, when resolved.
    pub resolved_path: Option<String>,
}

/// Outgoing and incoming links for one note.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VaultLinksView {
    /// Links written in the note.
    pub outgoing: Vec<VaultLinkView>,
    /// Links from other notes to this note.
    pub incoming: Vec<VaultLinkView>,
}

/// A vault operation failed without exposing an absolute path.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum VaultRuntimeError {
    /// A path is empty, escapes the vault, or touches a reserved location.
    #[error("invalid vault path")]
    InvalidPath,
    /// A note lacks valid frontmatter or a description.
    #[error("invalid vault note")]
    InvalidNote,
    /// The requested note does not exist.
    #[error("vault note not found")]
    NotFound,
    /// The note exceeds the store size cap.
    #[error("vault note is too large")]
    OverCap,
    /// Vault storage cannot be read or written.
    #[error("vault is unavailable")]
    Unavailable,
    /// The archive is invalid or violates the vault archive rules.
    #[error("invalid vault archive")]
    Archive,
    /// The archive operation was cancelled.
    #[error("vault archive operation cancelled")]
    Cancelled,
    /// Replace mode was requested for a non-empty vault.
    #[error("vault is not empty")]
    VaultNotEmpty,
}

/// Reads and writes notes inside one private vault directory.
#[derive(uniffi::Object)]
pub struct VaultRuntime {
    root: PathBuf,
    store: VaultStore,
}

#[uniffi::export]
impl VaultRuntime {
    /// Opens the private vault and creates it when missing.
    ///
    /// # Errors
    /// Returns an invalid directory or an I/O failure.
    #[uniffi::constructor]
    pub fn new(vault_dir: &str) -> Result<Arc<Self>, VaultRuntimeError> {
        let root = checked_directory(vault_dir).ok_or(VaultRuntimeError::InvalidPath)?;
        fs::create_dir_all(root).map_err(|_| VaultRuntimeError::Unavailable)?;
        Ok(Arc::new(Self {
            root: root.to_path_buf(),
            store: VaultStore::new(root),
        }))
    }

    /// Searches note titles, descriptions, tags, and source text.
    ///
    /// # Errors
    /// Returns an invalid note or storage error.
    pub fn search(&self, query: &str, limit: u32) -> Result<Vec<VaultNoteView>, VaultRuntimeError> {
        self.store
            .search(query, limit as usize)
            .map(|notes| notes.into_iter().map(Into::into).collect())
            .map_err(|error| map_error(&error))
    }

    /// Reads one note by relative Markdown path.
    ///
    /// # Errors
    /// Returns an invalid path, invalid note, missing note, or storage error.
    pub fn read(&self, path: &str) -> Result<VaultNoteView, VaultRuntimeError> {
        self.store
            .read(path)
            .map_err(|error| map_error(&error))?
            .map(Into::into)
            .ok_or(VaultRuntimeError::NotFound)
    }

    /// Replaces one complete note after store validation.
    ///
    /// # Errors
    /// Returns an invalid path, invalid note, size, or storage error.
    pub fn write(&self, path: &str, source: &str) -> Result<VaultNoteView, VaultRuntimeError> {
        self.store
            .write(path, source)
            .map_err(|error| map_error(&error))?;
        self.read(path)
    }

    /// Appends text to one note after store validation.
    ///
    /// # Errors
    /// Returns an invalid path, invalid note, missing note, size, or storage error.
    pub fn append(&self, path: &str, text: &str) -> Result<VaultNoteView, VaultRuntimeError> {
        self.store
            .append(path, text)
            .map_err(|error| map_error(&error))?;
        self.read(path)
    }

    /// Returns outgoing links and backlinks for a note.
    ///
    /// # Errors
    /// Returns an invalid path, invalid note, missing note, or storage error.
    pub fn links(&self, path: &str) -> Result<VaultLinksView, VaultRuntimeError> {
        self.store
            .links(path)
            .map(Into::into)
            .map_err(|error| map_error(&error))
    }

    /// Lists notes with no backlinks.
    ///
    /// # Errors
    /// Returns an invalid note or storage error.
    pub fn orphans(&self) -> Result<Vec<String>, VaultRuntimeError> {
        self.store.orphans().map_err(|error| map_error(&error))
    }

    /// Renders a bounded neighborhood from one map note.
    ///
    /// # Errors
    /// Returns an invalid path, invalid note, missing note, or storage error.
    pub fn map(
        &self,
        start: &str,
        depth: u32,
        token_budget: u32,
    ) -> Result<String, VaultRuntimeError> {
        self.store
            .map(start, depth as usize, token_budget as usize)
            .map_err(|error| map_error(&error))
    }

    /// Exports Markdown notes and supported attachments as a ZIP.
    ///
    /// # Errors
    /// Returns an invalid destination or storage error.
    pub fn export_zip(&self, dest_path: String) -> Result<(), VaultRuntimeError> {
        vault_archive::export(&self.root, &PathBuf::from(dest_path))
    }

    /// Imports Markdown notes and supported attachments from a ZIP.
    ///
    /// # Errors
    /// Returns an invalid archive, invalid note, non-empty-vault, or storage error.
    pub fn import_zip(
        &self,
        zip_path: String,
        mode: ImportMode,
    ) -> Result<VaultImportReportView, VaultRuntimeError> {
        self.import_zip_at_date(zip_path, mode, &vault_archive::import_zip_date())
    }

    /// Syncs the private vault with a platform-owned shared folder.
    ///
    /// # Errors
    /// Returns a platform, path, manifest, or filesystem error.
    pub fn sync_with_platform(
        &self,
        folder: Arc<dyn VaultFolderPlatform>,
    ) -> Result<VaultSyncReportView, VaultRuntimeError> {
        vault_sync::sync(&self.root, folder)
    }
}

impl VaultRuntime {
    pub(crate) fn import_zip_at_date(
        &self,
        zip_path: String,
        mode: ImportMode,
        date: &str,
    ) -> Result<VaultImportReportView, VaultRuntimeError> {
        vault_archive::import(&self.root, &PathBuf::from(zip_path), mode, date)
    }
}

impl From<VaultNote> for VaultNoteView {
    fn from(note: VaultNote) -> Self {
        Self {
            path: note.path,
            title: note.title,
            description: note.description,
            note_type: note.note_type,
            tags: note.tags,
            aliases: note.aliases,
            created: note.created,
            updated: note.updated,
            headings: note.headings,
            source: note.source,
        }
    }
}

impl From<VaultLink> for VaultLinkView {
    fn from(link: VaultLink) -> Self {
        Self {
            from: link.source_path,
            target: link.target,
            heading: link.heading,
            alias: link.alias,
            resolved: link.resolved,
            resolved_path: link.resolved_path,
        }
    }
}

impl From<NoteLinks> for VaultLinksView {
    fn from(links: NoteLinks) -> Self {
        Self {
            outgoing: links.outgoing.into_iter().map(Into::into).collect(),
            incoming: links.incoming.into_iter().map(Into::into).collect(),
        }
    }
}

fn map_error(error: &VaultError) -> VaultRuntimeError {
    match error {
        VaultError::Io(_) => VaultRuntimeError::Unavailable,
        VaultError::InvalidNote { .. } => VaultRuntimeError::InvalidNote,
        VaultError::MissingNote => VaultRuntimeError::NotFound,
        VaultError::InvalidPath => VaultRuntimeError::InvalidPath,
        VaultError::OverCap => VaultRuntimeError::OverCap,
    }
}

#[cfg(test)]
#[path = "vault_tests.rs"]
mod tests;
