//! Session and transcript writes. A turn's text is committed before its event is emitted.

use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use tether_types::{
    Message, MessageId, MessagePart, ModelRef, PermissionMode, ProjectId, Provider, Role,
    SessionId, SessionStatus, UsageSnapshot,
};
use ulid::Ulid;

use crate::{Store, StoreError};

/// The saved settings and current status of one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    /// Session id.
    pub id: SessionId,
    /// Project id.
    pub project_id: ProjectId,
    /// Display title, taken from the first user message until renamed.
    pub title: String,
    /// Provider for this session.
    pub provider: Provider,
    /// Provider model id.
    pub model_id: String,
    /// Permission mode.
    pub mode: PermissionMode,
    /// Last saved state.
    pub status: SessionStatus,
}

impl Store {
    /// Creates an empty session in an existing project.
    ///
    /// # Errors
    /// Returns an error when the project is absent or SQLite fails.
    pub fn create_session(
        &mut self,
        project_id: &ProjectId,
        model: &ModelRef,
        mode: PermissionMode,
        now_ms: i64,
    ) -> Result<SessionRecord, StoreError> {
        let id = SessionId::new(Ulid::new().to_string());
        self.conn.execute(
            "INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 'idle', ?6, ?6)",
            params![id.as_str(), project_id.as_str(), model.provider.as_str(), model.model_id, mode.as_str(), now_ms],
        )?;
        Ok(SessionRecord {
            id,
            project_id: project_id.clone(),
            title: String::new(),
            provider: model.provider,
            model_id: model.model_id.clone(),
            mode,
            status: SessionStatus::Idle,
        })
    }

    /// Loads one session, or `None` when its id is absent.
    ///
    /// # Errors
    /// Returns an error when SQLite fails or saved enum text is invalid.
    pub fn load_session(&self, id: &SessionId) -> Result<Option<SessionRecord>, StoreError> {
        let row: Option<(String, String, String, String, String, String)> = self.conn
            .query_row(
                "SELECT project_id, title, provider, model_id, mode, status FROM sessions WHERE id = ?1",
                [id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .optional()?;
        let Some((project_id, title, provider, model_id, mode, status)) = row else {
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
        }))
    }

    /// Loads transcript messages and their parts in sequence order.
    ///
    /// # Errors
    /// Returns an error when a saved row or part is invalid.
    pub fn messages(&self, id: &SessionId) -> Result<Vec<Message>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT id, role, seq, created_at, rewound FROM messages WHERE session_id = ?1 ORDER BY seq",
        )?;
        let rows = statement.query_map([id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })?;
        let mut messages = Vec::new();
        for row in rows {
            let (message_id, role, seq, created_at_ms, rewound) = row?;
            let message_id = MessageId::new(message_id);
            messages.push(Message {
                id: message_id.clone(),
                session_id: id.clone(),
                role: parse_enum("role", role)?,
                seq: u32::try_from(seq).map_err(|_| StoreError::SessionCounterOverflow)?,
                created_at_ms,
                rewound,
                parts: self.message_parts(&message_id)?,
            });
        }
        Ok(messages)
    }

    fn message_parts(&self, id: &MessageId) -> Result<Vec<MessagePart>, StoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT body_json FROM message_parts WHERE message_id = ?1 ORDER BY seq")?;
        let rows = statement.query_map([id.as_str()], |row| row.get::<_, String>(0))?;
        rows.map(|row| serde_json::from_str(&row?).map_err(StoreError::MessagePartJson))
            .collect()
    }

    /// Saves the user text and enters Requesting in one transaction.
    ///
    /// # Errors
    /// Returns an error when the session is absent or SQLite fails.
    pub fn begin_turn(
        &mut self,
        id: &SessionId,
        text: &str,
        now_ms: i64,
    ) -> Result<MessageId, StoreError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let message_id = insert_message(&tx, id, Role::User, now_ms)?;
        insert_part(
            &tx,
            &message_id,
            0,
            &MessagePart::Text {
                text: text.to_owned(),
            },
        )?;
        let title: String = text.trim().chars().take(80).collect();
        let changed = tx.execute(
            "UPDATE sessions SET title = CASE WHEN title = '' THEN ?2 ELSE title END, status = 'requesting', updated_at = ?3 WHERE id = ?1",
            params![id.as_str(), title, now_ms],
        )?;
        if changed == 0 {
            return Err(StoreError::MissingSessionRow);
        }
        tx.commit()?;
        Ok(message_id)
    }

    /// Starts an assistant message when the first output arrives.
    ///
    /// # Errors
    /// Returns an error when the session is absent or SQLite fails.
    pub fn create_assistant_message(
        &mut self,
        id: &SessionId,
        now_ms: i64,
    ) -> Result<MessageId, StoreError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let message_id = insert_message(&tx, id, Role::Assistant, now_ms)?;
        tx.commit()?;
        Ok(message_id)
    }

    /// Replaces the in-progress text and thinking parts atomically.
    ///
    /// # Errors
    /// Returns an error when SQLite or JSON encoding fails.
    pub fn save_assistant_parts(
        &mut self,
        id: &MessageId,
        text: &str,
        thinking: &str,
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM message_parts WHERE message_id = ?1",
            [id.as_str()],
        )?;
        let mut seq = 0;
        if !thinking.is_empty() {
            insert_part(
                &tx,
                id,
                seq,
                &MessagePart::Thinking {
                    text: thinking.to_owned(),
                },
            )?;
            seq += 1;
        }
        if !text.is_empty() {
            insert_part(
                &tx,
                id,
                seq,
                &MessagePart::Text {
                    text: text.to_owned(),
                },
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Saves a session state.
    ///
    /// # Errors
    /// Returns an error when the session is absent or SQLite fails.
    pub fn set_session_status(
        &self,
        id: &SessionId,
        status: SessionStatus,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        let changed = self.conn.execute(
            "UPDATE sessions SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![id.as_str(), status.as_str(), now_ms],
        )?;
        if changed == 0 {
            return Err(StoreError::MissingSessionRow);
        }
        Ok(())
    }

    /// Adds final token usage to the session total.
    ///
    /// # Errors
    /// Returns an error when a count overflows or SQLite fails.
    pub fn add_session_usage(
        &self,
        id: &SessionId,
        usage: UsageSnapshot,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        let input =
            i64::try_from(usage.tokens_in).map_err(|_| StoreError::SessionCounterOverflow)?;
        let output =
            i64::try_from(usage.tokens_out).map_err(|_| StoreError::SessionCounterOverflow)?;
        let cost =
            i64::try_from(usage.cost_micros).map_err(|_| StoreError::SessionCounterOverflow)?;
        let changed = self.conn.execute(
            "UPDATE sessions SET tokens_in = tokens_in + ?2, tokens_out = tokens_out + ?3, \
             cost_micros = cost_micros + ?4, updated_at = ?5 WHERE id = ?1",
            params![id.as_str(), input, output, cost, now_ms],
        )?;
        if changed == 0 {
            return Err(StoreError::MissingSessionRow);
        }
        Ok(())
    }

    /// Reads accumulated usage for one session.
    ///
    /// # Errors
    /// Returns an error when the session is absent or a count is out of range.
    pub fn session_usage(&self, id: &SessionId) -> Result<UsageSnapshot, StoreError> {
        let row: Option<(i64, i64, i64)> = self
            .conn
            .query_row(
                "SELECT tokens_in, tokens_out, cost_micros FROM sessions WHERE id = ?1",
                [id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (input, output, cost) = row.ok_or(StoreError::MissingSessionRow)?;
        Ok(UsageSnapshot {
            tokens_in: u64::try_from(input).map_err(|_| StoreError::SessionCounterOverflow)?,
            tokens_out: u64::try_from(output).map_err(|_| StoreError::SessionCounterOverflow)?,
            cost_micros: u64::try_from(cost).map_err(|_| StoreError::SessionCounterOverflow)?,
        })
    }

    /// Marks turns left active by process death as interrupted.
    ///
    /// # Errors
    /// Returns an error when SQLite fails.
    pub fn recover_active_sessions(&self) -> Result<usize, StoreError> {
        Ok(self.conn.execute(
            "UPDATE sessions SET status = 'interrupted' WHERE status NOT IN ('idle', 'interrupted', 'failed')",
            [],
        )?)
    }
}

fn parse_enum<T: std::str::FromStr>(field: &'static str, value: String) -> Result<T, StoreError> {
    value
        .parse()
        .map_err(|_| StoreError::InvalidSessionEnum { field, value })
}

fn insert_message(
    tx: &Transaction<'_>,
    session: &SessionId,
    role: Role,
    now_ms: i64,
) -> Result<MessageId, StoreError> {
    let seq: i64 = tx.query_row(
        "SELECT COALESCE(MAX(seq), -1) + 1 FROM messages WHERE session_id = ?1",
        [session.as_str()],
        |row| row.get(0),
    )?;
    let id = MessageId::new(Ulid::new().to_string());
    tx.execute(
        "INSERT INTO messages (id, session_id, role, seq, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id.as_str(), session.as_str(), role.as_str(), seq, now_ms],
    )?;
    Ok(id)
}

fn insert_part(
    tx: &Transaction<'_>,
    message: &MessageId,
    seq: i64,
    part: &MessagePart,
) -> Result<(), StoreError> {
    let body = serde_json::to_string(part).map_err(StoreError::MessagePartJson)?;
    tx.execute(
        "INSERT INTO message_parts (id, message_id, seq, kind, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![Ulid::new().to_string(), message.as_str(), seq, part.kind(), body],
    )?;
    Ok(())
}
