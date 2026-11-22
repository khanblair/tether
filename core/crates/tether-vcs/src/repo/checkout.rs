use std::fs;
use std::os::unix::fs::PermissionsExt;

use super::RepoError;

pub(super) fn checkout_tree(
    repo: &gix::Repository,
    commit: gix::hash::ObjectId,
) -> Result<(), RepoError> {
    let tree = repo
        .find_commit(commit)
        .map_err(|_| RepoError::Git)?
        .tree()
        .map_err(|_| RepoError::Git)?
        .id;
    let mut index = repo.index_from_tree(&tree).map_err(|_| RepoError::Git)?;
    let old_paths: Vec<_> = repo
        .open_index()
        .map_err(|_| RepoError::Git)?
        .entries_with_paths_by_filter_map(|_path, entry| {
            (entry.stage() == gix::index::entry::Stage::Unconflicted).then_some(())
        })
        .map(|(path, ())| path.to_string())
        .collect();
    let new_paths: std::collections::BTreeSet<_> = index
        .entries_with_paths_by_filter_map(|_path, entry| {
            (entry.stage() == gix::index::entry::Stage::Unconflicted).then_some(())
        })
        .map(|(path, ())| path.to_string())
        .collect();
    for path in old_paths {
        if !new_paths.contains(&path) {
            drop(fs::remove_file(
                repo.workdir().ok_or(RepoError::Git)?.join(path),
            ));
        }
    }
    for (path, entry) in index.entries_with_paths_by_filter_map(|_path, entry| {
        (entry.stage() == gix::index::entry::Stage::Unconflicted).then_some(entry.clone())
    }) {
        let blob = repo.find_blob(entry.id).map_err(|_| RepoError::Git)?;
        let full = repo.workdir().ok_or(RepoError::Git)?.join(path.to_string());
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).map_err(|_| RepoError::Io)?;
        }
        fs::write(&full, &blob.data).map_err(|_| RepoError::Io)?;
        let mut permissions = fs::metadata(&full)
            .map_err(|_| RepoError::Io)?
            .permissions();
        permissions.set_mode(if entry.mode == gix::index::entry::Mode::FILE_EXECUTABLE {
            0o755
        } else {
            0o644
        });
        fs::set_permissions(full, permissions).map_err(|_| RepoError::Io)?;
    }
    index
        .write(gix::index::write::Options::default())
        .map_err(|_| RepoError::Git)
}
