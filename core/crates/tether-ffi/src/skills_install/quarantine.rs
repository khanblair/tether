//! Quarantine folders: lookup, expiry, and the record used by approval.

use std::fs;
use std::path::{Path, PathBuf};

use super::QUARANTINE_TTL_MS;
use super::preview::inspect_content;
use super::views::ScanReportView;
use crate::skills::{SkillsRuntime, SkillsRuntimeError};

/// A quarantined skill that approval can move into place.
pub(super) struct InstallRecord {
    pub(super) id_dir: PathBuf,
    pub(super) skill_dir: PathBuf,
    pub(super) name: String,
    pub(super) scan: ScanReportView,
}

impl SkillsRuntime {
    pub(super) fn record(&self, id: &str) -> Result<InstallRecord, SkillsRuntimeError> {
        self.expire_quarantine((self.clock)())?;
        let id_dir = self.quarantine_dir(id)?;
        // A folder that expired, or was discarded, is gone: say so instead of "not a skill".
        if !id_dir.is_dir() {
            return Err(SkillsRuntimeError::Unavailable);
        }
        let inspected = inspect_content(&id_dir.join("content"))?;
        Ok(InstallRecord {
            id_dir,
            skill_dir: inspected.skill_dir,
            name: inspected.name,
            scan: inspected.scan.into(),
        })
    }

    pub(super) fn quarantine_dir(&self, id: &str) -> Result<PathBuf, SkillsRuntimeError> {
        if !valid_id(id) {
            return Err(SkillsRuntimeError::InvalidPath);
        }
        Ok(self.root.join(".quarantine").join(id))
    }

    pub(super) fn expire_quarantine(&self, now_ms: i64) -> Result<(), SkillsRuntimeError> {
        let Ok(entries) = fs::read_dir(self.root.join(".quarantine")) else {
            return Ok(());
        };
        for entry in entries.flatten() {
            expire_entry(&entry.path(), now_ms)?;
        }
        Ok(())
    }
}

/// Removes one quarantine folder when its `created` stamp is past the TTL.
fn expire_entry(path: &Path, now_ms: i64) -> Result<(), SkillsRuntimeError> {
    let Ok(created) = fs::read_to_string(path.join("created")) else {
        return Ok(());
    };
    if created
        .parse::<i64>()
        .is_ok_and(|time| now_ms.saturating_sub(time) >= QUARANTINE_TTL_MS)
    {
        fs::remove_dir_all(path).map_err(|_| SkillsRuntimeError::Unavailable)?;
    }
    Ok(())
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}
