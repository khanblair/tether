//! Archiving, restoring, deleting, and listing archived sessions.

use rusqlite::params;
use tether_types::{ProjectId, SessionId};

use crate::{SessionRecord, Store, StoreError};

impl Store {
    /// Archives or restores a session. Archiving does not change its activity time, so
    /// a restored session returns to its old place in the list.
    ///
    /// # Errors
    /// Returns an error for a missing or busy session, or SQLite failure.
    pub fn set_session_archived(&self, id: &SessionId, archived: bool) -> Result<(), StoreError> {
        let changed = self.conn.execute(
            "UPDATE sessions SET archived = ?2 WHERE id = ?1 AND status NOT IN ('requesting', 'streaming', 'checking_tool', 'running_tool', 'awaiting_approval')",
            params![id.as_str(), i64::from(archived)],
        )?;
        self.check_settings_update(id, changed)
    }

    /// Deletes a session and, by cascade, its messages, parts, checkpoints, todos, and
    /// search rows.
    ///
    /// # Errors
    /// Returns an error for a missing or busy session, or SQLite failure.
    pub fn delete_session(&self, id: &SessionId) -> Result<(), StoreError> {
        let changed = self.conn.execute(
            "DELETE FROM sessions WHERE id = ?1 AND status NOT IN ('requesting', 'streaming', 'checking_tool', 'running_tool', 'awaiting_approval')",
            [id.as_str()],
        )?;
        self.check_settings_update(id, changed)
    }

    /// Lists a project's archived sessions, newest activity first.
    ///
    /// # Errors
    /// Returns an error if SQLite or a saved session row is invalid.
    pub fn list_archived_sessions(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<SessionRecord>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT id FROM sessions WHERE project_id = ?1 AND archived = 1 ORDER BY updated_at DESC, id DESC",
        )?;
        let ids = statement.query_map([project_id.as_str()], |row| row.get::<_, String>(0))?;
        ids.map(|id| {
            let id = SessionId::new(id?);
            self.load_session(&id)?.ok_or(StoreError::MissingSessionRow)
        })
        .collect()
    }
}
