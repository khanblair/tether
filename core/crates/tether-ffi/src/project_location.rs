//! Validation and persistence for the folder used by new projects.

use std::fs::{self, OpenOptions};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_store::{ProjectRecord, Store, project_dir};

use crate::projects::ProjectRuntimeError;

pub(crate) fn validate_root(
    default_root: &Path,
    candidate: &Path,
    projects: &[ProjectRecord],
) -> Result<PathBuf, ProjectRuntimeError> {
    if !candidate.is_absolute() {
        return Err(ProjectRuntimeError::RootNotAbsolute);
    }
    let candidate_path = comparison_path(candidate)
        .or_else(|| normalized(candidate))
        .ok_or(ProjectRuntimeError::RootUnavailable)?;
    let candidate_paths = [Some(candidate_path), normalized(candidate)];
    let default_paths = [comparison_path(default_root), normalized(default_root)];
    if candidate_paths.iter().flatten().any(|candidate| {
        default_paths
            .iter()
            .flatten()
            .any(|root| candidate == root || candidate.starts_with(root))
    }) {
        return Err(ProjectRuntimeError::RootInsidePrivateRoot);
    }
    if projects.iter().any(|project| {
        let existing = project_dir(default_root, project);
        let existing_paths = [comparison_path(&existing), normalized(&existing)];
        candidate_paths.iter().flatten().any(|candidate| {
            existing_paths
                .iter()
                .flatten()
                .any(|root| candidate == root || candidate.starts_with(root))
        })
    }) {
        return Err(ProjectRuntimeError::RootInsideProject);
    }
    let root = prepare_directory(candidate)?;
    if root.parent().is_none() {
        return Err(ProjectRuntimeError::RootIsFilesystemRoot);
    }
    check_writable(&root)?;
    Ok(root)
}

pub(crate) fn save_root(
    store: &Store,
    default_root: &Path,
    candidate: Option<&str>,
) -> Result<(), ProjectRuntimeError> {
    let root = match candidate {
        Some(candidate) => {
            let projects = store
                .list_projects()
                .map_err(|_| ProjectRuntimeError::Unavailable)?;
            Some(validate_root(
                default_root,
                Path::new(candidate),
                &projects,
            )?)
        }
        None => None,
    };
    let root = root
        .as_deref()
        .map(|root| root.to_str().ok_or(ProjectRuntimeError::RootUnavailable))
        .transpose()?
        .map(str::to_owned);
    store
        .set_new_project_root(root.as_deref())
        .map_err(|_| ProjectRuntimeError::Unavailable)
}

fn prepare_directory(candidate: &Path) -> Result<PathBuf, ProjectRuntimeError> {
    if !candidate.exists() {
        let parent = candidate
            .parent()
            .filter(|parent| parent.is_dir())
            .ok_or(ProjectRuntimeError::RootCannotBeCreated)?;
        fs::create_dir(candidate).map_err(|_| ProjectRuntimeError::RootCannotBeCreated)?;
        if !parent.is_dir() {
            return Err(ProjectRuntimeError::RootCannotBeCreated);
        }
    }
    let metadata =
        fs::symlink_metadata(candidate).map_err(|_| ProjectRuntimeError::RootUnavailable)?;
    if metadata.file_type().is_symlink() {
        return Err(ProjectRuntimeError::RootIsSymlink);
    }
    if !metadata.is_dir() {
        return Err(ProjectRuntimeError::RootNotDirectory);
    }
    fs::canonicalize(candidate).map_err(|_| ProjectRuntimeError::RootUnavailable)
}

fn check_writable(root: &Path) -> Result<(), ProjectRuntimeError> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map_or(0, |duration| duration.as_nanos());
    let path = root.join(format!(".tether-write-test-{}-{stamp}", std::process::id()));
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(_) => fs::remove_file(path).map_err(|_| ProjectRuntimeError::RootNotWritable),
        Err(_) => Err(ProjectRuntimeError::RootNotWritable),
    }
}

fn normalized(path: &Path) -> Option<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    Some(normalized)
}

fn comparison_path(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let name = path.file_name()?;
    fs::canonicalize(parent)
        .ok()
        .map(|parent| parent.join(name))
        .or_else(|| normalized(path))
}
