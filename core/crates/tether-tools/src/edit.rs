//! Atomic text edits kept inside a project workspace.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_workspace::PathGuard;

const MAX_TEXT_BYTES: usize = 4 * 1024 * 1024;

/// One exact text replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditSpec {
    /// Text that must occur exactly once.
    pub old: String,
    /// Replacement text.
    pub new: String,
}

/// Why an edit was rejected.
#[derive(Debug)]
pub enum EditError {
    /// Filesystem operation failed.
    Io(io::Error),
    /// The old text was absent.
    MissingMatch,
    /// The old text occurred more than once.
    AmbiguousMatch,
    /// An empty old text is not a valid exact match.
    EmptyPattern,
    /// The destination already exists.
    DestinationExists,
    /// The target is not a regular file.
    NotFile,
    /// Text exceeds the edit tool cap.
    TooLarge,
}

impl From<io::Error> for EditError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Text file tools scoped to a validated project root.
#[derive(Debug, Clone)]
pub struct EditTools {
    guard: PathGuard,
}

impl EditTools {
    /// Uses the given path guard for every operation.
    #[must_use]
    pub fn new(guard: PathGuard) -> Self {
        Self { guard }
    }

    /// Writes one text file through a temporary file and rename.
    ///
    /// # Errors
    /// Returns path, size, or file errors.
    pub fn write(&self, path: &str, text: &str) -> Result<(), EditError> {
        let target = self.resolve_mutation_path(path)?;
        if text.len() > MAX_TEXT_BYTES {
            return Err(EditError::TooLarge);
        }
        if target.exists() && !target.is_file() {
            return Err(EditError::NotFile);
        }
        fs::create_dir_all(target.parent().ok_or(EditError::NotFile)?)?;
        atomic_write(&target, text.as_bytes())?;
        Ok(())
    }

    /// Replaces one exact text span. Multiple matches are an error.
    ///
    /// # Errors
    /// Returns a clear match error or file failure without changing the file.
    pub fn edit(&self, path: &str, spec: &EditSpec) -> Result<(), EditError> {
        self.multi_edit(path, std::slice::from_ref(spec))
    }

    /// Applies every replacement in memory, then writes once if all matches succeed.
    ///
    /// # Errors
    /// Returns a clear match error or file failure without changing the file.
    pub fn multi_edit(&self, path: &str, specs: &[EditSpec]) -> Result<(), EditError> {
        let target = self.resolve_mutation_path(path)?;
        if !target.is_file() {
            return Err(EditError::NotFile);
        }
        let mut text = fs::read_to_string(&target)?;
        if text.len() > MAX_TEXT_BYTES {
            return Err(EditError::TooLarge);
        }
        for spec in specs {
            text = replace_once(text, spec)?;
        }
        if !specs.is_empty() {
            atomic_write(&target, text.as_bytes())?;
        }
        Ok(())
    }

    /// Moves a regular file to an unused path inside the workspace.
    ///
    /// # Errors
    /// Returns path or file errors; an existing destination is never replaced.
    pub fn move_file(&self, from: &str, to: &str) -> Result<(), EditError> {
        let source = self.resolve_mutation_path(from)?;
        let target = self.resolve_mutation_path(to)?;
        if !source.is_file() {
            return Err(EditError::NotFile);
        }
        if target.exists() {
            return Err(EditError::DestinationExists);
        }
        fs::create_dir_all(target.parent().ok_or(EditError::NotFile)?)?;
        fs::rename(source, target)?;
        Ok(())
    }

    /// Deletes one regular file inside the workspace.
    ///
    /// # Errors
    /// Returns path or file errors.
    pub fn delete(&self, path: &str) -> Result<(), EditError> {
        let target = self.resolve_mutation_path(path)?;
        if !target.is_file() {
            return Err(EditError::NotFile);
        }
        fs::remove_file(target)?;
        Ok(())
    }

    fn resolve_mutation_path(&self, path: &str) -> Result<PathBuf, EditError> {
        if Path::new(path)
            .components()
            .any(|part| matches!(part, Component::Normal(name) if name == ".git"))
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "path leaves the workspace",
            )
            .into());
        }
        Ok(self.guard.resolve(path)?)
    }
}

fn replace_once(mut text: String, spec: &EditSpec) -> Result<String, EditError> {
    if spec.old.is_empty() {
        return Err(EditError::EmptyPattern);
    }
    let first = text.find(&spec.old).ok_or(EditError::MissingMatch)?;
    let next_char = spec.old.chars().next().ok_or(EditError::EmptyPattern)?;
    if text[first + next_char.len_utf8()..].contains(&spec.old) {
        return Err(EditError::AmbiguousMatch);
    }
    text.replace_range(first..first + spec.old.len(), &spec.new);
    if text.len() > MAX_TEXT_BYTES {
        return Err(EditError::TooLarge);
    }
    Ok(text)
}

fn remove_temporary_file(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let counter = NEXT.fetch_add(1, Ordering::Relaxed);
    let temporary: PathBuf =
        path.with_extension(format!("tmp-{}-{stamp}-{counter}", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let permissions = match fs::metadata(path) {
        Ok(metadata) => Some(metadata.permissions()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let result = (|| {
        let mut file = options.open(&temporary)?;
        file.write_all(bytes)?;
        if let Some(permissions) = permissions {
            file.set_permissions(permissions)?;
        }
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        remove_temporary_file(&temporary)?;
    }
    result
}

#[cfg(test)]
mod tests;
