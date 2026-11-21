//! Saved session settings, model catalog, usage, and search for the platform.

use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use tether_store::StoreError;
use tether_types::{ProjectId, SessionId};

use super::{SessionRuntime, SessionRuntimeError};
use crate::{
    ModelView, PermissionModeView, ProviderView, SessionRecordView, SessionUsageView,
    TranscriptHitView,
};

fn settings_error(error: &StoreError) -> SessionRuntimeError {
    match error {
        StoreError::MissingSessionRow => SessionRuntimeError::NotFound,
        StoreError::SessionBusy => SessionRuntimeError::Busy,
        StoreError::EmptySessionModel | StoreError::EmptySessionTitle => {
            SessionRuntimeError::EmptyInput
        }
        StoreError::BypassNeedsConfirmation => SessionRuntimeError::BypassNeedsConfirmation,
        _ => SessionRuntimeError::Unavailable,
    }
}

#[uniffi::export]
impl SessionRuntime {
    /// Lists models from the saved provider catalog in provider order.
    ///
    /// # Errors
    /// Returns an error when the catalog cannot be read.
    pub fn list_models(
        &self,
        provider: ProviderView,
    ) -> Result<Vec<ModelView>, SessionRuntimeError> {
        self.store()?
            .models(provider.into())
            .map(|models| models.into_iter().map(Into::into).collect())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Sets the provider and model for future turns.
    ///
    /// # Errors
    /// Returns an error for a blank model, absent session, busy session, or store failure.
    pub fn set_session_model(
        &self,
        session_id: &str,
        provider: ProviderView,
        model_id: &str,
    ) -> Result<(), SessionRuntimeError> {
        self.store()?
            .set_session_model(
                &SessionId::new(session_id),
                provider.into(),
                model_id,
                now_ms(),
            )
            .map_err(|error| settings_error(&error))?;
        self.forget_runner(session_id)
    }

    /// Sets the saved permission mode. The app checks biometrics before Bypass.
    ///
    /// # Errors
    /// Returns an error for an absent or busy session, or store failure.
    pub fn set_session_mode(
        &self,
        session_id: &str,
        mode: PermissionModeView,
    ) -> Result<(), SessionRuntimeError> {
        self.store()?
            .set_session_mode(&SessionId::new(session_id), mode.into(), now_ms())
            .map_err(|error| settings_error(&error))?;
        self.forget_runner(session_id)
    }

    /// Renames a session with a trimmed, nonblank title.
    ///
    /// # Errors
    /// Returns an error for a blank title, absent session, or store failure.
    pub fn rename_session(&self, session_id: &str, title: &str) -> Result<(), SessionRuntimeError> {
        self.store()?
            .rename_session(&SessionId::new(session_id), title)
            .map_err(|error| settings_error(&error))
    }

    /// Returns accumulated input, output, and cost for a session.
    ///
    /// # Errors
    /// Returns an error for an absent session or invalid stored usage.
    pub fn session_usage(&self, session_id: &str) -> Result<SessionUsageView, SessionRuntimeError> {
        let store = self.store()?;
        let id = SessionId::new(session_id);
        let usage = store
            .session_usage(&id)
            .map_err(|error| settings_error(&error))?;
        let price_known = store
            .session_price_known(&id)
            .map_err(|error| settings_error(&error))?;
        Ok(SessionUsageView {
            tokens_in: usage.tokens_in,
            tokens_out: usage.tokens_out,
            cost_micros: usage.cost_micros,
            price_known,
        })
    }

    /// Finds sessions in one project by title or live message text.
    ///
    /// # Errors
    /// Returns an error for an absent project or unreadable store.
    pub fn search_sessions(
        &self,
        project_id: &str,
        query: &str,
    ) -> Result<Vec<SessionRecordView>, SessionRuntimeError> {
        let store = self.store()?;
        let id = ProjectId::new(project_id);
        if store
            .load_project(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .is_none()
        {
            return Err(SessionRuntimeError::NotFound);
        }
        store
            .search_sessions(&id, query)
            .map(|sessions| sessions.into_iter().map(Into::into).collect())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Finds an exact phrase in live transcript text, at most 50 hits.
    ///
    /// # Errors
    /// Returns an error when the transcript index cannot be read.
    pub fn search_transcripts(
        &self,
        phrase: &str,
        limit: u32,
    ) -> Result<Vec<TranscriptHitView>, SessionRuntimeError> {
        self.store()?
            .session_search(phrase, limit.min(50) as usize)
            .map(|hits| hits.into_iter().map(Into::into).collect())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }
}

impl SessionRuntime {
    fn forget_runner(&self, session_id: &str) -> Result<(), SessionRuntimeError> {
        if let Some(runner) = self
            .runners
            .lock()
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .remove(&SessionId::new(session_id))
        {
            runner.valid.store(false, Ordering::Release);
        }
        Ok(())
    }
}

pub(super) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

pub(super) fn map_rewind_error(error: &tether_engine::SessionActorError) -> SessionRuntimeError {
    use tether_engine::{RewindError, SessionActorError};
    match error {
        SessionActorError::Rewind(RewindError::Store(
            tether_store::StoreError::MissingCheckpoint,
        )) => SessionRuntimeError::NotFound,
        SessionActorError::Rewind(RewindError::CorruptSnapshot) => {
            SessionRuntimeError::CorruptSnapshot
        }
        SessionActorError::Rewind(RewindError::InvalidPath) => SessionRuntimeError::InvalidPath,
        _ => SessionRuntimeError::Unavailable,
    }
}
