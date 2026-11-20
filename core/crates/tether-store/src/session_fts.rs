//! Exact phrase search over saved transcript text parts.

use rusqlite::params;
use tether_types::{MessageId, SessionId};

use crate::{Store, StoreError};

/// One matching saved text part and its owning message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSearchHit {
    /// Matched message.
    pub message_id: MessageId,
    /// Owning session.
    pub session_id: SessionId,
    /// Display title of the session.
    pub session_title: String,
    /// Text from the matched part.
    pub text: String,
}

impl Store {
    /// Finds a literal phrase in active transcript text. No model call is made.
    ///
    /// # Errors
    /// Returns SQLite failures.
    pub fn session_search(
        &self,
        phrase: &str,
        limit: usize,
    ) -> Result<Vec<SessionSearchHit>, StoreError> {
        let phrase = phrase.trim();
        if phrase.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let literal = format!("\"{}\"", phrase.replace('"', "\"\""));
        let mut statement = self.conn.prepare(
            "SELECT m.id, m.session_id, s.title, message_fts.body \
             FROM message_fts JOIN messages m ON m.id = message_fts.message_id \
             JOIN sessions s ON s.id = m.session_id \
             WHERE message_fts MATCH ?1 AND m.rewound = 0 \
             ORDER BY bm25(message_fts) LIMIT ?2",
        )?;
        let limit = i64::try_from(limit).unwrap_or(i64::MAX).min(100);
        let rows = statement.query_map(params![literal, limit], |row| {
            Ok(SessionSearchHit {
                message_id: MessageId::new(row.get::<_, String>(0)?),
                session_id: SessionId::new(row.get::<_, String>(1)?),
                session_title: row.get(2)?,
                text: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }
}

#[cfg(test)]
mod tests;
