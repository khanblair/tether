//! Filesystem checks for skill discovery.

use std::fs;
use std::path::{Component, Path};

use super::{ALLOWED_FOLDERS, MAX_REFERENCE_BYTES, MAX_SKILL_BYTES, SkillIssue};
use crate::{SkillDocument, SkillError, parse_skill};

pub(super) fn read_skill_file(path: &Path, folder: &str) -> Result<SkillDocument, SkillError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| map_io(&error))?;
    if metadata.file_type().is_symlink() {
        return Err(SkillError::Symlink);
    }
    if metadata.len() > MAX_SKILL_BYTES {
        return Err(SkillError::FileTooLarge);
    }
    parse_skill(
        folder,
        &fs::read_to_string(path).map_err(|error| map_io(&error))?,
    )
}

pub(super) fn validate_references(
    root: &Path,
    issues: &mut Vec<SkillIssue>,
) -> Result<(), SkillError> {
    for folder in ALLOWED_FOLDERS {
        let path = root.join(folder);
        if path.exists() && !is_symlink(&path) {
            validate_reference_tree(&path, issues)?;
        }
    }
    Ok(())
}

fn validate_reference_tree(root: &Path, issues: &mut Vec<SkillIssue>) -> Result<(), SkillError> {
    for entry in fs::read_dir(root)
        .map_err(|error| map_io(&error))?
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if is_symlink(&path) {
            issues.push(SkillIssue::Symlink { path });
            continue;
        }
        if path.is_dir() {
            validate_reference_tree(&path, issues)?;
            continue;
        }
        if fs::metadata(&path).map_err(|error| map_io(&error))?.len() > MAX_REFERENCE_BYTES {
            issues.push(SkillIssue::Invalid {
                path,
                error: "reference file is too large".into(),
            });
        }
    }
    Ok(())
}

pub(super) fn reference_files(root: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut files = Vec::new();
    for folder in ALLOWED_FOLDERS {
        let path = root.join(folder);
        if path.exists() && !is_symlink(&path) {
            collect_files(root, &path, &mut files)?;
        }
    }
    files.sort();
    Ok(files)
}

fn collect_files(
    root: &Path,
    folder: &Path,
    files: &mut Vec<String>,
) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(folder)?.filter_map(Result::ok) {
        let path = entry.path();
        if is_symlink(&path) {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, files)?;
            continue;
        }
        if path.is_file() {
            files.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}

pub(super) fn checked_relative(
    root: &Path,
    relative: &str,
) -> Result<std::path::PathBuf, SkillError> {
    let path = Path::new(relative);
    let mut components = path.components();
    let Some(Component::Normal(folder)) = components.next() else {
        return Err(SkillError::InvalidPath);
    };
    if !ALLOWED_FOLDERS.contains(&folder.to_string_lossy().as_ref())
        || components.any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(SkillError::InvalidPath);
    }
    let mut current = root.to_owned();
    for component in path.components() {
        let Component::Normal(part) = component else {
            return Err(SkillError::InvalidPath);
        };
        current.push(part);
        if is_symlink(&current) {
            return Err(SkillError::Symlink);
        }
    }
    Ok(current)
}

pub(super) fn count_skill_dirs(root: &Path) -> usize {
    fs::read_dir(root).map_or(0, |entries| {
        entries
            .filter_map(Result::ok)
            .filter(|entry| {
                !entry.file_name().to_string_lossy().starts_with('.') && is_directory(&entry.path())
            })
            .count()
    })
}
pub(super) fn cut_chars(source: &str, limit: usize) -> String {
    if source.len() <= limit {
        return source.to_owned();
    }
    let marker = "\n[truncated]";
    let mut end = limit.saturating_sub(marker.len());
    while end > 0 && !source.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{marker}", &source[..end])
}
pub(super) fn is_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir())
}
pub(super) fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
}
pub(super) fn map_io(error: &std::io::Error) -> SkillError {
    if error.kind() == std::io::ErrorKind::NotFound {
        SkillError::NotFound
    } else {
        SkillError::Io(error.to_string())
    }
}
