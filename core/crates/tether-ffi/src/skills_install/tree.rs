//! Walks an extracted skill tree: finds the skill folder and reads its files.

use std::fs;
use std::path::{Path, PathBuf};

use crate::skills::SkillsRuntimeError;

/// Finds the single folder that holds a `SKILL.md` file.
pub(super) fn find_skill_dir(root: &Path) -> Result<PathBuf, SkillsRuntimeError> {
    let mut found = Vec::new();
    collect_skill_dirs(root, &mut found)?;
    found.sort();
    found.dedup();
    match found.as_slice() {
        [] => Err(SkillsRuntimeError::NotASkill),
        [one] => Ok(one.clone()),
        many => Err(SkillsRuntimeError::SeveralSkills {
            names: many
                .iter()
                .filter_map(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map(str::to_owned)
                })
                .collect(),
        }),
    }
}

fn collect_skill_dirs(root: &Path, found: &mut Vec<PathBuf>) -> Result<(), SkillsRuntimeError> {
    for entry in fs::read_dir(root)
        .map_err(|_| SkillsRuntimeError::NotASkill)?
        .flatten()
    {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|_| SkillsRuntimeError::NotASkill)?;
        if metadata.file_type().is_symlink() {
            return Err(SkillsRuntimeError::NotASkill);
        }
        if metadata.is_dir() {
            collect_skill_dirs(&path, found)?;
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
            found.extend(path.parent().map(Path::to_path_buf));
        }
    }
    Ok(())
}

/// Reads every file under a skill folder, sorted by relative path.
pub(super) fn read_files(root: &Path) -> Result<Vec<(String, Vec<u8>)>, SkillsRuntimeError> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), SkillsRuntimeError> {
    for entry in fs::read_dir(current)
        .map_err(|_| SkillsRuntimeError::Unavailable)?
        .flatten()
    {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|_| SkillsRuntimeError::Unavailable)?;
        if metadata.file_type().is_symlink() {
            return Err(SkillsRuntimeError::Blocked);
        }
        if metadata.is_dir() {
            collect_files(root, &path, files)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| SkillsRuntimeError::Unavailable)?
            .to_string_lossy()
            .replace('\\', "/");
        files.push((
            relative,
            fs::read(path).map_err(|_| SkillsRuntimeError::Unavailable)?,
        ));
    }
    Ok(())
}
