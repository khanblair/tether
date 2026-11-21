//! File operations and bounded queries over the vault view.

use std::collections::{BTreeSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use super::{VaultError, VaultIndex, VaultLink, VaultNote, parse_note};

const MAX_NOTE_BYTES: usize = 128 * 1024;

/// Inbound and outbound links for a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteLinks {
    /// Links written inside the note.
    pub outgoing: Vec<VaultLink>,
    /// Links in other notes that point here.
    pub incoming: Vec<VaultLink>,
}

/// File backed vault operations. Each read rebuilds from the current files.
#[derive(Debug, Clone)]
pub struct VaultStore {
    root: PathBuf,
}

impl VaultStore {
    /// Creates a view over the private vault folder.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Loads the current note and link index.
    ///
    /// # Errors
    /// Returns unreadable or invalid notes.
    pub fn index(&self) -> Result<VaultIndex, VaultError> {
        VaultIndex::rebuild(&self.root)
    }

    /// Searches note titles, descriptions, tags, and source text.
    ///
    /// # Errors
    /// Returns unreadable or invalid notes.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<VaultNote>, VaultError> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        Ok(self
            .index()?
            .notes
            .into_values()
            .filter(|note| {
                note.title.to_lowercase().contains(&needle)
                    || note.description.to_lowercase().contains(&needle)
                    || note
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&needle))
                    || note.source.to_lowercase().contains(&needle)
            })
            .take(limit.min(100))
            .collect())
    }

    /// Reads one note from the rebuilt view.
    ///
    /// # Errors
    /// Returns invalid paths or unreadable notes.
    pub fn read(&self, path: &str) -> Result<Option<VaultNote>, VaultError> {
        valid_relative(path)?;
        let mut index = self.index()?;
        Ok(index.notes.remove(path))
    }

    /// Replaces one note after validating its frontmatter and byte cap.
    ///
    /// # Errors
    /// Returns invalid paths, note content, or file errors.
    pub fn write(&self, path: &str, source: &str) -> Result<(), VaultError> {
        let target = self.target_path(path)?;
        validate_note(path, source)?;
        fs::create_dir_all(target.parent().ok_or(VaultError::InvalidPath)?)?;
        atomic_write(&target, source.as_bytes())?;
        Ok(())
    }

    /// Appends text to an existing note, then validates the full result.
    ///
    /// # Errors
    /// Returns missing notes, invalid content, or file errors.
    pub fn append(&self, path: &str, addition: &str) -> Result<(), VaultError> {
        let target = self.target_path(path)?;
        let mut source = fs::read_to_string(&target).map_err(missing_or_io)?;
        source.push_str(addition);
        validate_note(path, &source)?;
        atomic_write(&target, source.as_bytes())?;
        Ok(())
    }

    /// Returns a note's outlinks and backlinks.
    ///
    /// # Errors
    /// Returns invalid paths or unreadable notes.
    pub fn links(&self, path: &str) -> Result<NoteLinks, VaultError> {
        valid_relative(path)?;
        let index = self.index()?;
        if !index.notes.contains_key(path) {
            return Err(VaultError::MissingNote);
        }
        Ok(NoteLinks {
            outgoing: index
                .links
                .iter()
                .filter(|link| link.source_path == path)
                .cloned()
                .collect(),
            incoming: index.backlinks(path).into_iter().cloned().collect(),
        })
    }

    /// Lists notes with no incoming links.
    ///
    /// # Errors
    /// Returns unreadable or invalid notes.
    pub fn orphans(&self) -> Result<Vec<String>, VaultError> {
        let index = self.index()?;
        Ok(index
            .notes
            .keys()
            .filter(|path| index.backlinks(path).is_empty())
            .cloned()
            .collect())
    }

    /// Renders a small neighborhood. UTF-8 bytes are capped by `token_budget`, a conservative
    /// upper bound on tokenizer output for ordinary text.
    ///
    /// # Errors
    /// Returns invalid paths or unreadable notes.
    pub fn map(
        &self,
        start: &str,
        depth: usize,
        token_budget: usize,
    ) -> Result<String, VaultError> {
        valid_relative(start)?;
        let index = self.index()?;
        if !index.notes.contains_key(start) {
            return Err(VaultError::MissingNote);
        }
        Ok(render_map(&index, start, depth, token_budget))
    }

    fn target_path(&self, path: &str) -> Result<PathBuf, VaultError> {
        checked_target_path(&self.root, path)
    }
}

fn render_map(index: &VaultIndex, start: &str, depth: usize, token_budget: usize) -> String {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([(start.to_owned(), 0)]);
    let mut output = String::new();
    while let Some((path, level)) = queue.pop_front() {
        if !visited.insert(path.clone()) {
            continue;
        }
        let note = &index.notes[&path];
        let line = format!("{}: {}\n", note.path, note.description);
        if output.len() + line.len() > token_budget {
            break;
        }
        output.push_str(&line);
        if level >= depth {
            continue;
        }
        let next_level = level + 1;
        queue.extend(
            resolved_neighbors(index, &path)
                .into_iter()
                .filter(|next| !visited.contains(next))
                .map(|next| (next, next_level)),
        );
    }
    output
}

fn checked_target_path(root: &Path, path: &str) -> Result<PathBuf, VaultError> {
    valid_relative(path)?;
    if is_symlink(root) {
        return Err(VaultError::InvalidPath);
    }
    let mut target = root.to_path_buf();
    for part in Path::new(path).components() {
        target.push(part.as_os_str());
        if is_symlink(&target) {
            return Err(VaultError::InvalidPath);
        }
    }
    Ok(target)
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
}

fn missing_or_io(error: io::Error) -> VaultError {
    if error.kind() == io::ErrorKind::NotFound {
        VaultError::MissingNote
    } else {
        VaultError::Io(error)
    }
}

/// Paths of the notes that a resolved link in `path` points to, in link order.
fn resolved_neighbors(index: &VaultIndex, path: &str) -> Vec<String> {
    index
        .links
        .iter()
        .filter(|link| link.source_path == path)
        .filter_map(|link| link.resolved_path.clone())
        .collect()
}

fn valid_relative(path: &str) -> Result<(), VaultError> {
    let input = Path::new(path);
    if input.extension().and_then(|ext| ext.to_str()) != Some("md")
        || !input
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        || path.starts_with('.')
        || path.contains("/.obsidian/")
    {
        return Err(VaultError::InvalidPath);
    }
    Ok(())
}

fn validate_note(path: &str, source: &str) -> Result<(), VaultError> {
    if source.len() > MAX_NOTE_BYTES {
        return Err(VaultError::OverCap);
    }
    parse_note(path, source.to_owned()).ok_or(VaultError::InvalidNote {
        path: path.to_owned(),
    })?;
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
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
mod tests {
    use super::{VaultError, VaultStore};
    use std::fs;

    #[test]
    fn tool_contracts_read_write_append_search_links_orphans_and_map() {
        let stamp = crate::unique_id();
        let root = std::env::temp_dir().join(format!("tether-vault-tools-{stamp}"));
        let store = VaultStore::new(&root);
        store
            .write(
                "Map.md",
                "---\ndescription: Entry map\ntype: moc\n---\n[[Note]]",
            )
            .unwrap();
        store
            .write(
                "Note.md",
                "---\ndescription: A useful note\ntags: [work]\n---\n# First",
            )
            .unwrap();
        assert_eq!(store.search("useful", 10).unwrap().len(), 1);
        assert_eq!(store.read("Note.md").unwrap().unwrap().title, "Note");
        store.append("Note.md", "\nA new line").unwrap();
        assert!(
            store
                .read("Note.md")
                .unwrap()
                .unwrap()
                .source
                .contains("A new line")
        );
        let links = store.links("Note.md").unwrap();
        assert_eq!(links.incoming.len(), 1);
        assert_eq!(links.outgoing, []);
        assert_eq!(store.orphans().unwrap(), vec!["Map.md"]);
        assert!(store.map("Map.md", 1, 100).unwrap().contains("Note.md"));
        assert!(store.map("Map.md", 1, 10).unwrap().len() <= 10);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_path_and_missing_description_never_write() {
        let stamp = crate::unique_id();
        let root = std::env::temp_dir().join(format!("tether-vault-guard-{stamp}"));
        let store = VaultStore::new(&root);
        let valid = "---\ndescription: Safe\n---\nText";
        assert!(matches!(
            store.write("../escape.md", valid),
            Err(VaultError::InvalidPath)
        ));
        assert!(matches!(
            store.write(".obsidian/config.md", valid),
            Err(VaultError::InvalidPath)
        ));
        assert!(matches!(
            store.write("No.md", "---\ntitle: No\n---"),
            Err(VaultError::InvalidNote { .. })
        ));
        assert!(!root.join("No.md").exists());
    }
}
