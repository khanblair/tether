//! Session context visibility operations.

use tether_types::SessionId;

use crate::{Store, StoreError};

impl Store {
    /// Hides all current messages from future model requests while keeping them in the transcript.
    ///
    /// # Errors
    /// Returns an error when the session is absent or SQLite fails.
    pub fn clear_context(&self, session_id: &SessionId) -> Result<(), StoreError> {
        let changed = self.conn.execute(
            "UPDATE messages SET rewound = 1 WHERE session_id = ?1",
            [session_id.as_str()],
        )?;
        if changed == 0 && self.load_session(session_id)?.is_none() {
            return Err(StoreError::MissingSessionRow);
        }
        Ok(())
    }
}
