//! Canonical workspace boundary for file tools.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Resolves paths beneath one trusted workspace root.
#[derive(Debug, Clone)]
pub struct PathGuard {
    root: PathBuf,
}

impl PathGuard {
    /// Opens an existing directory as the workspace boundary.
    ///
    /// # Errors
    /// Returns an error for a symlink, missing directory, or filesystem failure.
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref();
        if fs::symlink_metadata(root)?.file_type().is_symlink() {
            return Err(denied());
        }
        let root = fs::canonicalize(root)?;
        if !root.is_dir() {
            return Err(io::Error::new(io::ErrorKind::NotADirectory, "workspace is not a directory"));
        }
        Ok(Self { root })
    }

    /// Resolves an existing or new workspace-relative path. Symlinks are never followed.
    ///
    /// # Errors
    /// Returns an error if any component escapes the workspace or the filesystem fails.
    pub fn resolve(&self, relative: impl AsRef<Path>) -> io::Result<PathBuf> {
        let relative = relative.as_ref();
        if relative.as_os_str().is_empty() {
            return Err(denied());
        }
        let mut current = self.root.clone();
        for component in relative.components() {
            let Component::Normal(name) = component else {
                return Err(denied());
            };
            let candidate = current.join(name);
            match fs::symlink_metadata(&candidate) {
                Ok(metadata) if metadata.file_type().is_symlink() => return Err(denied()),
                Ok(_) => {
                    current = fs::canonicalize(candidate)?;
                    if !current.starts_with(&self.root) {
                        return Err(denied());
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => current = candidate,
                Err(error) => return Err(error),
            }
        }
        Ok(current)
    }

    /// The canonical workspace directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "path leaves the workspace")
}
