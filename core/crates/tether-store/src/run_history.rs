//! Derived history for model `run` tool calls.

use rusqlite::params;
use tether_types::{ProjectId, SessionId, ToolCallId};

use crate::{Store, StoreError};

/// One saved `run` tool call and its optional result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCallRow {
    /// Session that owns the call.
    pub session_id: SessionId,
    /// Saved display title of the session.
    pub session_title: String,
    /// Provider call identifier.
    pub call_id: ToolCallId,
    /// Original JSON arguments from the tool call.
    pub args_json: String,
    /// Saved result text, if the call has finished or was cancelled.
    pub result_text: Option<String>,
    /// Whether the saved result is marked as an error.
    pub result_is_error: Option<bool>,
    /// Creation time of the call message in Unix milliseconds.
    pub called_at_ms: i64,
}

impl Store {
    /// Lists live `run` calls for a project, newest call message first.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the derived history.
    pub fn run_calls(
        &self,
        project: &ProjectId,
        limit: u32,
    ) -> Result<Vec<RunCallRow>, StoreError> {
        let limit = i64::from(limit.clamp(1, 200));
        let sql = "SELECT call_session.id, call_session.title,
                          json_extract(call_part.body_json, '$.id'),
                          json_extract(call_part.body_json, '$.args_json'),
                          json_extract(result_part.body_json, '$.text'),
                          json_extract(result_part.body_json, '$.is_error'),
                          call_message.created_at
                   FROM message_parts call_part
                   JOIN messages call_message ON call_message.id = call_part.message_id
                   JOIN sessions call_session ON call_session.id = call_message.session_id
                   LEFT JOIN message_parts result_part
                     ON result_part.kind = 'tool_result'
                    AND json_extract(result_part.body_json, '$.tool_call_id') =
                        json_extract(call_part.body_json, '$.id')
                    AND EXISTS (
                        SELECT 1
                        FROM messages result_message
                        WHERE result_message.id = result_part.message_id
                          AND result_message.session_id = call_message.session_id
                    )
                   WHERE call_part.kind = 'tool_call'
                     AND json_extract(call_part.body_json, '$.tool') = 'run'
                     AND call_message.role = 'assistant'
                     AND call_message.rewound = 0
                     AND call_session.project_id = ?1
                     AND call_session.archived = 0
                   ORDER BY call_message.created_at DESC, call_message.id DESC
                   LIMIT ?2";
        let mut statement = self.conn.prepare(sql)?;
        let rows = statement.query_map(params![project.as_str(), limit], |row| {
            Ok(RunCallRow {
                session_id: SessionId::new(row.get::<_, String>(0)?),
                session_title: row.get(1)?,
                call_id: ToolCallId::new(row.get::<_, String>(2)?),
                args_json: row.get(3)?,
                result_text: row.get(4)?,
                result_is_error: row.get(5)?,
                called_at_ms: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

#[cfg(test)]
#[path = "run_history_tests.rs"]
mod tests;
