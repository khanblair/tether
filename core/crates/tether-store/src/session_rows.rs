//! Small helpers shared by the session write paths: enum parsing and row inserts.

use rusqlite::{Transaction, params};
use tether_types::{MessageId, MessagePart, Role, SessionId};
use ulid::Ulid;

use crate::StoreError;

pub(crate) fn parse_enum<T: std::str::FromStr>(
    field: &'static str,
    value: String,
) -> Result<T, StoreError> {
    value
        .parse()
        .map_err(|_| StoreError::InvalidSessionEnum { field, value })
}

pub(crate) fn insert_message(
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

pub(crate) fn insert_part(
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
