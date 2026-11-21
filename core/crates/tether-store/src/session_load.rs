//! Loading one saved session row.

use rusqlite::OptionalExtension;
use tether_types::{ProjectId, SessionId};

use crate::sessions::{SessionRecord, parse_enum};
use crate::{Store, StoreError};

type SessionRow = (String, String, String, String, String, String, i64, i64);

impl Store {
    /// Loads one session, or `None` when its id is absent.
    ///
    /// # Errors
    /// Returns an error when SQLite fails or saved enum text is invalid.
    pub fn load_session(&self, id: &SessionId) -> Result<Option<SessionRecord>, StoreError> {
        let row: Option<SessionRow> = self.conn
            .query_row(
                "SELECT project_id, title, provider, model_id, mode, status, created_at, updated_at \
                 FROM sessions WHERE id = ?1",
                [id.as_str()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .optional()?;
        let Some((project_id, title, provider, model_id, mode, status, created_at, updated_at)) =
            row
        else {
            return Ok(None);
        };
        Ok(Some(SessionRecord {
            id: id.clone(),
            project_id: ProjectId::new(project_id),
            title,
            provider: parse_enum("provider", provider)?,
            model_id,
            mode: parse_enum("mode", mode)?,
            status: parse_enum("status", status)?,
            created_at,
            updated_at,
        }))
    }
}
