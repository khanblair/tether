//! Serialized actor handling for checkpoint restore.

use tether_types::MessageId;

use crate::{RewindError, rewind_checkpoint, rewind_conversation_only};

use super::{CoreEvent, RewindScope, SessionActor, SessionActorError, SessionStatus, now_ms};

impl SessionActor {
    pub(super) fn rewind(
        &mut self,
        checkpoint_id: &str,
        scope: RewindScope,
    ) -> Result<(), SessionActorError> {
        if let Some(tools) = self.tools.as_ref() {
            rewind_checkpoint(
                &mut self.store,
                &self.session.id,
                checkpoint_id,
                scope,
                &tools.guard,
                &tools.blobs,
            )?;
        } else if scope == RewindScope::Conversation {
            rewind_conversation_only(&mut self.store, &self.session.id, checkpoint_id)?;
        } else {
            return Err(SessionActorError::NoWorkspace);
        }
        self.finish_rewind()
    }

    /// Hides a message and every later one. Needs no workspace runtime.
    pub(super) fn rewind_to_message(&mut self, message_id: &str) -> Result<(), SessionActorError> {
        let anchor = MessageId::new(message_id);
        self.store
            .rewind_conversation(&self.session.id, &anchor)
            .map_err(|error| SessionActorError::Rewind(RewindError::Store(error)))?;
        self.finish_rewind()
    }

    /// Sets the session idle and tells the caller the transcript changed.
    fn finish_rewind(&mut self) -> Result<(), SessionActorError> {
        self.store
            .set_session_status(&self.session.id, SessionStatus::Idle, now_ms())?;
        self.emit_status(SessionStatus::Idle);
        self.sink.on_event(CoreEvent::SessionRewound {
            session_id: self.session.id.clone(),
        });
        Ok(())
    }
}
