//! Writes to the two small memory files, with validation before persistence.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const SEPARATOR: &str = "\n§\n";

/// The only two files the memory tool may edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTarget {
    /// Agent notes.
    Memory,
    /// User preferences.
    User,
}

impl MemoryTarget {
    fn name(self) -> &'static str {
        match self {
            Self::Memory => "MEMORY.md",
            Self::User => "USER.md",
        }
    }

    fn cap(self) -> usize {
        match self {
            Self::Memory => 2_200,
            Self::User => 1_375,
        }
    }
}

/// A request from the `memory` tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryAction {
    /// Append a distinct entry.
    Add(String),
    /// Replace one exact entry.
    Patch {
        /// The entry to find, byte for byte.
        old: String,
        /// The entry that replaces it.
        new: String,
    },
    /// Remove one exact entry.
    Remove(String),
}

/// Result of a memory action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryResult {
    /// Whether disk content changed.
    pub changed: bool,
    /// Current file content after the action.
    pub content: String,
}

/// A rejected or failed memory operation.
#[derive(Debug)]
pub enum MemoryError {
    /// File I/O failed.
    Io(io::Error),
    /// The replacement target was not found.
    MissingEntry,
    /// The file would exceed its character cap.
    OverCap {
        /// The cap in characters for the target file.
        cap: usize,
    },
    /// The entry contains an injection, credential, or invisible character pattern.
    UnsafeContent,
    /// An entry has invalid separators or is empty.
    InvalidEntry,
}

impl From<io::Error> for MemoryError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Disk-backed memory scoped to a fixed directory.
#[derive(Debug)]
pub struct MemoryStore {
    root: PathBuf,
    lock: Mutex<()>,
}

impl MemoryStore {
    /// Creates a store; files are created on the first write.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            lock: Mutex::new(()),
        }
    }

    /// Reads one memory file without modifying it.
    ///
    /// # Errors
    /// Returns I/O failures other than a missing file.
    pub fn read(&self, target: MemoryTarget) -> Result<String, MemoryError> {
        read_file(&self.root.join(target.name()))
    }

    /// Applies an add, exact patch, or exact removal and writes it atomically.
    ///
    /// # Errors
    /// Rejects over-cap, duplicate-format, unsafe, or missing-entry operations.
    pub fn apply(
        &self,
        target: MemoryTarget,
        action: MemoryAction,
    ) -> Result<MemoryResult, MemoryError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| io::Error::other("memory lock poisoned"))?;
        let path = self.root.join(target.name());
        let original = read_file(&path)?;
        let mut entries: Vec<String> = original
            .split(SEPARATOR)
            .filter(|part| !part.trim().is_empty())
            .map(str::to_owned)
            .collect();
        if !apply_action(&mut entries, action)? {
            return Ok(MemoryResult {
                changed: false,
                content: original,
            });
        }
        for entry in &entries {
            validate_entry(entry)?;
        }
        let updated = entries.join(SEPARATOR);
        if updated.chars().count() > target.cap() {
            return Err(MemoryError::OverCap { cap: target.cap() });
        }
        if updated == original {
            return Ok(MemoryResult {
                changed: false,
                content: original,
            });
        }
        fs::create_dir_all(&self.root)?;
        write_atomic(&path, updated.as_bytes())?;
        Ok(MemoryResult {
            changed: true,
            content: updated,
        })
    }
}

/// Applies the action to the entry list. Returns `false` when nothing needs to change.
fn apply_action(entries: &mut Vec<String>, action: MemoryAction) -> Result<bool, MemoryError> {
    match action {
        MemoryAction::Add(entry) => {
            validate_entry(&entry)?;
            if entries.contains(&entry) {
                return Ok(false);
            }
            entries.push(entry);
        }
        MemoryAction::Patch { old, new } => {
            validate_entry(&new)?;
            let slot = entries
                .iter()
                .position(|entry| entry == &old)
                .ok_or(MemoryError::MissingEntry)?;
            entries[slot] = new;
        }
        MemoryAction::Remove(entry) => {
            let slot = entries
                .iter()
                .position(|item| item == &entry)
                .ok_or(MemoryError::MissingEntry)?;
            entries.remove(slot);
        }
    }
    Ok(true)
}

fn read_file(path: &Path) -> Result<String, MemoryError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}

fn validate_entry(entry: &str) -> Result<(), MemoryError> {
    if entry.trim().is_empty() || entry.contains(SEPARATOR) {
        return Err(MemoryError::InvalidEntry);
    }
    if entry.chars().any(is_invisible) || has_unsafe_pattern(entry) {
        return Err(MemoryError::UnsafeContent);
    }
    Ok(())
}

fn is_invisible(character: char) -> bool {
    matches!(character, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}')
}

fn has_unsafe_pattern(entry: &str) -> bool {
    let lower = entry.to_ascii_lowercase();
    [
        "ignore previous instructions",
        "ignore all previous instructions",
        "override instructions",
        "reveal the system prompt",
        "send secrets",
        "you are now",
        "act as the system",
        "sk-",
        "ghp_",
        "github_pat_",
        "aiza",
        "akia",
        "-----begin private key-----",
        "api_key=",
        "api_key:",
        "password=",
        "password:",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let stamp = crate::unique_id();
    let temporary = path.with_extension(format!("tmp-{stamp}"));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        // The write error is returned below. A failed cleanup must not replace it.
        fs::remove_file(&temporary).ok();
    }
    result
}

#[cfg(test)]
mod tests;
