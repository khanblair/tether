//! Stored approval answers.

use rusqlite::params;
use tether_types::{SessionId, ToolCallId};

use crate::{Store, StoreError};

/// One answered or automatically denied approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRecord {
    /// Stable approval row id.
    pub id: String,
    /// Session that owns the approval.
    pub session_id: SessionId,
    /// Provider call identifier.
    pub tool_call_id: ToolCallId,
    /// Tool name.
    pub tool: String,
    /// Guarded display target.
    pub target: String,
    /// Either `allow` or `deny`.
    pub decision: String,
    /// Optional denial explanation.
    pub reason: Option<String>,
    /// Answer time in Unix milliseconds.
    pub decided_at: i64,
    /// Answer source, currently `app`.
    pub via: String,
}

impl Store {
    /// Records one approval answer.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot save the row.
    pub fn record_approval(&self, record: &ApprovalRecord) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO approvals (id, session_id, tool_call_id, tool, target, decision, reason, decided_at, via) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![record.id, record.session_id.as_str(), record.tool_call_id.as_str(), record.tool, record.target, record.decision, record.reason, record.decided_at, record.via],
        )?;
        Ok(())
    }

    /// Lists approval answers for one session in answer order.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the rows.
    pub fn list_approvals(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<ApprovalRecord>, StoreError> {
        let mut statement = self.conn.prepare("SELECT id, tool_call_id, tool, target, decision, reason, decided_at, via FROM approvals WHERE session_id = ?1 ORDER BY decided_at, id")?;
        let rows = statement.query_map([session_id.as_str()], |row| {
            Ok(ApprovalRecord {
                id: row.get(0)?,
                session_id: session_id.clone(),
                tool_call_id: ToolCallId::new(row.get::<_, String>(1)?),
                tool: row.get(2)?,
                target: row.get(3)?,
                decision: row.get(4)?,
                reason: row.get(5)?,
                decided_at: row.get(6)?,
                via: row.get(7)?,
            })
        })?;
        rows.map(|row| row.map_err(StoreError::from)).collect()
    }
}
