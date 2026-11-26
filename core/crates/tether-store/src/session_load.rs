//! Loading one saved session row.

use rusqlite::{OptionalExtension, params};
use tether_types::{PermissionMode, ProjectId, Provider, ReasoningEffort, SessionId};

use crate::sessions::{SessionRecord, parse_enum};
use crate::{Store, StoreError};

type SessionRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    i64,
    Option<String>,
);

impl Store {
    /// Changes the model used by a session after its active turn ends.
    ///
    /// # Errors
    /// Returns an error for an empty model, missing session, busy session, or SQLite failure.
    pub fn set_session_model(
        &self,
        id: &SessionId,
        provider: Provider,
        model_id: &str,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        let model_id = model_id.trim();
        if model_id.is_empty() {
            return Err(StoreError::EmptySessionModel);
        }
        let changed = self.conn.execute(
            "UPDATE sessions SET provider = ?2, model_id = ?3, updated_at = MAX(updated_at + 1, ?4) WHERE id = ?1 AND status NOT IN ('requesting', 'streaming', 'checking_tool', 'running_tool', 'awaiting_approval')",
            params![id.as_str(), provider.as_str(), model_id, now_ms],
        )?;
        self.check_settings_update(id, changed)
    }

    /// Changes the saved permission mode after the active turn ends.
    ///
    /// # Errors
    /// Returns an error for a missing or busy session, or SQLite failure.
    pub fn set_session_mode(
        &self,
        id: &SessionId,
        mode: PermissionMode,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        if mode == PermissionMode::Bypass {
            return Err(StoreError::BypassNeedsConfirmation);
        }
        let changed = self.conn.execute(
            "UPDATE sessions SET mode = ?2, updated_at = MAX(updated_at + 1, ?3) WHERE id = ?1 AND status NOT IN ('requesting', 'streaming', 'checking_tool', 'running_tool', 'awaiting_approval')",
            params![id.as_str(), mode.as_str(), now_ms],
        )?;
        self.check_settings_update(id, changed)
    }

    /// Changes the saved reasoning effort after the active turn ends. `None` clears it,
    /// so the model default applies.
    ///
    /// # Errors
    /// Returns an error for a missing or busy session, or SQLite failure.
    pub fn set_session_reasoning(
        &self,
        id: &SessionId,
        effort: Option<ReasoningEffort>,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        let changed = self.conn.execute(
            "UPDATE sessions SET reasoning_effort = ?2, updated_at = MAX(updated_at + 1, ?3) WHERE id = ?1 AND status NOT IN ('requesting', 'streaming', 'checking_tool', 'running_tool', 'awaiting_approval')",
            params![id.as_str(), effort.map(ReasoningEffort::as_str), now_ms],
        )?;
        self.check_settings_update(id, changed)
    }

    fn check_settings_update(&self, id: &SessionId, changed: usize) -> Result<(), StoreError> {
        if changed != 0 {
            return Ok(());
        }
        if self.load_session(id)?.is_some() {
            Err(StoreError::SessionBusy)
        } else {
            Err(StoreError::MissingSessionRow)
        }
    }

    /// Loads one session, or `None` when its id is absent.
    ///
    /// # Errors
    /// Returns an error when SQLite fails or saved enum text is invalid.
    pub fn load_session(&self, id: &SessionId) -> Result<Option<SessionRecord>, StoreError> {
        let row: Option<SessionRow> = self.conn
            .query_row(
                "SELECT project_id, title, provider, model_id, mode, status, created_at, updated_at, \
                 reasoning_effort FROM sessions WHERE id = ?1",
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
                        row.get(8)?,
                    ))
                },
            )
            .optional()?;
        let Some((
            project_id,
            title,
            provider,
            model_id,
            mode,
            status,
            created_at,
            updated_at,
            reasoning_effort,
        )) = row
        else {
            return Ok(None);
        };
        let reasoning_effort = reasoning_effort
            .map(|value| parse_enum("reasoning_effort", value))
            .transpose()?;
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
            reasoning_effort,
        }))
    }
}
