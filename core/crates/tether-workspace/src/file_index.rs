//! A workspace-relative file index and bounded text search.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::PathGuard;

const BUILTIN_IGNORES: [&str; 10] = [
    "node_modules/",
    "target/",
    "build/",
    ".gradle/",
    ".idea/",
    "__pycache__/",
    ".venv/",
    "venv/",
    "dist/",
    ".DS_Store",
];

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
    ignored: BTreeSet<PathBuf>,
    defaults: Gitignore,
}

impl FileIndex {
    /// Scans files with Git-compatible ignore files and Tether defaults.
    ///
    /// # Errors
    /// Returns an error when a directory or ignore file cannot be read.
    pub fn scan(guard: PathGuard) -> io::Result<Self> {
        let defaults = default_matcher(guard.root())?;
        let mut files = Vec::new();
        let builder = walker(guard.root(), &defaults);
        for result in builder.build() {
            let entry = result.map_err(|error| io::Error::other(error.to_string()))?;
            if entry.depth() == 0 || entry.path_is_symlink() {
                continue;
            }
            let relative = entry
                .path()
                .strip_prefix(guard.root())
                .map_err(io::Error::other)?
                .to_path_buf();
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.is_file() {
                files.push(IndexedFile {
                    path: relative,
                    bytes: metadata.len(),
                });
            }
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
        let mut ignored = BTreeSet::new();
        record_default_ignored(guard.root(), &defaults, &mut ignored)?;
        Ok(Self {
            guard,
            files,
            ignored,
            defaults,
        })
    }

    /// Every indexed file in path order.
    #[must_use]
    pub fn files(&self) -> &[IndexedFile] {
        &self.files
    }

    /// Whether a relative path is ignored by the scan.
    #[must_use]
    pub fn is_ignored(&self, relative: &Path) -> bool {
        if relative.components().any(|part| part.as_os_str() == ".git") {
            return true;
        }
        if self
            .defaults
            .matched_path_or_any_parents(self.guard.root().join(relative), false)
            .is_ignore()
            || self.ignored.contains(relative)
        {
            return true;
        }
        relative.exists() && !self.files.iter().any(|file| file.path == relative)
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
        for file in &self.files {
            if hits.len() >= limit {
                break;
            }
            let path = self.guard.resolve(&file.path)?;
            grep_file(&path, &file.path, needle, limit, &mut hits)?;
        }
        Ok(hits)
    }
}

fn walker(root: &Path, defaults: &Gitignore) -> ignore::WalkBuilder {
    let matcher = defaults.clone();
    let mut builder = ignore::WalkBuilder::new(root);
    builder
        .hidden(false)
        .ignore(false)
        .git_global(false)
        .git_ignore(true)
        .git_exclude(true)
        .follow_links(false)
        .add_custom_ignore_filename(".tetherignore")
        .filter_entry(move |entry| {
            if entry.file_name().to_string_lossy() == ".git" {
                return false;
            }
            entry.file_type().is_none_or(|kind| {
                kind.is_dir() || !matcher.matched(entry.path(), false).is_ignore()
            })
        });
    builder
}

fn default_matcher(root: &Path) -> io::Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in BUILTIN_IGNORES {
        builder
            .add_line(None, pattern)
            .map_err(|error| io::Error::other(error.to_string()))?;
    }
    let path = root.join(".tetherignore");
    if fs::symlink_metadata(&path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
    {
        if let Some(error) = builder.add(path) {
            return Err(io::Error::other(error.to_string()));
        }
    }
    builder
        .build()
        .map_err(|error| io::Error::other(error.to_string()))
}

fn record_default_ignored(
    root: &Path,
    defaults: &Gitignore,
    ignored: &mut BTreeSet<PathBuf>,
) -> io::Result<()> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for item in fs::read_dir(directory)? {
            let item = item?;
            let path = item.path();
            let relative = path.strip_prefix(root).map_err(io::Error::other)?;
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if path.file_name().is_some_and(|name| name == ".git") {
                ignored.insert(relative.to_path_buf());
                continue;
            }
            if defaults.matched(&path, metadata.is_dir()).is_ignore() {
                ignored.insert(relative.to_path_buf());
            }
            if metadata.is_dir() {
                stack.push(path);
            }
        }
    }
    Ok(())
}

fn grep_file(
    path: &Path,
    relative: &Path,
    needle: &str,
    limit: usize,
    hits: &mut Vec<SearchHit>,
) -> io::Result<()> {
    let reader = BufReader::new(File::open(path)?);
    for (number, line) in reader.lines().enumerate() {
        let text = match line {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::InvalidData => return Ok(()),
            Err(error) => return Err(error),
        };
        if text.contains(needle) && hits.len() < limit {
            hits.push(SearchHit {
                path: relative.to_path_buf(),
                line: number + 1,
                text,
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
