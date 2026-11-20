//! A workspace-relative file index and bounded text search.

use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::PathGuard;

/// One indexed regular file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedFile {
    /// Workspace-relative path.
    pub path: PathBuf,
    /// File size in bytes at the last scan.
    pub bytes: u64,
}

/// A matching text line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    /// Workspace-relative path.
    pub path: PathBuf,
    /// One-based line number.
    pub line: usize,
    /// Matching line without a trailing newline.
    pub text: String,
}

/// Snapshot of regular files under a guarded workspace.
#[derive(Debug, Clone)]
pub struct FileIndex {
    guard: PathGuard,
    files: Vec<IndexedFile>,
}

impl FileIndex {
    /// Scans files, skipping `.git` at every depth and root `.gitignore` patterns.
    ///
    /// # Errors
    /// Returns an error when a directory cannot be read.
    pub fn scan(guard: PathGuard) -> io::Result<Self> {
        let ignores = read_ignores(&guard)?;
        let mut files = Vec::new();
        walk(guard.root(), guard.root(), &ignores, &mut files)?;
        files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(Self { guard, files })
    }

    /// Every indexed file in path order.
    #[must_use]
    pub fn files(&self) -> &[IndexedFile] {
        &self.files
    }

    /// Files whose relative path matches a simple `*` glob.
    #[must_use]
    pub fn glob(&self, pattern: &str) -> Vec<&IndexedFile> {
        self.files
            .iter()
            .filter(|file| glob_match(pattern, &file.path.to_string_lossy()))
            .collect()
    }

    /// Finds text in indexed files, stopping after `limit` matching lines.
    ///
    /// # Errors
    /// Returns an error if an indexed file changed to a disallowed path or cannot be read.
    pub fn grep(&self, needle: &str, limit: usize) -> io::Result<Vec<SearchHit>> {
        let mut hits = Vec::new();
        if needle.is_empty() || limit == 0 {
            return Ok(hits);
        }
        let mut files = self.files.iter();
        while let Some(file) = files.next().filter(|_| hits.len() < limit) {
            let path = self.guard.resolve(&file.path)?;
            grep_file(&path, &file.path, needle, limit, &mut hits)?;
        }
        Ok(hits)
    }
}

/// Appends matching lines from one file until `hits` reaches `limit`.
fn grep_file(
    path: &Path,
    relative: &Path,
    needle: &str,
    limit: usize,
    hits: &mut Vec<SearchHit>,
) -> io::Result<()> {
    let reader = BufReader::new(File::open(path)?);
    let mut file_hits = Vec::new();
    for (number, line) in reader.lines().enumerate() {
        let text = match line {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::InvalidData => return Ok(()),
            Err(error) => return Err(error),
        };
        if text.contains(needle) && hits.len() + file_hits.len() < limit {
            file_hits.push(SearchHit {
                path: relative.to_path_buf(),
                line: number + 1,
                text,
            });
        }
    }
    hits.extend(file_hits);
    Ok(())
}

fn read_ignores(guard: &PathGuard) -> io::Result<Vec<String>> {
    let path = match guard.resolve(".gitignore") {
        Ok(path) => path,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect())
}

fn walk(
    root: &Path,
    directory: &Path,
    ignores: &[String],
    files: &mut Vec<IndexedFile>,
) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).expect("walk stays under root");
        let name = relative.to_string_lossy();
        if entry.file_name() == ".git" || ignores.iter().any(|pattern| glob_match(pattern, &name)) {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            walk(root, &path, ignores, files)?;
        } else if kind.is_file() {
            files.push(IndexedFile {
                path: relative.to_path_buf(),
                bytes: entry.metadata()?.len(),
            });
        }
    }
    Ok(())
}

fn glob_match(pattern: &str, value: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or("");
    if !value.starts_with(first) {
        return false;
    }
    let mut rest = &value[first.len()..];
    let trailing_star = pattern.ends_with('*');
    for part in parts {
        if let Some(position) = rest.find(part) {
            rest = &rest[position + part.len()..];
        } else {
            return false;
        }
    }
    trailing_star || rest.is_empty()
}
