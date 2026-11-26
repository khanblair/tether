//! Raw bytes of one project file, for the in-app Preview.

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::files::{FilesRuntime, FilesRuntimeError, map_io_error, map_path_error, relative_path};

/// The most bytes one `read_file_bytes` call returns.
const BYTES_CAP: u64 = 10 * 1024 * 1024;

/// The bytes of one project file, cut at the caller's limit.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FileBytesView {
    /// Project-relative path with slash separators.
    pub path: String,
    /// Full file size in bytes.
    pub size_bytes: u64,
    /// The first bytes of the file, up to the read limit.
    pub bytes: Vec<u8>,
    /// Whether the file is longer than `bytes`.
    pub truncated: bool,
}

#[uniffi::export]
impl FilesRuntime {
    /// Reads the bytes of one regular project file for the Preview.
    ///
    /// At most `min(limit_bytes, 10 MiB)` bytes come back. A zero limit reads the 10 MiB cap.
    /// Paths with a `.git` component are never read.
    ///
    /// # Errors
    /// Returns `InvalidPath` for a folder, a symlink, a `.git` path, or an unsafe path.
    /// Returns `NotFound` for a missing file or project, and `Unavailable` for storage failure.
    pub fn read_file_bytes(
        &self,
        project_id: &str,
        path: &str,
        limit_bytes: u64,
    ) -> Result<FileBytesView, FilesRuntimeError> {
        let guard = self.project_guard(project_id)?;
        let target = guard
            .resolve(path)
            .map_err(|error| map_path_error(&error))?;
        if is_under_git(Path::new(path)) {
            return Err(FilesRuntimeError::InvalidPath);
        }
        let metadata = fs::metadata(&target).map_err(|error| map_path_error(&error))?;
        if !metadata.is_file() {
            return Err(FilesRuntimeError::InvalidPath);
        }
        let mut bytes = Vec::new();
        File::open(&target)
            .map_err(|error| map_io_error(&error))?
            .take(byte_cap(limit_bytes))
            .read_to_end(&mut bytes)
            .map_err(|error| map_io_error(&error))?;
        Ok(FileBytesView {
            path: relative_path(guard.root(), &target),
            size_bytes: metadata.len(),
            truncated: metadata.len() > bytes.len() as u64,
            bytes,
        })
    }
}

/// The read limit for one call. Zero and anything above 10 MiB mean 10 MiB.
fn byte_cap(limit_bytes: u64) -> u64 {
    if limit_bytes == 0 || limit_bytes > BYTES_CAP {
        BYTES_CAP
    } else {
        limit_bytes
    }
}

/// Whether any part of the path is the `.git` folder, which holds remote URLs and tokens.
fn is_under_git(path: &Path) -> bool {
    path.components().any(|part| part.as_os_str() == ".git")
}

#[cfg(test)]
#[path = "files_bytes_tests.rs"]
mod tests;
