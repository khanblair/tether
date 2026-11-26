//! Rewinding a session to one of its messages, for editing and regenerating a reply.

use std::sync::atomic::Ordering;

use super::{SessionRunner, SessionRuntimeError};

#[uniffi::export]
impl SessionRunner {
    /// Stops a running turn and hides the given message and every later one.
    /// Files are not restored. Call from a background thread.
    ///
    /// # Errors
    /// Returns an error when the runner is stale, the message is not in this session,
    /// or the store cannot be updated.
    pub fn rewind_to_message(&self, message_id: String) -> Result<(), SessionRuntimeError> {
        if !self.valid.load(Ordering::Acquire) {
            return Err(SessionRuntimeError::Unavailable);
        }
        self.runtime
            .block_on(self.handle.rewind_to_message(message_id))
            .map_err(|error| super::ux::map_rewind_error(&error))
    }
}
