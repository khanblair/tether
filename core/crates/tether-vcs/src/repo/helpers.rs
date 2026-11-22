use super::diff::tree_diff;
use super::{
    BTreeMap, CommitInfo, Component, FileState, ObjectId, Path, Repo, RepoError, StatusEntry,
    WorktreeItem, fs,
};

pub(super) fn add_tree_change(
    map: &mut BTreeMap<String, StatusEntry>,
    change: gix::diff::index::ChangeRef<'_, '_>,
) {
    let (path, state) = match change {
        gix::diff::index::ChangeRef::Addition { location, .. } => {
            (location.to_string(), FileState::Added)
        }
        gix::diff::index::ChangeRef::Deletion { location, .. } => {
            (location.to_string(), FileState::Deleted)
        }
        gix::diff::index::ChangeRef::Modification { location, .. } => {
            (location.to_string(), FileState::Modified)
        }
        gix::diff::index::ChangeRef::Rewrite { location, .. } => {
            (location.to_string(), FileState::Renamed)
        }
    };
    map.entry(path.clone())
        .or_insert(StatusEntry {
            path,
            index_state: None,
            worktree_state: None,
        })
        .index_state = Some(state);
}

pub(super) fn add_worktree_change(map: &mut BTreeMap<String, StatusEntry>, item: WorktreeItem) {
    match item {
        WorktreeItem::Modification {
            rela_path, status, ..
        } => {
            let state = match status {
                gix::status::plumbing::index_as_worktree::EntryStatus::Conflict { .. } => {
                    FileState::Conflicted
                }
                gix::status::plumbing::index_as_worktree::EntryStatus::Change(
                    gix::status::plumbing::index_as_worktree::Change::Removed,
                ) => FileState::Deleted,
                gix::status::plumbing::index_as_worktree::EntryStatus::Change(_) => {
                    FileState::Modified
                }
                _ => return,
            };
            put_work(map, rela_path.to_string(), state);
        }
        WorktreeItem::DirectoryContents { entry, .. }
            if matches!(entry.status, gix::dir::entry::Status::Untracked) =>
        {
            put_work(map, entry.rela_path.to_string(), FileState::Untracked);
        }
        WorktreeItem::Rewrite { dirwalk_entry, .. } => {
            put_work(map, dirwalk_entry.rela_path.to_string(), FileState::Renamed);
        }
        WorktreeItem::DirectoryContents { .. } => {}
    }
}
pub(super) fn put_work(map: &mut BTreeMap<String, StatusEntry>, path: String, state: FileState) {
    map.entry(path.clone())
        .or_insert(StatusEntry {
            path,
            index_state: None,
            worktree_state: None,
        })
        .worktree_state = Some(state);
}

pub(super) fn validate_relative(path: &str) -> Result<(), RepoError> {
    let p = Path::new(path);
    if path.is_empty()
        || p.is_absolute()
        || p.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            ) || c.as_os_str() == ".git"
        })
    {
        Err(RepoError::InvalidName)
    } else {
        Ok(())
    }
}

/// Validates a git branch name using gitoxide's ref validator.
///
/// # Errors
/// Returns [`RepoError::InvalidName`] when the name cannot be a Git ref.
pub fn validate_branch_name(name: &str) -> Result<(), RepoError> {
    if name.is_empty()
        || name.starts_with('-')
        || std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lock"))
        || name.ends_with('/')
        || name.contains("..")
        || name
            .chars()
            .any(|c| matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\' | '\0'))
    {
        return Err(RepoError::InvalidName);
    }
    let full_name = format!("refs/heads/{name}");
    gix::validate::reference::branch_name(full_name.as_bytes().into())
        .map(|_| ())
        .map_err(|_| RepoError::InvalidName)
}

/// Creates the default task branch name.
#[must_use]
pub fn branch_name_for_task(text: &str) -> String {
    let mut slug = String::new();
    let mut dash = false;
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            push_slug_alphanumeric(&mut slug, c);
            dash = false;
        } else {
            dash = push_slug_separator(&mut slug, dash);
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        slug.push_str("work");
    }
    format!("tether/{slug}")
}

fn push_slug_alphanumeric(slug: &mut String, character: char) {
    if slug.len() < 40 {
        slug.push(character);
    }
}

fn push_slug_separator(slug: &mut String, already_dash: bool) -> bool {
    if !slug.is_empty() && !already_dash && slug.len() < 40 {
        slug.push('-');
        true
    } else {
        already_dash
    }
}

/// Adds a numeric suffix until `base` is unused.
///
/// # Errors
/// Returns an error when `base` is invalid or refs cannot be read.
pub fn unique_branch_name(repo: &Repo, base: &str) -> Result<String, RepoError> {
    validate_branch_name(base)?;
    let existing: std::collections::BTreeSet<_> =
        repo.branches()?.into_iter().map(|b| b.name).collect();
    if !existing.contains(base) {
        return Ok(base.to_owned());
    }
    for n in 2..=u32::MAX {
        let candidate = format!("{base}-{n}");
        if !existing.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(RepoError::Git)
}

pub(super) fn update_index(
    index: &mut gix::index::File,
    path: &str,
    id: ObjectId,
    mode: gix::index::entry::Mode,
) {
    remove_index_entry(index, path);
    index.dangerously_push_entry(
        gix::index::entry::Stat::default(),
        id,
        gix::index::entry::Flags::empty(),
        mode,
        path.into(),
    );
    index.sort_entries();
}
pub(super) fn remove_index_entry(index: &mut gix::index::File, path: &str) {
    index.remove_entries(|_, p, _| p == path);
}
pub(super) fn remove_from_index(repo: &gix::Repository, path: &str) -> Result<(), RepoError> {
    let mut index = repo.open_index().map_err(|_| RepoError::Git)?;
    remove_index_entry(&mut index, path);
    index
        .write(gix::index::write::Options::default())
        .map_err(|_| RepoError::Git)
}
pub(super) fn unstage_one(repo: &gix::Repository, path: &str) -> Result<(), RepoError> {
    let mut index = repo.open_index().map_err(|_| RepoError::Git)?;
    remove_index_entry(&mut index, path);
    if let Ok(head) =
        repo.index_from_tree(&repo.head_tree_id_or_empty().map_err(|_| RepoError::Git)?)
        && let Some((_, entry)) = head
            .entries_with_paths_by_filter_map(|p, e| (p == path).then_some(e.clone()))
            .next()
    {
        let mode = entry.mode;
        index.dangerously_push_entry(entry.stat, entry.id, entry.flags, mode, path.into());
        index.sort_entries();
    }
    index
        .write(gix::index::write::Options::default())
        .map_err(|_| RepoError::Git)
}
pub(super) fn index_tree(repo: &gix::Repository) -> Result<ObjectId, RepoError> {
    let index = repo.open_index().map_err(|_| RepoError::Git)?;
    index_tree_from(repo, &index)
}
pub(super) fn index_tree_from(
    repo: &gix::Repository,
    index: &gix::index::File,
) -> Result<ObjectId, RepoError> {
    let mut editor = repo.empty_tree().edit().map_err(|_| RepoError::Git)?;
    for (path, entry) in index.entries_with_paths_by_filter_map(|_p, e| {
        (e.stage() == gix::index::entry::Stage::Unconflicted).then_some(e.clone())
    }) {
        let kind = entry
            .mode
            .to_tree_entry_mode()
            .ok_or(RepoError::Git)?
            .into();
        editor
            .upsert(path, kind, entry.id)
            .map_err(|_| RepoError::Git)?;
    }
    Ok(editor.write().map_err(|_| RepoError::Git)?.detach())
}
pub(super) fn tree_for(repo: &gix::Repository, name: &str) -> Result<ObjectId, RepoError> {
    Ok(repo
        .rev_parse_single(name)
        .map_err(|_| RepoError::RefNotFound)?
        .object()
        .map_err(|_| RepoError::Git)?
        .peel_to_tree()
        .map_err(|_| RepoError::Git)?
        .id)
}
pub(super) fn worktree_diff(
    repo: &gix::Repository,
    old: Option<ObjectId>,
    _new: Option<ObjectId>,
) -> Result<String, RepoError> {
    let index = index_tree(repo)?;
    let worktree = worktree_tree(repo)?;
    let from = old.unwrap_or(index);
    Ok(tree_diff(repo, from, worktree))
}

fn worktree_tree(repo: &gix::Repository) -> Result<ObjectId, RepoError> {
    let mut index = repo.open_index().map_err(|_| RepoError::Git)?;
    let paths: Vec<_> = index
        .entries_with_paths_by_filter_map(|_path, entry| {
            (entry.stage() == gix::index::entry::Stage::Unconflicted).then_some(())
        })
        .map(|(path, ())| path.to_string())
        .collect();
    for path in paths {
        let full = repo.workdir().ok_or(RepoError::Git)?.join(&path);
        if full.is_file() {
            let data = fs::read(&full).map_err(|_| RepoError::Io)?;
            let id = repo.write_blob(data).map_err(|_| RepoError::Git)?.detach();
            update_index(&mut index, &path, id, file_mode(&full));
        } else {
            remove_index_entry(&mut index, &path);
        }
    }
    index_tree_from(repo, &index)
}

fn file_mode(path: &Path) -> gix::index::entry::Mode {
    if executable(path) {
        gix::index::entry::Mode::FILE_EXECUTABLE
    } else {
        gix::index::entry::Mode::FILE
    }
}
pub(super) fn commit_info(
    commit: &gix::Commit<'_>,
    decoded: &gix::objs::CommitRef<'_>,
) -> CommitInfo {
    let author = commit.author().ok();
    let time = commit
        .committer()
        .ok()
        .and_then(|s| s.time().ok())
        .map(|t| t.seconds)
        .unwrap_or_default();
    let message = decoded.message.to_string();
    let subject = message.lines().next().unwrap_or_default().to_owned();
    CommitInfo {
        id: commit.id.to_string(),
        parents: commit.parent_ids().map(|p| p.to_string()).collect(),
        author_name: author
            .as_ref()
            .map(|s| s.name.to_string())
            .unwrap_or_default(),
        author_email: author.map(|s| s.email.to_string()).unwrap_or_default(),
        committer_time: time,
        subject,
        message,
    }
}
pub(super) fn executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            m.permissions().mode() & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            false
        }
    })
}
