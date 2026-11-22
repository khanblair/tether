//! Persisted context compaction and interrupted-turn operations.

use rusqlite::params;
use tether_types::{Message, MessageId, MessagePart, Role, SessionId};
use ulid::Ulid;

use crate::{Store, StoreError};

/// One saved compaction marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionRecord {
    /// Stable marker id.
    pub id: String,
    /// Session that owns the marker.
    pub session_id: SessionId,
    /// Summary message shown in the transcript.
    pub summary_message_id: MessageId,
    /// First message replaced in provider requests.
    pub first_message_id: MessageId,
    /// Last message replaced in provider requests.
    pub last_message_id: MessageId,
    /// Estimated or reported tokens before the summary.
    pub tokens_before: u32,
    /// Estimated tokens after the summary.
    pub tokens_after: u32,
    /// Creation time in Unix milliseconds.
    pub created_at: i64,
    /// Optional user focus.
    pub focus: Option<String>,
}

impl Store {
    /// Reads the configured automatic compaction threshold.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the setting.
    pub fn compaction_threshold(&self) -> Result<u8, StoreError> {
        self.conn
            .query_row(
                "SELECT compact_at_percent FROM context_settings WHERE id = 1",
                [],
                |row| row.get::<_, u8>(0),
            )
            .map_err(StoreError::from)
    }

    /// Saves the configured automatic compaction threshold.
    ///
    /// # Errors
    /// Returns an error when the value is outside 50 to 95 or SQLite fails.
    pub fn set_compaction_threshold(&self, percent: u8) -> Result<(), StoreError> {
        if !(50..=95).contains(&percent) {
            return Err(StoreError::InvalidContextThreshold);
        }
        self.conn.execute(
            "UPDATE context_settings SET compact_at_percent = ?1 WHERE id = 1",
            [percent],
        )?;
        Ok(())
    }

    /// Lists compaction markers in creation order.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the rows.
    pub fn compactions(&self, session_id: &SessionId) -> Result<Vec<CompactionRecord>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT id, summary_message_id, first_message_id, last_message_id,
                    tokens_before, tokens_after, created_at, focus
             FROM compactions WHERE session_id = ?1 ORDER BY created_at, id",
        )?;
        let rows = statement.query_map([session_id.as_str()], |row| {
            Ok(CompactionRecord {
                id: row.get(0)?,
                session_id: session_id.clone(),
                summary_message_id: MessageId::new(row.get::<_, String>(1)?),
                first_message_id: MessageId::new(row.get::<_, String>(2)?),
                last_message_id: MessageId::new(row.get::<_, String>(3)?),
                tokens_before: read_u32(row.get(4)?)?,
                tokens_after: read_u32(row.get(5)?)?,
                created_at: row.get(6)?,
                focus: row.get(7)?,
            })
        })?;
        rows.map(|row| row.map_err(StoreError::from)).collect()
    }

    /// Saves a compaction marker after its summary message exists.
    ///
    /// # Errors
    /// Returns an error when SQLite fails.
    pub fn insert_compaction(&self, record: &CompactionRecord) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO compactions
             (id, session_id, summary_message_id, first_message_id, last_message_id,
              tokens_before, tokens_after, created_at, focus)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.id,
                record.session_id.as_str(),
                record.summary_message_id.as_str(),
                record.first_message_id.as_str(),
                record.last_message_id.as_str(),
                i64::from(record.tokens_before),
                i64::from(record.tokens_after),
                record.created_at,
                record.focus,
            ],
        )?;
        Ok(())
    }

    /// Returns live messages with compacted ranges replaced by their summaries.
    ///
    /// # Errors
    /// Returns an error when the transcript or compaction rows are invalid.
    pub fn context_messages(&self, session_id: &SessionId) -> Result<Vec<Message>, StoreError> {
        let messages = self
            .messages(session_id)?
            .into_iter()
            .filter(|message| !message.rewound)
            .collect::<Vec<_>>();
        let compactions = self.compactions(session_id)?;
        let positions = messages
            .iter()
            .map(|message| (message.id.clone(), message.seq))
            .collect::<std::collections::HashMap<_, _>>();
        let summaries = messages
            .iter()
            .filter(|message| {
                compactions
                    .iter()
                    .any(|item| item.summary_message_id == message.id)
            })
            .cloned()
            .collect::<Vec<_>>();
        // A summary is placed where its compacted range was, never at its own position.
        let in_place = messages.into_iter().filter(|message| {
            !compactions
                .iter()
                .any(|item| item.summary_message_id == message.id)
        });
        let mut result = Vec::new();
        for message in in_place {
            let seq = message.seq;
            match compactions
                .iter()
                .find(|item| compaction_covers(item, &positions, seq))
            {
                Some(compaction) => push_summary(&mut result, &summaries, compaction),
                None => result.push(message),
            }
        }
        Ok(result)
    }

    /// Adds synthetic results for tool calls left open by process death.
    ///
    /// # Errors
    /// Returns an error when the transcript cannot be read or written.
    pub fn repair_interrupted(&mut self, session_id: &SessionId) -> Result<(), StoreError> {
        let messages = self.messages(session_id)?;
        let parts = || messages.iter().flat_map(|message| &message.parts);
        let calls = parts()
            .filter_map(|part| match part {
                MessagePart::ToolCall { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let results = parts()
            .filter_map(|part| match part {
                MessagePart::ToolResult { tool_call_id, .. } => Some(tool_call_id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for call in calls.into_iter().filter(|call| !results.contains(call)) {
            self.append_message(
                session_id,
                Role::Tool,
                &[MessagePart::ToolResult {
                    tool_call_id: call,
                    is_error: true,
                    text: "This call was interrupted when the app stopped. Its effect is unknown. Check the current state before you retry it.".into(),
                    result_ref: Some("synthetic".into()),
                }],
                0,
            )?;
        }
        Ok(())
    }

    /// Returns the last unresolved operation for an interrupted session.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the session transcript.
    pub fn interruption_step(&self, session_id: &SessionId) -> Result<Option<String>, StoreError> {
        let messages = self.messages(session_id)?;
        let has_assistant = messages
            .iter()
            .any(|message| message.role == Role::Assistant);
        let parts = || messages.iter().rev().flat_map(|message| &message.parts);
        let calls = parts()
            .filter_map(|part| match part {
                MessagePart::ToolCall {
                    id,
                    tool,
                    args_json,
                } => Some((id.clone(), tool.clone(), args_json.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let results = parts()
            .filter_map(|part| match part {
                MessagePart::ToolResult { tool_call_id, .. } => Some(tool_call_id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if let Some((_, tool, args)) = calls.into_iter().find(|(id, _, _)| !results.contains(id)) {
            return Ok(Some(format!("tool: {tool} {args}")));
        }
        Ok(has_assistant.then(|| "waiting for the model".into()))
    }

    /// Removes compaction markers that include or follow a rewind anchor.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot update the markers.
    pub fn remove_compactions_after(
        &self,
        session_id: &SessionId,
        message_id: &MessageId,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM compactions WHERE session_id = ?1 AND EXISTS (
                SELECT 1 FROM messages anchor, messages last
                WHERE anchor.id = ?2 AND last.id = compactions.last_message_id
                  AND last.seq >= anchor.seq)",
            params![session_id.as_str(), message_id.as_str()],
        )?;
        Ok(())
    }
}

fn compaction_covers(
    compaction: &CompactionRecord,
    positions: &std::collections::HashMap<MessageId, u32>,
    seq: u32,
) -> bool {
    let first = positions.get(&compaction.first_message_id).copied();
    let last = positions.get(&compaction.last_message_id).copied();
    first
        .zip(last)
        .is_some_and(|(first, last)| (first..=last).contains(&seq))
}

fn push_summary(result: &mut Vec<Message>, summaries: &[Message], compaction: &CompactionRecord) {
    let Some(summary) = summaries
        .iter()
        .find(|item| item.id == compaction.summary_message_id)
    else {
        return;
    };
    if !result.iter().any(|item| item.id == summary.id) {
        result.push(summary.clone());
    }
}

fn read_u32(value: i64) -> Result<u32, rusqlite::Error> {
    u32::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, value))
}

/// Makes a new compaction marker id.
#[must_use]
pub fn new_compaction_id() -> String {
    Ulid::new().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_has_the_product_default_and_rejects_out_of_range_values() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.compaction_threshold().unwrap(), 80);
        assert!(matches!(
            store.set_compaction_threshold(49),
            Err(StoreError::InvalidContextThreshold)
        ));
        store.set_compaction_threshold(95).unwrap();
        assert_eq!(store.compaction_threshold().unwrap(), 95);
    }
}
