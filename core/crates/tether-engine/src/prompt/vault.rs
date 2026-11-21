use std::fs;
use std::path::{Path, PathBuf};

use tether_memory::vault::VaultStore;

const VAULT_CAP: usize = 6_000;

/// Loads the bounded session slice from the vault.
///
/// # Errors
/// This helper reports an empty slice when the vault is empty, invalid, or unreadable.
#[must_use]
pub fn load_vault_slice(root: &Path, project_name: &str) -> String {
    let parsed = VaultStore::new(root).index().ok();
    let mut parts = Vec::new();
    if let Some(source) = read_vault_file(root, "index.md") {
        parts.push(format!(
            "Vault index, first 40 lines:\n{}",
            first_lines(&source, 40)
        ));
    }
    if let Some(index) = parsed.as_ref()
        && let Some(note) = find_map(index, project_name)
    {
        parts.push(format!("Active project map note:\n{}", note.source));
    }
    if let Some(source) = read_vault_file(root, "log.md") {
        parts.push(format!(
            "Recent vault log entries:\n{}",
            recent_lines(&source, 5)
        ));
    }
    cap_slice(&parts.join("\n\n"))
}

fn read_vault_file(root: &Path, name: &str) -> Option<String> {
    if root
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return None;
    }
    let path = PathBuf::from(root).join(name);
    if path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn find_map<'a>(
    index: &'a tether_memory::vault::VaultIndex,
    project: &str,
) -> Option<&'a tether_memory::vault::VaultNote> {
    index.notes.values().find(|note| {
        note.note_type.as_deref() == Some("moc")
            && (note.title.eq_ignore_ascii_case(project)
                || note
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(project)))
    })
}

fn first_lines(source: &str, count: usize) -> String {
    source.lines().take(count).collect::<Vec<_>>().join("\n")
}

fn recent_lines(source: &str, count: usize) -> String {
    let lines: Vec<_> = source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let start = lines.len().saturating_sub(count);
    lines[start..].join("\n")
}

fn cap_slice(source: &str) -> String {
    let mut end = source.len().min(VAULT_CAP);
    while !source.is_char_boundary(end) {
        end -= 1;
    }
    if end == source.len() {
        source.to_owned()
    } else {
        format!("{}\n[truncated]", &source[..end.saturating_sub(12)])
    }
}
