//! Bounded, workspace-scoped read tools.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use tether_workspace::{FileIndex, IndexedFile, PathGuard, SearchHit};

const MAX_READ_BYTES: u64 = 1024 * 1024;
const MAX_RESULTS: usize = 1_000;

/// A file read with the first returned line number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadFile {
    /// One-based first line in `text`.
    pub start_line: usize,
    /// File contents or the requested inclusive line range.
    pub text: String,
}

/// A direct child of a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    /// Workspace-relative path.
    pub path: PathBuf,
    /// Whether this entry is a directory.
    pub is_dir: bool,
}

/// Read-only tools whose paths stay inside one workspace.
#[derive(Debug, Clone)]
pub struct ReadTools {
    guard: PathGuard,
}

impl ReadTools {
    /// Binds all reads to a validated workspace root.
    #[must_use]
    pub fn new(guard: PathGuard) -> Self {
        Self { guard }
    }

    /// Reads a UTF-8 file or an inclusive one-based line range.
    /// A complete file read is capped at 1 MiB; line ranges are also capped.
    ///
    /// # Errors
    /// Rejects escaped paths, non-files, invalid ranges, binary or oversized text.
    pub fn read_file(&self, path: &str, lines: Option<(usize, usize)>) -> io::Result<ReadFile> {
        let path = self.guard.resolve(path)?;
        if !path.is_file() || is_git_path(self.guard.root(), &path) {
            return Err(invalid_input("not a readable workspace file"));
        }
        let metadata = fs::metadata(&path)?;
        if metadata.len() > MAX_READ_BYTES {
            return Err(invalid_input("file exceeds the read limit"));
        }
        let mut bytes = Vec::new();
        File::open(path)?.take(MAX_READ_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_READ_BYTES || bytes.contains(&0) {
            return Err(invalid_input("file is binary or exceeds the read limit"));
        }
        let text = String::from_utf8(bytes).map_err(|_| invalid_input("file is not UTF-8"))?;
        let Some((start, end)) = lines else {
            return Ok(ReadFile { start_line: 1, text });
        };
        if start == 0 || end < start {
            return Err(invalid_input("invalid line range"));
        }
        let selected = text
            .split_inclusive('\n')
            .skip(start - 1)
            .take(end - start + 1)
            .collect();
        Ok(ReadFile {
            start_line: start,
            text: selected,
        })
    }

    /// Lists direct children in path order, excluding Git metadata and symlinks.
    /// Pass an empty path for the workspace root.
    ///
    /// # Errors
    /// Returns an error for an escaped or unreadable directory.
    pub fn list_dir(&self, directory: &str) -> io::Result<Vec<DirectoryEntry>> {
        let path = if directory.is_empty() {
            self.guard.root().to_path_buf()
        } else {
            self.guard.resolve(directory)?
        };
        if is_git_path(self.guard.root(), &path) {
            return Err(invalid_input("Git metadata is not readable"));
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() || !(kind.is_dir() || kind.is_file()) {
                continue;
            }
            let path = entry.path();
            if is_git_path(self.guard.root(), &path) {
                continue;
            }
            entries.push(DirectoryEntry {
                path: path.strip_prefix(self.guard.root()).map_err(io::Error::other)?.to_path_buf(),
                is_dir: kind.is_dir(),
            });
            if entries.len() > MAX_RESULTS {
                return Err(invalid_input("directory exceeds the result limit"));
            }
        }
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(entries)
    }

    /// Finds indexed files whose relative paths match the index's glob syntax.
    ///
    /// # Errors
    /// Returns an error if the workspace cannot be indexed.
    pub fn glob(&self, pattern: &str) -> io::Result<Vec<IndexedFile>> {
        let index = FileIndex::scan(self.guard.clone())?;
        Ok(index.glob(pattern).into_iter().take(MAX_RESULTS).cloned().collect())
    }

    /// Finds literal text in indexed files, up to the requested result cap.
    /// Regex search remains a later index implementation task.
    ///
    /// # Errors
    /// Returns an error if the workspace cannot be scanned or a file cannot be read.
    pub fn grep(&self, needle: &str, limit: usize) -> io::Result<Vec<SearchHit>> {
        FileIndex::scan(self.guard.clone())?.grep(needle, limit.min(MAX_RESULTS))
    }
}

fn is_git_path(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .is_ok_and(|relative| relative.components().any(|part| part.as_os_str() == ".git"))
}

fn invalid_input(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests;
