//! Settings and small records used by skills, memory, and vault tools.

use rusqlite::{OptionalExtension, params};
use tether_types::{SessionId, ToolCallId};

use crate::{Store, StoreError};

const SKILL_WRITE_APPROVAL: &str = "skill_write_approval";
const MEMORY_WRITE_APPROVAL: &str = "memory_write_approval";
const VAULT_WRITE_APPROVAL: &str = "vault_write_approval";

/// One staged skill change shown before an approved write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillChange {
    /// Stable staged change id.
    pub id: String,
    /// Owning session.
    pub session_id: SessionId,
    /// Provider tool call id.
    pub tool_call_id: ToolCallId,
    /// Create, patch, or delete.
    pub action: String,
    /// Skill name.
    pub name: String,
    /// Human-readable diff.
    pub diff: String,
    /// Creation time in Unix milliseconds.
    pub created_at: i64,
}

impl Store {
    /// Returns whether skill writes require approval. The default is enabled.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn skill_write_approval(&self) -> Result<bool, StoreError> {
        self.approval_setting(SKILL_WRITE_APPROVAL, true)
    }

    /// Returns whether memory writes require approval. The default is disabled.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn memory_write_approval(&self) -> Result<bool, StoreError> {
        self.approval_setting(MEMORY_WRITE_APPROVAL, false)
    }

    /// Returns whether vault writes require approval. The default is disabled.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn vault_write_approval(&self) -> Result<bool, StoreError> {
        self.approval_setting(VAULT_WRITE_APPROVAL, false)
    }

    /// Saves one of the three write approval switches.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn set_write_approval(&self, key: &str, enabled: bool) -> Result<(), StoreError> {
        if !matches!(
            key,
            SKILL_WRITE_APPROVAL | MEMORY_WRITE_APPROVAL | VAULT_WRITE_APPROVAL
        ) {
            return Err(StoreError::InvalidSkillSetting);
        }
        self.conn.execute(
            "INSERT INTO skill_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, if enabled { "true" } else { "false" }],
        )?;
        Ok(())
    }

    /// Enables or disables one skill by name.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn set_skill_enabled(&self, name: &str, enabled: bool) -> Result<(), StoreError> {
        if enabled {
            self.conn
                .execute("DELETE FROM skill_disabled WHERE name = ?1", [name])?;
        } else {
            self.conn.execute(
                "INSERT OR IGNORE INTO skill_disabled (name) VALUES (?1)",
                [name],
            )?;
        }
        Ok(())
    }

    /// Returns whether a named skill is enabled.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn skill_enabled(&self, name: &str) -> Result<bool, StoreError> {
        Ok(!self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM skill_disabled WHERE name = ?1)",
            [name],
            |row| row.get::<_, bool>(0),
        )?)
    }

    /// Lists disabled skill names in deterministic order.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn disabled_skills(&self) -> Result<Vec<String>, StoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT name FROM skill_disabled ORDER BY name")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Stages a skill diff for the approval UI.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn stage_skill_change(&self, change: &SkillChange) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO skill_changes
             (id, session_id, tool_call_id, action, name, diff, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                change.id,
                change.session_id.as_str(),
                change.tool_call_id.as_str(),
                change.action,
                change.name,
                change.diff,
                change.created_at
            ],
        )?;
        Ok(())
    }

    /// Reads a staged skill diff for one tool call.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn skill_change_diff(
        &self,
        session_id: &SessionId,
        tool_call_id: &ToolCallId,
    ) -> Result<Option<String>, StoreError> {
        self.conn
            .query_row(
                "SELECT diff FROM skill_changes WHERE session_id = ?1 AND tool_call_id = ?2",
                params![session_id.as_str(), tool_call_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Removes staged changes for one tool call.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn delete_skill_change(
        &self,
        session_id: &SessionId,
        tool_call_id: &ToolCallId,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM skill_changes WHERE session_id = ?1 AND tool_call_id = ?2",
            params![session_id.as_str(), tool_call_id.as_str()],
        )?;
        Ok(())
    }

    /// Records one user-skill view for the upkeep job.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn record_skill_view(&self, name: &str, viewed_at: i64) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO skill_usage (name, last_viewed_at, views, created_at)
             VALUES (?1, ?2, 1, ?2)
             ON CONFLICT(name) DO UPDATE SET last_viewed_at = excluded.last_viewed_at,
                 views = skill_usage.views + 1",
            params![name, viewed_at],
        )?;
        Ok(())
    }

    fn approval_setting(&self, key: &str, default: bool) -> Result<bool, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM skill_settings WHERE key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map_or(default, |value| value == "true"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tether_types::{ModelRef, PermissionMode, Provider};

    #[test]
    fn approval_settings_and_skill_usage_round_trip() {
        let mut store = Store::open_in_memory().unwrap();
        let project = store.create_project("Skills", 1).unwrap();
        let session = store
            .create_session(
                &project.id,
                &ModelRef {
                    provider: Provider::OpenAi,
                    model_id: "test".into(),
                },
                PermissionMode::Ask,
                1,
            )
            .unwrap();
        assert!(store.skill_write_approval().unwrap());
        assert!(!store.memory_write_approval().unwrap());
        store
            .set_write_approval("memory_write_approval", true)
            .unwrap();
        assert!(store.memory_write_approval().unwrap());
        store.record_skill_view("docs", 7).unwrap();
        store.record_skill_view("docs", 9).unwrap();
        let views: (i64, i64) = store
            .conn
            .query_row(
                "SELECT last_viewed_at, views FROM skill_usage WHERE name = 'docs'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(views, (9, 2));
        assert_eq!(session.project_id, project.id);
    }
}
