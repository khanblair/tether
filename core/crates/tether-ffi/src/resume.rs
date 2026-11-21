//! Resume and discard controls for sessions recovered after process death.

use tether_types::SessionId;

use crate::{InterruptionView, SessionRuntime, SessionRuntimeError};

#[uniffi::export]
impl SessionRuntime {
    /// Reads the step saved for an interrupted session.
    ///
    /// # Errors
    /// Returns `NotFound` when the session is absent.
    pub fn interruption(
        &self,
        session_id: &str,
    ) -> Result<Option<InterruptionView>, SessionRuntimeError> {
        let store = self.store()?;
        let id = SessionId::new(session_id);
        let session = store
            .load_session(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .ok_or(SessionRuntimeError::NotFound)?;
        let step = store
            .interruption_step(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        Ok(step.map(|step| InterruptionView {
            step,
            at_ms: session.updated_at,
        }))
    }

    /// Repairs and resumes an interrupted session without adding a user message.
    ///
    /// # Errors
    /// Returns `Busy`, `NotFound`, or `Unavailable` when the session cannot resume.
    pub fn resume_session(&self, session_id: &str) -> Result<(), SessionRuntimeError> {
        let runner = self.open_session(session_id)?;
        self.runtime
            .block_on(runner.handle.resume())
            .map_err(map_resume_error)
    }

    /// Repairs an interrupted session and leaves it idle.
    ///
    /// # Errors
    /// Returns `Busy`, `NotFound`, or `Unavailable` when the session cannot be discarded.
    pub fn discard_interrupted(&self, session_id: &str) -> Result<(), SessionRuntimeError> {
        let runner = self.open_session(session_id)?;
        self.runtime
            .block_on(runner.handle.discard_interrupted())
            .map_err(map_resume_error)
    }
}

fn map_resume_error(error: tether_engine::SessionActorError) -> SessionRuntimeError {
    match error {
        tether_engine::SessionActorError::NotInterrupted => SessionRuntimeError::Busy,
        tether_engine::SessionActorError::NotFound => SessionRuntimeError::NotFound,
        _ => SessionRuntimeError::Unavailable,
    }
}
