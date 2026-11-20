//! Session list, message search, and rename.

use rusqlite::params;
use tether_types::{ProjectId, SessionId};

use crate::{SessionRecord, Store, StoreError};

impl Store {
    /// Lists a project's sessions, newest activity first.
    ///
    /// # Errors
    /// Returns an error if SQLite or a saved session row is invalid.
    pub fn list_sessions(&self, project_id: &ProjectId) -> Result<Vec<SessionRecord>, StoreError> {
        self.find_sessions(project_id, None)
    }

    /// Finds sessions by title or non-rewound message text, newest activity first.
    ///
    /// # Errors
    /// Returns an error if SQLite or a saved session row is invalid.
    pub fn search_sessions(
        &self,
        project_id: &ProjectId,
        query: &str,
    ) -> Result<Vec<SessionRecord>, StoreError> {
        let query = query.trim();
        if query.is_empty() {
            return self.list_sessions(project_id);
        }
        self.find_sessions(project_id, Some(query))
    }

    fn find_sessions(
        &self,
        project_id: &ProjectId,
        query: Option<&str>,
    ) -> Result<Vec<SessionRecord>, StoreError> {
        let sql = "SELECT s.id FROM sessions s WHERE s.project_id = ?1 AND (
            ?2 IS NULL OR instr(lower(s.title), lower(?2)) > 0 OR EXISTS (
                SELECT 1 FROM messages m JOIN message_parts p ON p.message_id = m.id
                WHERE m.session_id = s.id AND m.rewound = 0 AND p.kind = 'text'
                AND instr(lower(json_extract(p.body_json, '$.text')), lower(?2)) > 0
            )) ORDER BY s.updated_at DESC, s.id DESC";
        let mut statement = self.conn.prepare(sql)?;
        let ids = statement.query_map(params![project_id.as_str(), query], |row| {
            row.get::<_, String>(0)
        })?;
        ids.map(|id| {
            let id = SessionId::new(id?);
            self.load_session(&id)?.ok_or(StoreError::MissingSessionRow)
        })
        .collect()
    }

    /// Replaces the title chosen from the first user message.
    ///
    /// # Errors
    /// Returns an error for a blank title, missing session, or SQLite failure.
    pub fn rename_session(&self, id: &SessionId, title: &str) -> Result<(), StoreError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(StoreError::EmptySessionTitle);
        }
        let changed = self.conn.execute(
            "UPDATE sessions SET title = ?2 WHERE id = ?1",
            params![id.as_str(), title],
        )?;
        if changed == 0 {
            return Err(StoreError::MissingSessionRow);
        }
        Ok(())
    }
}
