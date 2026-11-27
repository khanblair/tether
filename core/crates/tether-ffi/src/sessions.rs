//! UniFFI entry point for session creation, transcript reads, and plain chat turns.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::approvals::ApprovalBroker;
use crate::{
    CheckpointView, EventSink, MessageView, ProviderView, RewindScopeView, SecretVault,
    SendOutcomeView, SessionRecordView,
};
use tether_engine::SessionHandle;
use tether_engine::prompt::PromptSources;
use tether_store::Store;
#[cfg(test)]
use tether_types::Provider;
use tether_types::{ModelRef, PermissionMode, ProjectId, SessionId};

#[path = "sessions/provider.rs"]
mod provider;
use provider::{provider_key, session_provider_factory};
#[path = "sessions/open.rs"]
mod open;
#[path = "sessions/prompt.rs"]
mod prompt;
#[path = "sessions/recovery.rs"]
mod recovery;
mod rewind_message;
#[path = "sessions/user_runs.rs"]
mod user_runs;
#[path = "sessions/ux.rs"]
mod ux;
pub use recovery::recover_interrupted_sessions;
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
    /// Bypass is only available after the app confirms it in memory.
    #[error("bypass needs confirmation")]
    BypassNeedsConfirmation,
    /// Global instructions exceed the 32 KiB cap.
    #[error("global instructions exceed their character cap")]
    InstructionsTooLong,
    /// Plan execution needs a working mode other than Plan or Bypass.
    #[error("working mode must not be Plan or Bypass")]
    InvalidWorkingMode,
}

/// Owns the process runtime. Building it does not change saved turn status;
/// call `recover_interrupted_sessions` once at process start instead.
#[derive(uniffi::Object)]
pub struct SessionRuntime {
    database_path: String,
    vault: Arc<SecretVault>,
    sink: Arc<dyn EventSink>,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
    pub(crate) runners: Mutex<HashMap<SessionId, Arc<SessionRunner>>>,
    pub(crate) project_roots: Mutex<HashMap<SessionId, PathBuf>>,
    pub(crate) broker: Arc<ApprovalBroker>,
    pub(crate) files_root: PathBuf,
    pub(crate) network: Arc<tether_engine::network::NetworkControl>,
    pub(crate) notify: Arc<crate::notify::NotifyState>,
    pub(crate) mcp_manager: Arc<tether_engine::McpManager>,
    pub(crate) linux_shell_slot: Arc<crate::shell::ShellSlot>,
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
        let mcp_manager = crate::mcp::manager_from_store(&store, &vault)
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        let files_root = prompt::files_root(Path::new(database_path));
        Ok(Arc::new(Self {
            database_path: database_path.into(),
            vault,
            sink,
            runtime: Arc::new(runtime),
            runners: Mutex::new(HashMap::new()),
            project_roots: Mutex::new(HashMap::new()),
            broker: Arc::new(ApprovalBroker::new()),
            files_root,
            network: Arc::new(tether_engine::network::NetworkControl::new()),
            notify: Arc::new(crate::notify::NotifyState::new()),
            mcp_manager,
            linux_shell_slot: crate::shell::ShellSlot::new(),
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
    pub(crate) fn session_sink(&self) -> Arc<dyn tether_engine::EventSink> {
        Arc::new(crate::notify::SessionSink {
            sink: Arc::clone(&self.sink),
            state: Arc::clone(&self.notify),
        })
    }

    pub(crate) fn store(&self) -> Result<Store, SessionRuntimeError> {
        Store::open(Path::new(&self.database_path)).map_err(|_| SessionRuntimeError::Unavailable)
    }

    fn global_instructions_path(&self) -> PathBuf {
        self.files_root.join("instructions").join("GLOBAL.md")
    }

    fn prompt_sources(
        &self,
        project_name: &str,
        project_root: Option<PathBuf>,
        skills_index: Option<String>,
    ) -> PromptSources {
        PromptSources {
            project_name: project_name.to_owned(),
            project_root,
            memory_root: Some(self.files_root.join("memory")),
            vault_root: Some(self.files_root.join("memory").join("vault")),
            global_instructions_path: Some(self.global_instructions_path()),
            skills_index,
            ..PromptSources::default()
        }
    }
}

/// A running actor for one session.
#[derive(uniffi::Object)]
pub struct SessionRunner {
    pub(crate) handle: SessionHandle,
    runtime: Arc<tokio::runtime::Runtime>,
    valid: AtomicBool,
    pub(crate) background: Option<Arc<tether_engine::BackgroundProcesses>>,
}

impl SessionRunner {
    pub(crate) fn shutdown_background(&self) {
        if let Some(background) = &self.background {
            background.shutdown();
        }
    }
}

impl Drop for SessionRunner {
    fn drop(&mut self) {
        self.shutdown_background();
    }
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

    /// Queues a turn and returns resolved and skipped file mentions.
    ///
    /// # Errors
    /// Returns an error for empty input, a stopped actor, or unavailable storage.
    pub fn send_message_with_outcome(
        &self,
        text: String,
    ) -> Result<SendOutcomeView, SessionRuntimeError> {
        if !self.valid.load(Ordering::Acquire) {
            return Err(SessionRuntimeError::Unavailable);
        }
        self.runtime
            .block_on(self.handle.send_message_with_mentions(text))
            .map(Into::into)
            .map_err(|error| match error {
                tether_engine::SessionActorError::EmptyMessage => SessionRuntimeError::EmptyInput,
                _ => SessionRuntimeError::Unavailable,
            })
    }

    /// Clears future model context while preserving the transcript.
    ///
    /// # Errors
    /// Returns an error when the actor is stopped or storage is unavailable.
    pub fn clear_context(&self) -> Result<(), SessionRuntimeError> {
        if !self.valid.load(Ordering::Acquire) {
            return Err(SessionRuntimeError::Unavailable);
        }
        self.runtime
            .block_on(self.handle.clear_context())
            .map_err(|_| SessionRuntimeError::Unavailable)
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
            .map_err(|error| ux::map_rewind_error(&error))
    }
}

#[cfg(test)]
#[path = "sessions/session_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "sessions/reasoning_tests.rs"]
mod reasoning_tests;

#[cfg(test)]
#[path = "sessions/archive_tests.rs"]
mod archive_tests;
#[cfg(test)]
mod rewind_message_tests;
#[cfg(test)]
mod slow_bypass_tests;
