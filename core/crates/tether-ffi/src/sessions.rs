//! UniFFI entry point for session creation, transcript reads, and plain chat turns.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tether_engine::{ProviderFactory, SessionActor, SessionHandle, ToolRuntime, provider_for};
use tether_secrets::SecretRef;
use tether_store::{BlobStore, SessionRecord, Store};
use tether_types::{CoreError, ModelRef, PermissionMode, ProjectId, Provider, SessionId};
use tether_workspace::PathGuard;

use crate::{
    CheckpointView, EventSink, MessageView, ProviderView, RewindScopeView, SecretVault,
    SessionRecordView,
};

#[path = "sessions/ux.rs"]
mod ux;
use ux::now_ms;

/// A session could not be created, opened, or updated.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SessionRuntimeError {
    /// The requested session does not exist.
    #[error("session not found")]
    NotFound,
    /// The selected provider has no saved key.
    #[error("provider key is not saved")]
    MissingKey,
    /// The model id, message, or title is empty.
    #[error("input must have text")]
    EmptyInput,
    /// The session is in the middle of a turn or approval.
    #[error("session is busy")]
    Busy,
    /// A checkpoint snapshot failed its hash or size check.
    #[error("checkpoint snapshot is corrupt")]
    CorruptSnapshot,
    /// A checkpoint path failed the workspace guard.
    #[error("checkpoint path is invalid")]
    InvalidPath,
    /// The session store or provider is unavailable.
    #[error("session is unavailable")]
    Unavailable,
}

/// Owns the process runtime. Reopening it marks unfinished turns as interrupted.
#[derive(uniffi::Object)]
pub struct SessionRuntime {
    database_path: String,
    vault: Arc<SecretVault>,
    sink: Arc<dyn EventSink>,
    runtime: Arc<tokio::runtime::Runtime>,
    runners: Mutex<HashMap<SessionId, Arc<SessionRunner>>>,
}

#[uniffi::export]
impl SessionRuntime {
    /// Opens the session database and starts the actor runtime.
    ///
    /// # Errors
    /// Returns an error when the database or runtime cannot start.
    #[uniffi::constructor]
    pub fn new(
        database_path: &str,
        vault: Arc<SecretVault>,
        sink: Arc<dyn EventSink>,
    ) -> Result<Arc<Self>, SessionRuntimeError> {
        let store =
            Store::open(Path::new(database_path)).map_err(|_| SessionRuntimeError::Unavailable)?;
        store
            .recover_active_sessions()
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        Ok(Arc::new(Self {
            database_path: database_path.into(),
            vault,
            sink,
            runtime: Arc::new(runtime),
            runners: Mutex::new(HashMap::new()),
        }))
    }

    /// Creates a session in a project already saved by the project store.
    ///
    /// # Errors
    /// Returns an error for an empty model or a missing project.
    pub fn create_session(
        &self,
        project_id: &str,
        provider: ProviderView,
        model_id: &str,
    ) -> Result<SessionRecordView, SessionRuntimeError> {
        if model_id.trim().is_empty() {
            return Err(SessionRuntimeError::EmptyInput);
        }
        let mut store = self.store()?;
        store
            .create_session(
                &ProjectId::new(project_id),
                &ModelRef {
                    provider: provider.into(),
                    model_id: model_id.into(),
                },
                PermissionMode::Ask,
                now_ms(),
            )
            .map(Into::into)
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Reads saved session settings, including a recovered Interrupted status.
    ///
    /// # Errors
    /// Returns an error when the session is absent or unreadable.
    pub fn load_session(&self, session_id: &str) -> Result<SessionRecordView, SessionRuntimeError> {
        self.store()?
            .load_session(&SessionId::new(session_id))
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .map(Into::into)
            .ok_or(SessionRuntimeError::NotFound)
    }

    /// Lists a project's sessions by newest activity.
    ///
    /// # Errors
    /// Returns an error when the project is absent or storage cannot be read.
    pub fn list_sessions(
        &self,
        project_id: String,
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
            .list_sessions(&id)
            .map(|sessions| sessions.into_iter().map(Into::into).collect())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Returns the first model id in the saved provider catalog, if any.
    ///
    /// # Errors
    /// Returns an error when the catalog cannot be read.
    pub fn default_model(
        &self,
        provider: ProviderView,
    ) -> Result<Option<String>, SessionRuntimeError> {
        self.store()?
            .models(provider.into())
            .map(|models| models.into_iter().next().map(|model| model.id))
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Reads the saved transcript in message order.
    ///
    /// # Errors
    /// Returns an error when the session is absent or the database is unreadable.
    pub fn transcript(&self, session_id: &str) -> Result<Vec<MessageView>, SessionRuntimeError> {
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
            .messages(&id)
            .map(|messages| messages.into_iter().map(Into::into).collect())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Lists the session's checkpoints from newest to oldest.
    ///
    /// # Errors
    /// Returns an error when the session is absent or the database cannot be read.
    pub fn list_checkpoints(
        &self,
        session_id: &str,
    ) -> Result<Vec<CheckpointView>, SessionRuntimeError> {
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
            .list_checkpoints(&id)
            .map(|checkpoints| checkpoints.into_iter().map(Into::into).collect())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Opens one session actor using its saved provider key.
    ///
    /// # Errors
    /// Returns an error when the session or key is absent, or the provider cannot start.
    pub fn open_session(
        &self,
        session_id: &str,
    ) -> Result<Arc<SessionRunner>, SessionRuntimeError> {
        self.open(session_id, None)
    }

    /// Opens a session with guarded tools rooted in its saved project workspace.
    ///
    /// # Errors
    /// Returns an error when the project folder, session, or provider key is unavailable.
    pub fn open_session_with_workspace(
        &self,
        session_id: &str,
        workspaces_root: &str,
    ) -> Result<Arc<SessionRunner>, SessionRuntimeError> {
        self.open(session_id, Some(Path::new(workspaces_root)))
    }
}

impl SessionRuntime {
    fn open(
        &self,
        session_id: &str,
        workspaces_root: Option<&Path>,
    ) -> Result<Arc<SessionRunner>, SessionRuntimeError> {
        let id = SessionId::new(session_id);
        let mut runners = self
            .runners
            .lock()
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        if let Some(existing) = runners.get(&id) {
            return Ok(Arc::clone(existing));
        }
        let store = self.store()?;
        let session = store
            .load_session(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .ok_or(SessionRuntimeError::NotFound)?;
        provider_key(&self.vault, session.provider)?;
        let provider_factory = session_provider_factory(Arc::clone(&self.vault), session.provider);
        let sink: Arc<dyn tether_engine::EventSink> =
            Arc::new(crate::callbacks::SinkAdapter(Arc::clone(&self.sink)));
        let (actor, handle) = if let Some(root) = workspaces_root {
            let tools = session_tool_runtime(&store, &session, root)?;
            SessionActor::new_with_tool_runtime(store, &id, provider_factory, sink, tools)
        } else {
            SessionActor::new_with_provider_factory(store, &id, provider_factory, sink)
        }
        .map_err(|_| SessionRuntimeError::Unavailable)?;
        self.runtime.spawn(actor.run());
        let runner = Arc::new(SessionRunner {
            handle,
            runtime: Arc::clone(&self.runtime),
            valid: AtomicBool::new(true),
        });
        runners.insert(id, Arc::clone(&runner));
        Ok(runner)
    }
    fn store(&self) -> Result<Store, SessionRuntimeError> {
        Store::open(Path::new(&self.database_path)).map_err(|_| SessionRuntimeError::Unavailable)
    }
}

fn session_tool_runtime(
    store: &Store,
    session: &SessionRecord,
    root: &Path,
) -> Result<ToolRuntime, SessionRuntimeError> {
    let root_guard = PathGuard::new(root).map_err(|_| SessionRuntimeError::Unavailable)?;
    let project_path = store
        .project_path(&session.project_id)
        .map_err(|_| SessionRuntimeError::Unavailable)?
        .ok_or(SessionRuntimeError::Unavailable)?;
    let project_root = root_guard
        .resolve(&project_path)
        .map_err(|_| SessionRuntimeError::Unavailable)?;
    let guard = PathGuard::new(project_root).map_err(|_| SessionRuntimeError::Unavailable)?;
    let blob_root = root
        .parent()
        .ok_or(SessionRuntimeError::Unavailable)?
        .join("blobs");
    let blobs = BlobStore::open(blob_root).map_err(|_| SessionRuntimeError::Unavailable)?;
    Ok(ToolRuntime {
        guard,
        blobs,
        approve: Arc::new(|_, _| false),
    })
}

fn provider_key(
    vault: &SecretVault,
    provider: Provider,
) -> Result<tether_secrets::Secret, SessionRuntimeError> {
    let reference = SecretRef::new(&format!("provider-{}", provider.as_str()))
        .map_err(|_| SessionRuntimeError::Unavailable)?;
    vault
        .store
        .get(&reference)
        .map_err(|_| SessionRuntimeError::Unavailable)?
        .ok_or(SessionRuntimeError::MissingKey)
}

fn session_provider_factory(vault: Arc<SecretVault>, provider: Provider) -> ProviderFactory {
    Arc::new(move || {
        let key = provider_key(&vault, provider).map_err(|error| match error {
            SessionRuntimeError::MissingKey => CoreError::Auth {
                message: "provider key is not saved".into(),
            },
            _ => CoreError::Internal {
                message: "provider key is unavailable".into(),
            },
        })?;
        let key_text = key.expose_text().map_err(|_| CoreError::Internal {
            message: "provider key is unavailable".into(),
        })?;
        provider_for(provider, key_text).map_err(CoreError::from)
    })
}

/// A running actor for one session.
#[derive(uniffi::Object)]
pub struct SessionRunner {
    handle: SessionHandle,
    runtime: Arc<tokio::runtime::Runtime>,
    valid: AtomicBool,
}

#[uniffi::export]
impl SessionRunner {
    /// Queues a plain text turn. Events arrive through the runtime's sink.
    /// Call from a background thread.
    ///
    /// # Errors
    /// Returns an error for empty input or a stopped actor.
    pub fn send_message(&self, text: String) -> Result<(), SessionRuntimeError> {
        if !self.valid.load(Ordering::Acquire) {
            return Err(SessionRuntimeError::Unavailable);
        }
        self.runtime
            .block_on(self.handle.send_message(text))
            .map_err(|error| match error {
                tether_engine::SessionActorError::EmptyMessage => SessionRuntimeError::EmptyInput,
                _ => SessionRuntimeError::Unavailable,
            })
    }

    /// Cancels the active turn. Saved text remains in the transcript.
    pub fn interrupt(&self) {
        self.handle.interrupt();
    }

    /// Stops a running turn and restores a saved checkpoint in actor order.
    /// Call from a background thread.
    ///
    /// # Errors
    /// Returns an error when the checkpoint is absent or restoration fails.
    pub fn rewind(
        &self,
        checkpoint_id: String,
        scope: RewindScopeView,
    ) -> Result<(), SessionRuntimeError> {
        if !self.valid.load(Ordering::Acquire) {
            return Err(SessionRuntimeError::Unavailable);
        }
        self.runtime
            .block_on(self.handle.rewind(checkpoint_id, scope.into()))
            .map_err(|error| map_rewind_error(&error))
    }
}

fn map_rewind_error(error: &tether_engine::SessionActorError) -> SessionRuntimeError {
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

#[cfg(test)]
#[path = "sessions/session_tests.rs"]
mod tests;
