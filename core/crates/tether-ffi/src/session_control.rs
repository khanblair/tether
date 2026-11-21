//! Checklist, plan, and plan execution calls.

use tether_engine::SessionActorError;
use tether_store::Store;
use tether_types::{PermissionMode, SessionId};

use crate::{PermissionModeView, PlanView, SessionRuntime, SessionRuntimeError, TodoItemView};

#[uniffi::export]
impl SessionRuntime {
    /// Reads the current checklist for a session.
    ///
    /// # Errors
    /// Returns `NotFound` when the session does not exist or `Unavailable` on a store error.
    pub fn todos(&self, session_id: &str) -> Result<Vec<TodoItemView>, SessionRuntimeError> {
        let store = self.store()?;
        let id = SessionId::new(session_id);
        require_session(&store, &id)?;
        store
            .todos(&id)
            .map(|items| items.into_iter().map(Into::into).collect())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Reads the latest plan that has not been followed by a user message.
    ///
    /// # Errors
    /// Returns `NotFound` when the session does not exist or `Unavailable` on a store error.
    pub fn pending_plan(&self, session_id: &str) -> Result<Option<PlanView>, SessionRuntimeError> {
        let store = self.store()?;
        let id = SessionId::new(session_id);
        require_session(&store, &id)?;
        store
            .pending_plan(&id)
            .map(|plan| plan.map(Into::into))
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Switches to a working mode and sends `Run the plan.` through the actor.
    ///
    /// # Errors
    /// Returns `InvalidWorkingMode` for Plan or Bypass, or an actor error mapping.
    pub fn run_plan(
        &self,
        session_id: &str,
        working_mode: PermissionModeView,
    ) -> Result<(), SessionRuntimeError> {
        let mode: PermissionMode = working_mode.into();
        if matches!(mode, PermissionMode::Plan | PermissionMode::Bypass) {
            return Err(SessionRuntimeError::InvalidWorkingMode);
        }
        let runner = self.runner(session_id)?;
        self.runtime
            .block_on(runner.handle.run_plan(mode))
            .map_err(map_actor_error)
    }
}

fn require_session(store: &Store, id: &SessionId) -> Result<(), SessionRuntimeError> {
    store
        .load_session(id)
        .map_err(|_| SessionRuntimeError::Unavailable)?
        .map(|_| ())
        .ok_or(SessionRuntimeError::NotFound)
}

fn map_actor_error(error: SessionActorError) -> SessionRuntimeError {
    match error {
        SessionActorError::NotFound => SessionRuntimeError::NotFound,
        SessionActorError::Store(tether_store::StoreError::SessionBusy) => {
            SessionRuntimeError::Busy
        }
        _ => SessionRuntimeError::Unavailable,
    }
}
