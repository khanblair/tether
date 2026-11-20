//! Serialized actor handling for checkpoint restore.

use crate::{rewind_checkpoint, rewind_conversation_only};

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
        self.store
            .set_session_status(&self.session.id, SessionStatus::Idle, now_ms())?;
        self.emit_status(SessionStatus::Idle);
        self.sink.on_event(CoreEvent::SessionRewound {
            session_id: self.session.id.clone(),
        });
        Ok(())
    }
}
