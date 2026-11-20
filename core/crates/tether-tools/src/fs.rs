//! Bounded, workspace-scoped read tools.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use regex::RegexBuilder;
use tether_workspace::{FileIndex, IndexedFile, PathGuard, SearchHit};

const MAX_READ_BYTES: u64 = 1024 * 1024;
const MAX_RESULTS: usize = 1_000;
const MAX_GREP_RESULT_BYTES: usize = 16 * 1024;
const MAX_GREP_LINE_BYTES: usize = 1024;

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
        File::open(path)?
            .take(MAX_READ_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_READ_BYTES || bytes.contains(&0) {
            return Err(invalid_input("file is binary or exceeds the read limit"));
        }
        let text = String::from_utf8(bytes).map_err(|_| invalid_input("file is not UTF-8"))?;
        let Some((start, end)) = lines else {
            return Ok(ReadFile {
                start_line: 1,
                text,
            });
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
        list_entries(self.guard.root(), &path)
    }

    /// Finds indexed files whose relative paths match the index's glob syntax.
    ///
    /// # Errors
    /// Returns an error if the workspace cannot be indexed.
    pub fn glob(&self, pattern: &str) -> io::Result<Vec<IndexedFile>> {
        let index = FileIndex::scan(self.guard.clone())?;
        Ok(index
            .glob(pattern)
            .into_iter()
            .take(MAX_RESULTS)
            .cloned()
            .collect())
    }

    /// Finds lines matching a regex in indexed files, up to the requested result cap.
    ///
    /// # Errors
    /// Returns an error if the workspace cannot be scanned or a file cannot be read.
    #[expect(
        clippy::excessive_nesting,
        clippy::too_many_lines,
        reason = "file and line bounds are checked inside the workspace scan"
    )]
    pub fn grep(&self, pattern: &str, limit: usize) -> io::Result<Vec<SearchHit>> {
        if pattern.is_empty() || pattern.len() > 4096 {
            return Err(invalid_input("grep pattern must contain 1 to 4096 bytes"));
        }
        let regex = RegexBuilder::new(pattern)
            .size_limit(1024 * 1024)
            .build()
            .map_err(|_| invalid_input("invalid grep regex"))?;
        let mut hits = Vec::new();
        let mut result_bytes: usize = 0;
        if limit == 0 {
            return Ok(hits);
        }
        let index = FileIndex::scan(self.guard.clone())?;
        for file in index.files() {
            if hits.len() >= limit.min(MAX_RESULTS) {
                break;
            }
            if file.bytes > MAX_READ_BYTES {
                continue;
            }
            let path = self.guard.resolve(&file.path)?;
            let mut bytes = Vec::new();
            File::open(path)?
                .take(MAX_READ_BYTES + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_READ_BYTES || bytes.contains(&0) {
                continue;
            }
            let Ok(text) = String::from_utf8(bytes) else {
                continue;
            };
            for (line, text) in text.lines().enumerate() {
                if let Some(found) = regex.find(text) {
                    let excerpt = match_excerpt(text, found.start());
                    let bytes = file.path.as_os_str().len() + excerpt.len() + 32;
                    if result_bytes.saturating_add(bytes) > MAX_GREP_RESULT_BYTES {
                        return Ok(hits);
                    }
                    result_bytes += bytes;
                    hits.push(SearchHit {
                        path: file.path.clone(),
                        line: line + 1,
                        text: excerpt,
                    });
                    if hits.len() >= limit.min(MAX_RESULTS) {
                        break;
                    }
                }
            }
        }
        Ok(hits)
    }
}

fn match_excerpt(line: &str, match_start: usize) -> String {
    if line.len() <= MAX_GREP_LINE_BYTES {
        return line.to_owned();
    }
    let mut start = match_start.saturating_sub(MAX_GREP_LINE_BYTES / 4);
    while !line.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (start + MAX_GREP_LINE_BYTES - 6).min(line.len());
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    let prefix = if start > 0 { "…" } else { "" };
    let suffix = if end < line.len() { "…" } else { "" };
    format!("{prefix}{}{suffix}", &line[start..end])
}

fn list_entries(root: &Path, path: &Path) -> io::Result<Vec<DirectoryEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() || !(kind.is_dir() || kind.is_file()) {
            continue;
        }
        let path = entry.path();
        if is_git_path(root, &path) {
            continue;
        }
        entries.push(DirectoryEntry {
            path: path
                .strip_prefix(root)
                .map_err(io::Error::other)?
                .to_path_buf(),
            is_dir: kind.is_dir(),
        });
        if entries.len() > MAX_RESULTS {
            return Err(invalid_input("directory exceeds the result limit"));
        }
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
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
