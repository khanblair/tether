//! Read-only project files for the tree, viewer, and search.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::sync::Arc;

use tether_store::Store;
use tether_types::ProjectId;
use tether_workspace::{FileIndex, PathGuard};

const DIR_CAP: usize = 2_000;
const SEARCH_CAP: usize = 100;
const PREVIEW_CAP: u64 = 256 * 1024;
const NUL_CHECK_CAP: usize = 8 * 1024;

/// One direct child in a project directory.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FileEntryView {
    /// File or folder name.
    pub name: String,
    /// Project-relative path with slash separators.
    pub path: String,
    /// Whether this entry is a folder.
    pub is_dir: bool,
    /// File size in bytes, or zero for a folder.
    pub size_bytes: u64,
    /// Whether the project ignore rules hide this entry by default.
    pub ignored: bool,
}

/// A bounded directory listing.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DirListingView {
    /// Sorted direct children.
    pub entries: Vec<FileEntryView>,
    /// Whether more than 2,000 visible children exist.
    pub truncated: bool,
}

/// A bounded read-only file preview.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FilePreviewView {
    /// Project-relative path with slash separators.
    pub path: String,
    /// Full file size in bytes.
    pub size_bytes: u64,
    /// UTF-8 text when the file is not binary.
    pub text: Option<String>,
    /// Whether the preview stops before the file ends.
    pub truncated: bool,
    /// Whether invalid UTF-8 or a NUL byte was found.
    pub binary: bool,
}

/// One matching line in a project file.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FileTextHitView {
    /// Project-relative file path.
    pub path: String,
    /// One-based line number.
    pub line: u32,
    /// Matching line without the trailing newline.
    pub text: String,
}

/// A project file operation failed without exposing an absolute path.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FilesRuntimeError {
    /// The path is unsafe or names a file where a folder is needed.
    #[error("invalid project path")]
    InvalidPath,
    /// The project or file does not exist.
    #[error("project file not found")]
    NotFound,
    /// The database or workspace cannot be read.
    #[error("project files are unavailable")]
    Unavailable,
}

/// Reads a saved project's files without changing them.
#[derive(uniffi::Object)]
pub struct FilesRuntime {
    database_path: String,
    workspaces_root: String,
}

#[uniffi::export]
impl FilesRuntime {
    /// Opens the project database for file browsing.
    ///
    /// # Errors
    /// Returns `Unavailable` when the database cannot open.
    #[uniffi::constructor]
    pub fn new(database_path: &str, workspaces_root: &str) -> Result<Arc<Self>, FilesRuntimeError> {
        Store::open(Path::new(database_path)).map_err(|_| FilesRuntimeError::Unavailable)?;
        Ok(Arc::new(Self {
            database_path: database_path.into(),
            workspaces_root: workspaces_root.into(),
        }))
    }

    /// Lists the direct children of a project folder.
    ///
    /// # Errors
    /// Returns an invalid path, missing folder, or unavailable storage.
    pub fn list_dir(
        &self,
        project_id: &str,
        dir: &str,
        show_ignored: bool,
    ) -> Result<DirListingView, FilesRuntimeError> {
        let guard = self.project_guard(project_id)?;
        let directory = if dir.is_empty() {
            guard.root().to_path_buf()
        } else {
            guard.resolve(dir).map_err(|error| map_path_error(&error))?
        };
        if !directory.is_dir() {
            return Err(path_kind_error(&directory));
        }
        let index = FileIndex::scan(guard.clone()).map_err(|error| map_io_error(&error))?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(directory).map_err(|error| map_io_error(&error))? {
            let entry = entry.map_err(|error| map_io_error(&error))?;
            entries.extend(child_view(&entry, &guard, &index, show_ignored)?);
        }
        entries.sort_by(|left, right| {
            right
                .is_dir
                .cmp(&left.is_dir)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                .then_with(|| left.name.cmp(&right.name))
        });
        let truncated = entries.len() > DIR_CAP;
        entries.truncate(DIR_CAP);
        Ok(DirListingView { entries, truncated })
    }

    /// Reads at most 256 KiB of a regular project file.
    ///
    /// # Errors
    /// Returns an invalid path, missing file, or unavailable storage.
    pub fn read_file(
        &self,
        project_id: &str,
        path: &str,
    ) -> Result<FilePreviewView, FilesRuntimeError> {
        let guard = self.project_guard(project_id)?;
        let target = guard
            .resolve(path)
            .map_err(|error| map_path_error(&error))?;
        let metadata = fs::metadata(&target).map_err(|error| map_path_error(&error))?;
        if !metadata.is_file() {
            return Err(FilesRuntimeError::InvalidPath);
        }
        let mut bytes = Vec::new();
        File::open(&target)
            .map_err(|error| map_io_error(&error))?
            .take(PREVIEW_CAP)
            .read_to_end(&mut bytes)
            .map_err(|error| map_io_error(&error))?;
        let truncated = metadata.len() > PREVIEW_CAP;
        let binary_nul = bytes.iter().take(NUL_CHECK_CAP).any(|byte| *byte == 0);
        let text = preview_text(&bytes, truncated, binary_nul);
        Ok(FilePreviewView {
            path: relative_path(guard.root(), &target),
            size_bytes: metadata.len(),
            binary: text.is_none(),
            text,
            truncated,
        })
    }

    /// Finds project files by a simple glob or case-insensitive substring.
    ///
    /// # Errors
    /// Returns `NotFound` for an unknown project or `Unavailable` for storage failure.
    pub fn search_file_names(
        &self,
        project_id: &str,
        pattern: &str,
        limit: u32,
    ) -> Result<Vec<String>, FilesRuntimeError> {
        let index = FileIndex::scan(self.project_guard(project_id)?)
            .map_err(|error| map_io_error(&error))?;
        let cap = usize::try_from(limit).unwrap_or(SEARCH_CAP).min(SEARCH_CAP);
        if pattern.contains('*') {
            return Ok(glob_file_names(&index, pattern, cap));
        }
        let needle = pattern.to_lowercase();
        Ok(index
            .files()
            .iter()
            .filter(|file| file.path.to_string_lossy().to_lowercase().contains(&needle))
            .take(cap)
            .map(|file| path_string(&file.path))
            .collect())
    }

    /// Finds matching text lines in indexed project files.
    ///
    /// # Errors
    /// Returns `NotFound` for an unknown project or `Unavailable` for storage failure.
    pub fn search_file_text(
        &self,
        project_id: &str,
        text: &str,
        limit: u32,
    ) -> Result<Vec<FileTextHitView>, FilesRuntimeError> {
        let index = FileIndex::scan(self.project_guard(project_id)?)
            .map_err(|error| map_io_error(&error))?;
        let cap = usize::try_from(limit).unwrap_or(SEARCH_CAP).min(SEARCH_CAP);
        index
            .grep(text, cap)
            .map_err(|error| map_io_error(&error))?
            .into_iter()
            .map(|hit| {
                Ok(FileTextHitView {
                    path: path_string(&hit.path),
                    line: u32::try_from(hit.line).map_err(|_| FilesRuntimeError::Unavailable)?,
                    text: hit.text,
                })
            })
            .collect()
    }
}

fn glob_file_names(index: &FileIndex, pattern: &str, cap: usize) -> Vec<String> {
    let mut matches = index.glob(pattern);
    matches.extend(index.glob(&format!("*/{pattern}")));
    index
        .files()
        .iter()
        .filter(|file| matches.iter().any(|matched| matched.path == file.path))
        .take(cap)
        .map(|file| path_string(&file.path))
        .collect()
}

impl FilesRuntime {
    pub(crate) fn project_guard(&self, project_id: &str) -> Result<PathGuard, FilesRuntimeError> {
        let store = Store::open(Path::new(&self.database_path))
            .map_err(|_| FilesRuntimeError::Unavailable)?;
        let project_path = store
            .project_path(&ProjectId::new(project_id.to_owned()))
            .map_err(|_| FilesRuntimeError::Unavailable)?
            .ok_or(FilesRuntimeError::NotFound)?;
        PathGuard::new(Path::new(&self.workspaces_root).join(project_path))
            .map_err(|_| FilesRuntimeError::Unavailable)
    }
}

fn child_view(
    entry: &fs::DirEntry,
    guard: &PathGuard,
    index: &FileIndex,
    show_ignored: bool,
) -> Result<Option<FileEntryView>, FilesRuntimeError> {
    let kind = entry.file_type().map_err(|error| map_io_error(&error))?;
    if kind.is_symlink() || (!kind.is_dir() && !kind.is_file()) || entry.file_name() == ".git" {
        return Ok(None);
    }
    let path = entry.path();
    let relative = path
        .strip_prefix(guard.root())
        .map_err(|_| FilesRuntimeError::Unavailable)?;
    let ignored = index.is_ignored(relative);
    if ignored && !show_ignored {
        return Ok(None);
    }
    let size_bytes = if kind.is_dir() {
        0
    } else {
        entry
            .metadata()
            .map_err(|error| map_io_error(&error))?
            .len()
    };
    Ok(Some(FileEntryView {
        name: entry.file_name().to_string_lossy().into_owned(),
        path: path_string(relative),
        is_dir: kind.is_dir(),
        size_bytes,
        ignored,
    }))
}

fn preview_text(bytes: &[u8], truncated: bool, binary_nul: bool) -> Option<String> {
    if binary_nul {
        return None;
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => Some(text.to_owned()),
        Err(error) if truncated && error.error_len().is_none() => {
            Some(String::from_utf8_lossy(&bytes[..error.valid_up_to()]).into_owned())
        }
        Err(_) => None,
    }
}

fn path_string(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).map(path_string).unwrap_or_default()
}

fn path_kind_error(path: &Path) -> FilesRuntimeError {
    if path.exists() {
        FilesRuntimeError::InvalidPath
    } else {
        FilesRuntimeError::NotFound
    }
}

pub(crate) fn map_path_error(error: &io::Error) -> FilesRuntimeError {
    match error.kind() {
        io::ErrorKind::NotFound => FilesRuntimeError::NotFound,
        io::ErrorKind::PermissionDenied | io::ErrorKind::NotADirectory => {
            FilesRuntimeError::InvalidPath
        }
        _ => FilesRuntimeError::Unavailable,
    }
}

pub(crate) fn map_io_error(error: &io::Error) -> FilesRuntimeError {
    if error.kind() == io::ErrorKind::NotFound {
        FilesRuntimeError::NotFound
    } else {
        FilesRuntimeError::Unavailable
    }
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
