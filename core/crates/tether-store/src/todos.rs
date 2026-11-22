//! Session checklist persistence.

use rusqlite::{TransactionBehavior, params};
use tether_types::{SessionId, TodoItem};

use crate::{Store, StoreError};

impl Store {
    /// Replaces every checklist item for a session in one transaction.
    ///
    /// # Errors
    /// Returns an error when a position overflows or SQLite fails.
    pub fn replace_todos(
        &mut self,
        session_id: &SessionId,
        items: &[TodoItem],
        updated_at: i64,
    ) -> Result<(), StoreError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "DELETE FROM session_todos WHERE session_id = ?1",
            [session_id.as_str()],
        )?;
        for (position, item) in items.iter().enumerate() {
            let position = i64::try_from(position).map_err(|_| StoreError::PositionOverflow)?;
            tx.execute(
                "INSERT INTO session_todos (session_id, position, text, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    session_id.as_str(),
                    position,
                    item.text,
                    item.status.as_str(),
                    updated_at
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Reads a session checklist in its saved order.
    ///
    /// # Errors
    /// Returns an error when SQLite contains an unknown checklist status.
    pub fn todos(&self, session_id: &SessionId) -> Result<Vec<TodoItem>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT text, status FROM session_todos WHERE session_id = ?1 ORDER BY position",
        )?;
        let rows = statement.query_map([session_id.as_str()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.map(|row| {
            let (text, status) = row?;
            let status = status.parse().map_err(|_| StoreError::InvalidSessionEnum {
                field: "todo status",
                value: status,
            })?;
            Ok(TodoItem { text, status })
        })
        .collect()
    }
}
