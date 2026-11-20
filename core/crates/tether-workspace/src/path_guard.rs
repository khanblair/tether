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
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "workspace is not a directory",
            ));
        }
        Ok(Self { root })
    }

    /// Resolves an existing or new workspace-relative path. Symlinks are never followed.
    ///
    /// # Errors
    /// Returns an error if any component escapes the workspace or the filesystem fails.
    pub fn resolve(&self, relative: impl AsRef<Path>) -> io::Result<PathBuf> {
        let relative = relative.as_ref();
        if relative.as_os_str().is_empty() || has_current_dir_segment(relative) {
            return Err(denied());
        }
        let mut current = self.root.clone();
        for component in relative.components() {
            current = self.step(&current, component)?;
        }
        Ok(current)
    }

    /// Adds one component to `current`, rejecting anything but a plain name,
    /// a symlink, or a path that canonicalizes outside the root.
    fn step(&self, current: &Path, component: Component<'_>) -> io::Result<PathBuf> {
        let Component::Normal(name) = component else {
            return Err(denied());
        };
        let candidate = current.join(name);
        match fs::symlink_metadata(&candidate) {
            Ok(metadata) if metadata.file_type().is_symlink() => Err(denied()),
            Ok(_) => self.inside_root(fs::canonicalize(candidate)?),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(candidate),
            Err(error) => Err(error),
        }
    }

    fn inside_root(&self, path: PathBuf) -> io::Result<PathBuf> {
        if path.starts_with(&self.root) {
            Ok(path)
        } else {
            Err(denied())
        }
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

fn has_current_dir_segment(path: &Path) -> bool {
    path.as_os_str()
        .as_encoded_bytes()
        .split(|byte| *byte == std::path::MAIN_SEPARATOR as u8)
        .any(|part| part == b".")
}
