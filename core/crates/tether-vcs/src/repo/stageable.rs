//! Lists the files a stage-all would add, without relying on status classification.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use gix::worktree::stack::state::ignore::Source;

use super::RepoError;

type Excludes<'repo> = gix::AttributeStack<'repo>;

/// What a walk should do with one directory entry.
enum Step {
    Skip,
    Take,
    Enter,
}

/// Lists files to stage: every tracked file that still exists, and every untracked file that
/// no ignore rule excludes. Symlinks count as files and are never followed. `.git`, ignored
/// folders without tracked files, and symlinked folders are not entered.
pub(super) fn stageable_files(
    repo: &gix::Repository,
    root: &Path,
) -> Result<Vec<String>, RepoError> {
    let index = repo
        .index_or_load_from_head_or_empty()
        .map_err(|_| RepoError::Git)?;
    let tracked: BTreeSet<String> = index
        .entries_with_paths_by_filter_map(|_path, _entry| Some(()))
        .map(|(path, ())| path.to_string())
        .collect();
    let mut excludes = repo
        .excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped)
        .map_err(|_| RepoError::Git)?;
    let mut found = Vec::new();
    walk(root, "", &tracked, &mut excludes, &mut found)?;
    found.sort();
    Ok(found)
}

fn walk(
    root: &Path,
    relative_dir: &str,
    tracked: &BTreeSet<String>,
    excludes: &mut Excludes<'_>,
    found: &mut Vec<String>,
) -> Result<(), RepoError> {
    let dir = root.join(relative_dir);
    for item in fs::read_dir(&dir).map_err(|_| RepoError::Io)? {
        let item = item.map_err(|_| RepoError::Io)?;
        let name = item.file_name().to_string_lossy().into_owned();
        let relative = if relative_dir.is_empty() {
            name.clone()
        } else {
            format!("{relative_dir}/{name}")
        };
        let kind = item.file_type().map_err(|_| RepoError::Io)?;
        match step(&name, &relative, kind, tracked, excludes) {
            Step::Skip => {}
            Step::Take => found.push(relative),
            Step::Enter => walk(root, &relative, tracked, excludes, found)?,
        }
    }
    Ok(())
}

fn step(
    name: &str,
    relative: &str,
    kind: fs::FileType,
    tracked: &BTreeSet<String>,
    excludes: &mut Excludes<'_>,
) -> Step {
    if name == ".git" {
        return Step::Skip;
    }
    if kind.is_dir() {
        let prefix = format!("{relative}/");
        let holds_tracked = tracked
            .range(prefix.clone()..)
            .next()
            .is_some_and(|path| path.starts_with(&prefix));
        let mode = Some(gix::index::entry::Mode::DIR);
        let ignored_folder = is_excluded(excludes, relative, mode);
        return if ignored_folder && !holds_tracked {
            Step::Skip
        } else {
            Step::Enter
        };
    }
    if !kind.is_file() && !kind.is_symlink() {
        return Step::Skip;
    }
    let mode = if kind.is_symlink() {
        gix::index::entry::Mode::SYMLINK
    } else {
        gix::index::entry::Mode::FILE
    };
    if tracked.contains(relative) || !is_excluded(excludes, relative, Some(mode)) {
        Step::Take
    } else {
        Step::Skip
    }
}

/// Asks git's ignore rules; a lookup that fails counts as excluded so nothing doubtful is staged.
fn is_excluded(
    excludes: &mut Excludes<'_>,
    relative: &str,
    mode: Option<gix::index::entry::Mode>,
) -> bool {
    excludes
        .at_path(relative, mode)
        .map_or(true, |platform| platform.is_excluded())
}
