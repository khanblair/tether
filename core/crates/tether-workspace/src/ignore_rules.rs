//! Git-compatible ignore rules shared by the file index and archives.
//!
//! Precedence, highest first: `.tetherignore`, `.gitignore` files from the deepest folder up to
//! the project root, `.git/info/exclude`, then Tether's built-in defaults. A symlinked rules file
//! is never read, and `.git` is hidden at every depth. A folder that is ignored is not entered,
//! as in Git, so a file inside it cannot be re-included.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

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

/// One regular file found by a walk.
#[derive(Debug, Clone)]
pub(crate) struct WalkedFile {
    /// Absolute path under the project root.
    pub absolute: PathBuf,
    /// Path relative to the project root.
    pub relative: PathBuf,
    /// Size in bytes.
    pub bytes: u64,
}

/// The files a walk kept and the symlinks it skipped.
#[derive(Debug, Default)]
pub(crate) struct Walk {
    /// Regular files in walk order.
    pub files: Vec<WalkedFile>,
    /// Symlinks that were not followed or listed.
    pub skipped_symlinks: u64,
}

/// Ignore rules loaded from one project folder.
#[derive(Debug, Clone)]
pub(crate) struct IgnoreRules {
    root: PathBuf,
    tether: Gitignore,
    builtin: Gitignore,
    exclude: Option<Gitignore>,
    /// The `.gitignore` of each walked folder, keyed by its path relative to the root.
    folders: HashMap<PathBuf, Gitignore>,
}

impl IgnoreRules {
    /// Reads the root-level rule files.
    ///
    /// # Errors
    /// Returns an error when a regular rules file cannot be read.
    pub(crate) fn load(root: &Path) -> io::Result<Self> {
        let mut builtin = GitignoreBuilder::new(root);
        for pattern in BUILTIN_IGNORES {
            builtin
                .add_line(None, pattern)
                .map_err(|error| io::Error::other(error.to_string()))?;
        }
        let builtin = builtin
            .build()
            .map_err(|error| io::Error::other(error.to_string()))?;
        Ok(Self {
            root: root.to_path_buf(),
            tether: read_rules(root, &root.join(".tetherignore"))?.unwrap_or_else(Gitignore::empty),
            builtin,
            exclude: read_rules(root, &root.join(".git/info/exclude"))?,
            folders: HashMap::new(),
        })
    }

    /// Lists regular files, skipping ignored paths unless `include_ignored` is set.
    ///
    /// `.git` is skipped either way. The walk also remembers each folder's `.gitignore` for
    /// [`IgnoreRules::is_ignored`].
    ///
    /// # Errors
    /// Returns an error when a folder or a regular rules file cannot be read.
    pub(crate) fn walk(&mut self, include_ignored: bool) -> io::Result<Walk> {
        let mut walker = Walker {
            rules: self,
            include_ignored,
            walk: Walk::default(),
            stack: vec![Folder {
                absolute: self.root.clone(),
                relative: PathBuf::new(),
                chain: Vec::new(),
            }],
            folders: HashMap::new(),
        };
        while let Some(folder) = walker.stack.pop() {
            walker.visit(folder)?;
        }
        let Walker { walk, folders, .. } = walker;
        self.folders = folders;
        Ok(walk)
    }

    /// Whether a path, or any folder above it, is ignored by the rules seen in the last walk.
    #[must_use]
    pub(crate) fn is_ignored(&self, relative: &Path, is_dir: bool) -> bool {
        if relative.components().any(|part| part.as_os_str() == ".git") {
            return true;
        }
        any_component_ignored(self, relative, is_dir)
    }

    fn decide(&self, path: &Path, is_dir: bool, chain: &[&Gitignore]) -> bool {
        let layers = std::iter::once(&self.tether)
            .chain(chain.iter().rev().copied())
            .chain(self.exclude.as_ref());
        let decided = layers
            .map(|rules| rules.matched(path, is_dir))
            .find(|found| !found.is_none());
        match decided {
            Some(found) => found.is_ignore(),
            None => self.builtin.matched(path, is_dir).is_ignore(),
        }
    }
}

/// Checks each folder on the way down to `relative`, then the path itself, and stops at the
/// first ignored one.
fn any_component_ignored(rules: &IgnoreRules, relative: &Path, is_dir: bool) -> bool {
    let mut chain: Vec<&Gitignore> = rules.folders.get(Path::new("")).into_iter().collect();
    let mut current = PathBuf::new();
    let depth = relative.components().count();
    for (index, part) in relative.components().enumerate() {
        current.push(part);
        let component_is_dir = index + 1 < depth || is_dir;
        if rules.decide(&rules.root.join(&current), component_is_dir, &chain) {
            return true;
        }
        chain.extend(rules.folders.get(&current));
    }
    false
}

/// A folder waiting to be walked, with the `.gitignore` files that apply to it.
struct Folder {
    absolute: PathBuf,
    relative: PathBuf,
    chain: Vec<Gitignore>,
}

struct Walker<'a> {
    rules: &'a IgnoreRules,
    include_ignored: bool,
    walk: Walk,
    stack: Vec<Folder>,
    folders: HashMap<PathBuf, Gitignore>,
}

impl Walker<'_> {
    fn visit(&mut self, mut folder: Folder) -> io::Result<()> {
        let rules = read_rules(&folder.absolute, &folder.absolute.join(".gitignore"))?;
        if let Some(rules) = rules {
            folder.chain.push(rules.clone());
            self.folders.insert(folder.relative.clone(), rules);
        }
        for item in fs::read_dir(&folder.absolute)? {
            self.entry(&folder, &item?)?;
        }
        Ok(())
    }

    fn entry(&mut self, folder: &Folder, item: &fs::DirEntry) -> io::Result<()> {
        let name = item.file_name();
        if name == ".git" {
            return Ok(());
        }
        let kind = item.file_type()?;
        if kind.is_symlink() {
            self.walk.skipped_symlinks += 1;
            return Ok(());
        }
        let path = item.path();
        let chain: Vec<&Gitignore> = folder.chain.iter().collect();
        if !self.include_ignored && self.rules.decide(&path, kind.is_dir(), &chain) {
            return Ok(());
        }
        let relative = folder.relative.join(&name);
        if kind.is_dir() {
            self.stack.push(Folder {
                absolute: path,
                relative,
                chain: folder.chain.clone(),
            });
        } else if kind.is_file() {
            self.walk.files.push(WalkedFile {
                bytes: item.metadata()?.len(),
                absolute: path,
                relative,
            });
        }
        Ok(())
    }
}

/// Reads a rules file when it is a regular file; a symlink or a missing file gives `None`.
fn read_rules(folder: &Path, path: &Path) -> io::Result<Option<Gitignore>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    }
    File::open(path)?;
    let mut builder = GitignoreBuilder::new(folder);
    // A line Git would reject is dropped and the rest of the file still applies.
    let _partial = builder.add(path);
    builder
        .build()
        .map(Some)
        .map_err(|error| io::Error::other(error.to_string()))
}
