//! Staging: which files a stage-all adds, and how paths are written to the index.
//!
//! The index is read from disk once and written once per call. Reading it through the
//! repository's cached copy between writes can serve a stale index when the index file's
//! timestamp has not changed yet, which drops the entry the previous write added.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use gix::worktree::stack::state::ignore::Source;

use super::RepoError;
use super::helpers::{executable, remove_index_entry, update_index};

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

/// Stages one path. See [`stage_paths`].
pub(super) fn stage_one(repo: &gix::Repository, path: &str) -> Result<(), RepoError> {
    stage_paths(repo, &[path.to_owned()])
}

/// Stages many paths with one index read and one index write.
///
/// A regular file is stored as its content, a symlink as its link text (never the file it
/// points to), and a missing path is removed from the index. A path that passes through a
/// symlink is refused.
pub(super) fn stage_paths(repo: &gix::Repository, paths: &[String]) -> Result<(), RepoError> {
    let workdir = repo.workdir().ok_or(RepoError::Git)?;
    let mut index = read_index(repo)?;
    for path in paths {
        reject_symlinked_ancestors(workdir, path)?;
        stage_into(repo, &mut index, workdir, path)?;
    }
    index
        .write(gix::index::write::Options::default())
        .map_err(|_| RepoError::Git)
}

/// The index as it is on disk now, or the `HEAD` tree (or nothing) when no index file exists.
fn read_index(repo: &gix::Repository) -> Result<gix::index::File, RepoError> {
    match repo.open_index() {
        Ok(index) => Ok(index),
        Err(_) => repo
            .index_or_load_from_head_or_empty()
            .map(gix::worktree::IndexPersistedOrInMemory::into_owned)
            .map_err(|_| RepoError::Git),
    }
}

fn stage_into(
    repo: &gix::Repository,
    index: &mut gix::index::File,
    workdir: &Path,
    path: &str,
) -> Result<(), RepoError> {
    let full = workdir.join(path);
    match fs::symlink_metadata(&full) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = fs::read_link(&full).map_err(|_| RepoError::Io)?;
            let id = repo
                .write_blob(target.as_os_str().as_encoded_bytes())
                .map_err(|_| RepoError::Git)?
                .detach();
            update_index(index, path, id, gix::index::entry::Mode::SYMLINK);
        }
        Ok(meta) if meta.is_file() => {
            let bytes = fs::read(&full).map_err(|_| RepoError::Io)?;
            let id = repo.write_blob(bytes).map_err(|_| RepoError::Git)?.detach();
            let mode = if executable(&full) {
                gix::index::entry::Mode::FILE_EXECUTABLE
            } else {
                gix::index::entry::Mode::FILE
            };
            update_index(index, path, id, mode);
        }
        Ok(_) => remove_index_entry(index, path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            remove_index_entry(index, path);
        }
        Err(_) => return Err(RepoError::Io),
    }
    Ok(())
}

/// Refuses a path that passes through a symlink, so staging never reads outside the repository.
fn reject_symlinked_ancestors(workdir: &Path, path: &str) -> Result<(), RepoError> {
    let mut current = workdir.to_path_buf();
    let parts: Vec<_> = Path::new(path).components().collect();
    for part in &parts[..parts.len().saturating_sub(1)] {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => return Err(RepoError::InvalidName),
            Ok(_) => {}
            Err(_) => return Ok(()),
        }
    }
    Ok(())
}
