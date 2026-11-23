//! Prune-only upkeep for user skills: usage scan and archive moves.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use tether_skills::{SkillScope, SkillState, SkillUsage, UpkeepAction};

use super::scope::valid_name;
use super::views::UpkeepActionView;
use crate::skills::{SkillsRuntime, SkillsRuntimeError};

impl SkillsRuntime {
    pub(super) fn usages(&self) -> Result<Vec<SkillUsage>, SkillsRuntimeError> {
        let entries = fs::read_dir(&self.root).map_err(|_| SkillsRuntimeError::Unavailable)?;
        Ok(entries
            .flatten()
            .filter_map(|entry| usage_for(&entry))
            .collect())
    }

    pub(super) fn apply_action(
        &self,
        action: UpkeepAction,
    ) -> Result<UpkeepActionView, SkillsRuntimeError> {
        if action.to == SkillState::Archived {
            let archive = self.root.join(".archive");
            fs::create_dir_all(&archive).map_err(|_| SkillsRuntimeError::Unavailable)?;
            fs::rename(self.root.join(&action.name), archive.join(&action.name))
                .map_err(|_| SkillsRuntimeError::Unavailable)?;
        }
        Ok(action.into())
    }
}

/// Usage for one user skill folder, or `None` for anything that is not a valid skill folder.
fn usage_for(entry: &fs::DirEntry) -> Option<SkillUsage> {
    let name = entry.file_name().to_string_lossy().into_owned();
    if !valid_name(&name) || !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
        return None;
    }
    let created = entry
        .metadata()
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(epoch_ms)
        .unwrap_or(0);
    Some(SkillUsage {
        name,
        scope: SkillScope::User,
        last_used_at: Some(created),
        created_at: created,
        state: SkillState::Active,
    })
}

fn epoch_ms(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}
