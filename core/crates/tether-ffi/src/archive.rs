//! Shared archive limits and FFI error mapping.

use tether_workspace::archive::{ArchiveError, ExtractLimits};

/// Returns the archive limits used by project and vault imports.
pub(crate) fn extract_limits() -> ExtractLimits {
    ExtractLimits::default()
}

/// Maps a safe archive failure to a short app-facing detail.
pub(crate) fn archive_detail(error: &ArchiveError) -> &'static str {
    match error {
        ArchiveError::InvalidEntryName => "invalid archive entry",
        ArchiveError::TooManyEntries => "archive has too many entries",
        ArchiveError::TooLarge => "archive is too large",
        ArchiveError::Ratio => "archive compression ratio is unsafe",
        ArchiveError::Cancelled => "archive operation cancelled",
        ArchiveError::InvalidDestination => "archive destination is invalid",
        ArchiveError::NotZip => "file is not a ZIP archive",
        ArchiveError::Io(_) => "archive storage is unavailable",
    }
}

/// Returns whether an archive failure means the input was not acceptable.
pub(crate) fn is_archive_input_error(error: &ArchiveError) -> bool {
    matches!(
        error,
        ArchiveError::InvalidEntryName
            | ArchiveError::TooManyEntries
            | ArchiveError::TooLarge
            | ArchiveError::Ratio
            | ArchiveError::NotZip
    )
}
