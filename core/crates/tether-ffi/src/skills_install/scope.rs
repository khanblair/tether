//! Storage roots for user and trusted project skills, and name validation.

use std::path::{Path, PathBuf};

use tether_store::{Store, project_dir};
use tether_types::ProjectId;

use super::views::SkillScopeView;
use crate::skills::{SkillsRuntime, SkillsRuntimeError};

impl SkillsRuntime {
    pub(super) fn scope_root(&self, scope: &SkillScopeView) -> Result<PathBuf, SkillsRuntimeError> {
        match scope {
            SkillScopeView::User => Ok(self.root.clone()),
            SkillScopeView::Project { project_id } => project_skill_root(&self.root, project_id),
        }
    }
}

fn project_skill_root(root: &Path, project_id: &str) -> Result<PathBuf, SkillsRuntimeError> {
    let app = root.parent().ok_or(SkillsRuntimeError::Unavailable)?;
    let store =
        Store::open(&app.join("db/tether.db")).map_err(|_| SkillsRuntimeError::Unavailable)?;
    let id = ProjectId::new(project_id.to_owned());
    if !store
        .is_project_trusted(&id)
        .map_err(|_| SkillsRuntimeError::Unavailable)?
    {
        return Err(SkillsRuntimeError::NotTrusted);
    }
    let project = store
        .load_project(&id)
        .map_err(|_| SkillsRuntimeError::Unavailable)?
        .ok_or(SkillsRuntimeError::Unavailable)?;
    Ok(project_dir(&app.join("workspaces"), &project).join(".tether/skills"))
}

pub(super) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
