//! Entry name validation for ZIP extraction.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::ArchiveError;

/// One archive entry after its name passed validation.
#[derive(Debug, Clone)]
pub(super) struct EntryInfo {
    /// Position in the archive.
    pub index: usize,
    /// Safe relative path.
    pub path: PathBuf,
    /// Whether the entry is a directory.
    pub is_dir: bool,
    /// Whether the entry is a symbolic link, which extraction skips.
    pub is_symlink: bool,
}

/// Turns an entry name into a safe relative path and says whether it names a directory.
///
/// Rejects empty names, NUL, backslashes, absolute paths, drive letters, and any empty, `.`,
/// or `..` component.
pub(super) fn safe_entry_name(name: &str) -> Result<(PathBuf, bool), ArchiveError> {
    if name.is_empty() || name.contains('\0') || name.contains('\\') {
        return Err(ArchiveError::InvalidEntryName);
    }
    let is_dir = name.ends_with('/');
    let trimmed = name.trim_end_matches('/');
    if trimmed.is_empty() || Path::new(trimmed).is_absolute() {
        return Err(ArchiveError::InvalidEntryName);
    }
    let mut path = PathBuf::new();
    for (index, component) in trimmed.split('/').enumerate() {
        let drive_letter = index == 0 && component.ends_with(':');
        if component.is_empty() || component == "." || component == ".." || drive_letter {
            return Err(ArchiveError::InvalidEntryName);
        }
        path.push(component);
    }
    Ok((path, is_dir))
}

/// Rejects duplicate paths and a file that is also used as a folder, in any entry order.
pub(super) fn reject_collisions(entries: &[EntryInfo]) -> Result<(), ArchiveError> {
    let mut kinds: HashMap<&Path, bool> = HashMap::with_capacity(entries.len());
    for entry in entries {
        if kinds.insert(entry.path.as_path(), entry.is_dir).is_some() {
            return Err(ArchiveError::InvalidEntryName);
        }
    }
    let file_under_folder_use = entries.iter().any(|entry| {
        entry
            .path
            .ancestors()
            .skip(1)
            .any(|ancestor| kinds.get(ancestor) == Some(&false))
    });
    if file_under_folder_use {
        return Err(ArchiveError::InvalidEntryName);
    }
    Ok(())
}

/// Drops the one folder that holds every entry, when there is such a folder.
///
/// An archive whose entries are not all inside one shared folder, including one that holds a
/// single file at its top level, is left unchanged. The folder's own entry is removed.
pub(super) fn strip_single_top_level(entries: &mut Vec<EntryInfo>) {
    let Some(top) = shared_top_folder(entries) else {
        return;
    };
    entries.retain(|entry| entry.path != top);
    for entry in entries {
        if let Ok(rest) = entry.path.strip_prefix(&top) {
            entry.path = rest.to_path_buf();
        }
    }
}

fn shared_top_folder(entries: &[EntryInfo]) -> Option<PathBuf> {
    let mut top: Option<PathBuf> = None;
    for entry in entries {
        let first = PathBuf::from(entry.path.components().next()?.as_os_str());
        let single = entry.path == first;
        if single && !entry.is_dir {
            return None;
        }
        match &top {
            None => top = Some(first),
            Some(existing) if *existing == first => {}
            Some(_) => return None,
        }
    }
    top
}
