//! UniFFI entry point for session creation, transcript reads, and plain chat turns.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_engine::{ProviderFactory, SessionActor, SessionHandle, ToolRuntime, provider_for};
use tether_secrets::SecretRef;
use tether_store::{BlobStore, SessionRecord, Store};
use tether_types::{CoreError, ModelRef, PermissionMode, ProjectId, Provider, SessionId};
use tether_workspace::PathGuard;

use crate::{EventSink, MessageView, ProviderView, SecretVault, SessionStatusView};

/// A session could not be created, opened, or updated.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SessionRuntimeError {
    /// The requested session does not exist.
    #[error("session not found")]
    NotFound,
    /// The selected provider has no saved key.
    #[error("provider key is not saved")]
    MissingKey,
    /// The model id or message is empty.
    #[error("a model and message must have text")]
    EmptyInput,
    /// The session store or provider is unavailable.
    #[error("session is unavailable")]
    Unavailable,
}

/// Saved session settings and status for the platform.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SessionRecordView {
    /// Session id.
    pub id: String,
    /// Project id.
    pub project_id: String,
    /// Display title.
    pub title: String,
    /// Provider.
    pub provider: ProviderView,
    /// Provider model id.
    pub model_id: String,
    /// Saved status.
    pub status: SessionStatusView,
}

impl From<SessionRecord> for SessionRecordView {
    fn from(session: SessionRecord) -> Self {
        Self {
            id: session.id.into(),
            project_id: session.project_id.into(),
            title: session.title,
            provider: session.provider.into(),
            model_id: session.model_id,
            status: session.status.into(),
        }
    }
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
    let may_edit = matches!(
        session.mode,
        PermissionMode::AcceptEdits | PermissionMode::Auto | PermissionMode::Bypass
    );
    Ok(ToolRuntime {
        guard,
        blobs,
        approve: Arc::new(move |_, _| may_edit),
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
}

#[uniffi::export]
impl SessionRunner {
    /// Queues a plain text turn. Events arrive through the runtime's sink.
    /// Call from a background thread.
    ///
    /// # Errors
    /// Returns an error for empty input or a stopped actor.
    pub fn send_message(&self, text: String) -> Result<(), SessionRuntimeError> {
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
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SecretsPlatform;
    use tether_types::{ModelRef, SessionStatus};

    struct PlainPlatform;

    impl SecretsPlatform for PlainPlatform {
        fn wrap(
            &self,
            plaintext: Vec<u8>,
            _context: Vec<u8>,
        ) -> Result<Vec<u8>, crate::SecretsPlatformFailure> {
            Ok(plaintext)
        }

        fn unwrap(
            &self,
            wrapped: Vec<u8>,
            _context: Vec<u8>,
        ) -> Result<Vec<u8>, crate::SecretsPlatformFailure> {
            Ok(wrapped)
        }
    }

    struct NoopSink;

    impl EventSink for NoopSink {
        fn on_event(&self, _event: crate::CoreEventView) {}
    }

    fn seeded_session(db: &Path) -> SessionId {
        let mut store = Store::open(db).unwrap();
        let project = ProjectId::new("01TESTPROJECT00000000000000");
        rusqlite::Connection::open(db)
            .unwrap()
            .execute(
                "INSERT INTO projects (id, name, path, created_at) VALUES (?1, 'Test', 'test', 1)",
                [project.as_str()],
            )
            .unwrap();
        let session = store
            .create_session(
                &project,
                &ModelRef {
                    provider: Provider::Anthropic,
                    model_id: "test-model".into(),
                },
                PermissionMode::Ask,
                1,
            )
            .unwrap();
        store.begin_turn(&session.id, "Hi", 2).unwrap();
        session.id
    }

    #[test]
    fn reopening_recovers_an_active_turn_and_reuses_its_actor() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("tether.db");
        let session_id = seeded_session(&db);

        let vault = SecretVault::new(
            dir.path().join("secrets").to_string_lossy().into_owned(),
            Arc::new(PlainPlatform),
        );
        let runtime =
            SessionRuntime::new(db.to_str().unwrap(), Arc::clone(&vault), Arc::new(NoopSink))
                .unwrap();
        let view = runtime.load_session(session_id.as_str()).unwrap();
        assert_eq!(view.status, SessionStatusView::Interrupted);
        assert_eq!(runtime.transcript(session_id.as_str()).unwrap().len(), 1);
        assert!(matches!(
            runtime.open_session(session_id.as_str()),
            Err(SessionRuntimeError::MissingKey)
        ));
        vault.put("provider-anthropic", "test-key").unwrap();
        let first = runtime.open_session(session_id.as_str()).unwrap();
        let again = runtime.open_session(session_id.as_str()).unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        assert_eq!(
            Store::open(&db)
                .unwrap()
                .load_session(&session_id)
                .unwrap()
                .unwrap()
                .status,
            SessionStatus::Interrupted
        );
    }

    #[test]
    fn workspace_session_requires_a_saved_project_folder() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("tether.db");
        let session_id = seeded_session(&db);
        let vault = SecretVault::new(
            dir.path().join("secrets").to_string_lossy().into_owned(),
            Arc::new(PlainPlatform),
        );
        vault.put("provider-anthropic", "test-key").unwrap();
        let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
        let root = dir.path().join("workspaces");
        std::fs::create_dir(&root).unwrap();
        assert!(matches!(
            runtime.open_session_with_workspace(session_id.as_str(), root.to_str().unwrap()),
            Err(SessionRuntimeError::Unavailable)
        ));
        std::fs::create_dir(root.join("test")).unwrap();
        assert!(
            runtime
                .open_session_with_workspace(session_id.as_str(), root.to_str().unwrap())
                .is_ok()
        );
    }

    #[test]
    fn provider_factory_reads_the_current_key_after_removal() {
        let dir = tempfile::tempdir().unwrap();
        let vault = SecretVault::new(
            dir.path().join("secrets").to_string_lossy().into_owned(),
            Arc::new(PlainPlatform),
        );
        vault.put("provider-anthropic", "old-key").unwrap();
        let factory = session_provider_factory(Arc::clone(&vault), Provider::Anthropic);
        assert!(factory().is_ok());
        vault.delete("provider-anthropic").unwrap();
        assert!(matches!(
            factory(),
            Err(tether_types::CoreError::Auth { .. })
        ));
        vault.put("provider-anthropic", "new-key").unwrap();
        assert!(factory().is_ok());
    }
}
