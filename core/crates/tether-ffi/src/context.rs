//! Context breakdown and compaction controls.

use tether_engine::context;
use tether_types::SessionId;

use crate::{ContextBreakdownView, SessionRuntime, SessionRuntimeError};

#[uniffi::export]
impl SessionRuntime {
    /// Reads the current context breakdown for a session.
    ///
    /// # Errors
    /// Returns `NotFound` when the session is absent.
    pub fn context_breakdown(
        &self,
        session_id: &str,
    ) -> Result<ContextBreakdownView, SessionRuntimeError> {
        let store = self.store()?;
        let id = SessionId::new(session_id);
        let session = store
            .load_session(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .ok_or(SessionRuntimeError::NotFound)?;
        let messages = store
            .context_messages(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        let window = store
            .models(session.provider)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .into_iter()
            .find(|model| model.id == session.model_id)
            .and_then(|model| model.context_window);
        let prompt =
            tether_engine::prompt::PromptBuilder::build(&tether_engine::prompt::PromptInputs {
                project_name: "",
                executors: &[],
                branch: None,
                mode: session.mode,
                tool_specs: &[],
                global_instructions: "",
                project_instructions: "",
                memory_block: "",
                vault_block: "",
                skills_index: None,
                today: "",
            });
        let usage = store
            .session_usage(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        let reported =
            (usage.tokens_in > 0).then(|| u32::try_from(usage.tokens_in).unwrap_or(u32::MAX));
        Ok(context::breakdown(window, &prompt.sections, &[], &messages, reported).into())
    }

    /// Lists context compaction markers for a session.
    ///
    /// # Errors
    /// Returns `NotFound` when the session is absent.
    pub fn compactions(
        &self,
        session_id: &str,
    ) -> Result<Vec<crate::CompactionView>, SessionRuntimeError> {
        let store = self.store()?;
        let id = SessionId::new(session_id);
        if store
            .load_session(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .is_none()
        {
            return Err(SessionRuntimeError::NotFound);
        }
        store
            .compactions(&id)
            .map(|items| items.into_iter().map(Into::into).collect())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Sets the automatic compaction threshold.
    ///
    /// # Errors
    /// Returns `Unavailable` when storage rejects the setting.
    pub fn set_compaction_threshold(&self, percent: u8) -> Result<(), SessionRuntimeError> {
        self.store()?
            .set_compaction_threshold(percent)
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Reads the automatic compaction threshold.
    ///
    /// # Errors
    /// Returns `Unavailable` when storage cannot read the setting.
    pub fn compaction_threshold(&self) -> Result<u8, SessionRuntimeError> {
        self.store()?
            .compaction_threshold()
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Compacts a session transcript on demand.
    ///
    /// # Errors
    /// Returns `NotFound` when the session is not open or `Unavailable` on failure.
    pub fn compact(
        &self,
        session_id: &str,
        focus: Option<String>,
    ) -> Result<(), SessionRuntimeError> {
        let runner = self.open_session(session_id)?;
        self.runtime
            .block_on(runner.handle.compact(focus))
            .map_err(|_| SessionRuntimeError::Unavailable)
    }
}
