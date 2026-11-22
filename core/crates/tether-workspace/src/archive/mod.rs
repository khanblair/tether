//! Safe, bounded ZIP import and export for project workspaces.

use std::io;

use zip::result::ZipError;

#[cfg(test)]
mod archive_tests;
mod create;
mod extract;
mod names;

pub use create::create_zip;
pub use extract::extract_zip;

const DEFAULT_MAX_ENTRIES: u64 = 50_000;
const DEFAULT_MAX_TOTAL_BYTES: u64 = 1 << 30;
const DEFAULT_MAX_FILE_BYTES: u64 = 256 << 20;
const RATIO_MIN_BYTES: u64 = 100 << 20;
const RATIO_LIMIT: u64 = 200;

/// Limits applied while extracting a ZIP archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtractLimits {
    /// Maximum number of archive entries.
    pub max_entries: u64,
    /// Maximum total uncompressed bytes.
    pub max_total_bytes: u64,
    /// Maximum uncompressed bytes for one file.
    pub max_file_bytes: u64,
}

impl Default for ExtractLimits {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_MAX_ENTRIES,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }
}

/// Options applied while creating a ZIP archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateOptions {
    /// Include files ignored by the workspace's ignore rules.
    pub include_ignored: bool,
    /// Maximum total bytes read from regular files.
    pub max_total_bytes: u64,
}

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            include_ignored: false,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
        }
    }
}

/// Results from extracting a ZIP archive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExtractReport {
    /// Number of regular files written.
    pub files: u64,
    /// Number of directories created.
    pub directories: u64,
    /// Number of uncompressed file bytes written.
    pub bytes: u64,
    /// Number of symbolic-link entries skipped.
    pub skipped_symlinks: u64,
}

/// Results from creating a ZIP archive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CreateReport {
    /// Number of regular files archived.
    pub files: u64,
    /// Number of regular file bytes read.
    pub bytes: u64,
    /// Number of symbolic links skipped.
    pub skipped_symlinks: u64,
}

/// A safe archive operation failure.
#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    /// An entry name is unsafe.
    #[error("invalid archive entry name")]
    InvalidEntryName,
    /// The archive contains too many entries.
    #[error("archive has too many entries")]
    TooManyEntries,
    /// The archive exceeds a byte limit.
    #[error("archive is too large")]
    TooLarge,
    /// A file has an unsafe compression ratio.
    #[error("archive compression ratio is unsafe")]
    Ratio,
    /// The operation was cancelled.
    #[error("archive operation cancelled")]
    Cancelled,
    /// The destination path is unsafe.
    #[error("archive destination is invalid")]
    InvalidDestination,
    /// The input is not a valid ZIP archive.
    #[error("file is not a ZIP archive")]
    NotZip,
    /// The filesystem operation failed.
    #[error("archive I/O failure")]
    Io(#[source] io::Error),
}

impl From<ZipError> for ArchiveError {
    fn from(error: ZipError) -> Self {
        match error {
            ZipError::Io(error) => Self::Io(error),
            _ => Self::NotZip,
        }
    }
}
