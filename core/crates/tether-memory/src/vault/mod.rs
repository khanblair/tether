//! Rebuildable view of Markdown notes and wikilinks in a private vault folder.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub mod sync;
mod tools;
pub use tools::{NoteLinks, VaultStore};

/// One parsed Markdown note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultNote {
    /// Relative Markdown path within the vault.
    pub path: String,
    /// Display title from frontmatter or filename.
    pub title: String,
    /// Required one-line description.
    pub description: String,
    /// Optional note category, including `moc`.
    pub note_type: Option<String>,
    /// Optional tags.
    pub tags: Vec<String>,
    /// Optional aliases.
    pub aliases: Vec<String>,
    /// Optional created date.
    pub created: Option<String>,
    /// Optional updated date.
    pub updated: Option<String>,
    /// Markdown headings without their leading markers.
    pub headings: Vec<String>,
    /// Original Markdown source.
    pub source: String,
}

/// A wikilink in one note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultLink {
    /// Path of the note containing the link.
    pub source_path: String,
    /// Note name as written in the link.
    pub target: String,
    /// Heading after `#`, when present.
    pub heading: Option<String>,
    /// Display alias after `|`, when present.
    pub alias: Option<String>,
    /// Whether the target note exists in the rebuilt index.
    pub resolved: bool,
}

/// One complete, deterministic view of the vault files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultIndex {
    /// Notes keyed by relative Markdown path.
    pub notes: BTreeMap<String, VaultNote>,
    /// Wikilinks in source path order.
    pub links: Vec<VaultLink>,
}

/// A vault read or parse failure.
#[derive(Debug)]
pub enum VaultError {
    /// Filesystem access failed.
    Io(io::Error),
    /// A note lacks valid frontmatter or a description.
    InvalidNote {
        /// Relative path of the invalid note.
        path: String,
    },
    /// A requested note does not exist.
    MissingNote,
    /// A path could escape the private vault or touch a symlink.
    InvalidPath,
    /// A note would exceed the file size cap.
    OverCap,
}

impl From<io::Error> for VaultError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl VaultIndex {
    /// Discards any prior index and parses the current files again.
    ///
    /// # Errors
    /// Returns a read error or a note with missing required metadata.
    pub fn rebuild(root: &Path) -> Result<Self, VaultError> {
        let mut paths = Vec::new();
        if root.exists() {
            collect_markdown(root, root, &mut paths)?;
        }
        paths.sort();
        let mut notes = BTreeMap::new();
        for (relative, absolute) in paths {
            let source = fs::read_to_string(absolute)?;
            let note = parse_note(&relative, source).ok_or(VaultError::InvalidNote {
                path: relative.clone(),
            })?;
            notes.insert(relative, note);
        }
        let links = resolve_links(&notes);
        Ok(Self { notes, links })
    }

    /// Returns links pointing to an existing note.
    #[must_use]
    pub fn backlinks(&self, path: &str) -> Vec<&VaultLink> {
        self.links
            .iter()
            .filter(|link| {
                link.resolved
                    && resolve_target(&self.notes, &link.source_path, &link.target)
                        == Some(path.to_owned())
            })
            .collect()
    }
}

/// Parses the links in every note and marks which ones point to an existing note.
fn resolve_links(notes: &BTreeMap<String, VaultNote>) -> Vec<VaultLink> {
    let mut links = Vec::new();
    for note in notes.values() {
        for mut link in parse_links(&note.path, &note.source) {
            link.resolved = resolve_target(notes, &note.path, &link.target).is_some();
            links.push(link);
        }
    }
    links
}

fn collect_markdown(
    root: &Path,
    folder: &Path,
    paths: &mut Vec<(String, PathBuf)>,
) -> io::Result<()> {
    for item in fs::read_dir(folder)? {
        let item = item?;
        let kind = item.file_type()?;
        let path = item.path();
        if kind.is_symlink() || item.file_name() == ".obsidian" {
            continue;
        }
        if kind.is_dir() {
            collect_markdown(root, &path, paths)?;
        } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            let relative = path
                .strip_prefix(root)
                .map_err(io::Error::other)?
                .to_string_lossy()
                .replace('\\', "/");
            paths.push((relative, path));
        }
    }
    Ok(())
}

fn parse_note(path: &str, source: String) -> Option<VaultNote> {
    let mut lines = source.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut fields = parse_fields(&mut lines)?;
    let description = fields.remove("description")?;
    if description.trim().is_empty() {
        return None;
    }
    let title = fields.remove("title").unwrap_or_else(|| {
        Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(path)
            .to_owned()
    });
    let headings = lines
        .filter_map(|line| {
            let level = line.bytes().take_while(|byte| *byte == b'#').count();
            if (1..=6).contains(&level) {
                line.get(level..).map(|text| text.trim().to_owned())
            } else {
                None
            }
        })
        .collect();
    Some(VaultNote {
        path: path.to_owned(),
        title,
        description,
        note_type: fields.remove("type"),
        tags: list(fields.remove("tags")),
        aliases: list(fields.remove("aliases")),
        created: fields.remove("created"),
        updated: fields.remove("updated"),
        headings,
        source,
    })
}

/// Reads `key: value` lines up to the closing `---`. Returns `None` when the block is malformed
/// or never closes.
fn parse_fields(lines: &mut std::str::Lines<'_>) -> Option<BTreeMap<String, String>> {
    let mut fields = BTreeMap::new();
    for line in lines {
        if line.trim() == "---" {
            return Some(fields);
        }
        if line.trim().is_empty() {
            continue;
        }
        let (key, value) = line.split_once(':')?;
        if key.trim().is_empty() || line.starts_with(' ') {
            return None;
        }
        fields.insert(key.trim().to_owned(), scalar(value.trim()));
    }
    None
}

fn scalar(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value)
        .to_owned()
}

fn list(value: Option<String>) -> Vec<String> {
    value
        .map(|v| {
            v.trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|item| scalar(item.trim()))
                .filter(|item| !item.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_links(source_path: &str, source: &str) -> Vec<VaultLink> {
    let mut links = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let inner = &rest[..end];
        let (destination, alias) = inner
            .split_once('|')
            .map_or((inner, None), |(left, right)| {
                (left, Some(right.to_owned()))
            });
        let (target, heading) = destination
            .split_once('#')
            .map_or((destination, None), |(left, right)| {
                (left, Some(right.to_owned()))
            });
        if !target.trim().is_empty() {
            links.push(VaultLink {
                source_path: source_path.to_owned(),
                target: target.trim().to_owned(),
                heading,
                alias,
                resolved: false,
            });
        }
        rest = &rest[end + 2..];
    }
    links
}

fn resolve_target(
    notes: &BTreeMap<String, VaultNote>,
    source: &str,
    target: &str,
) -> Option<String> {
    let target = target.trim_end_matches(".md");
    let exact = format!("{target}.md");
    if notes.contains_key(&exact) {
        return Some(exact);
    }
    let sibling = Path::new(source)
        .parent()
        .unwrap_or(Path::new(""))
        .join(&exact)
        .to_string_lossy()
        .replace('\\', "/");
    if notes.contains_key(&sibling) {
        return Some(sibling);
    }
    let matches: Vec<_> = notes
        .keys()
        .filter(|path| Path::new(path).file_stem().and_then(|stem| stem.to_str()) == Some(target))
        .collect();
    if matches.len() == 1 {
        matches.into_iter().next().cloned()
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
