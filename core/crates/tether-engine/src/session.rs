//! One actor per session serializes turns and commits streamed text before emitting it.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_providers::ChatProvider;
use tether_store::{BlobStore, SessionRecord, Store, StoreError};
use tether_types::{CoreError, CoreEvent, PermissionMode, SessionId, SessionStatus};
use tether_workspace::PathGuard;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::commands::{MentionResolution, resolve_mentions};
use crate::{
    Approver, EventSink, RewindError, RewindScope, ToolLoop, prompt::PromptSources,
    tool_loop::ToolLoopContext,
};

mod compact;
mod dispatch;
mod handle;
mod prompt;
mod rewind;
mod stream;
mod turn;

use handle::Command;
pub use handle::SessionHandle;

const TEXT_BATCH_MS: u64 = 32;
const TEXT_BATCH_CHARS: usize = 64;
const MAX_OUTPUT_TOKENS: u32 = 4096;

/// Resolves the provider and its current credentials when a turn begins.
pub type ProviderFactory = Arc<dyn Fn() -> Result<Arc<dyn ChatProvider>, CoreError> + Send + Sync>;

/// A guarded workspace and checkpoint store used by tool turns.
pub struct ToolRuntime {
    /// Restricts tools to one project workspace.
    pub guard: PathGuard,
    /// Stores prior bytes before file mutations.
    pub blobs: BlobStore,
    /// Resolves interactive approvals.
    pub approver: Arc<dyn Approver>,
    /// Current mode, including an in-memory Bypass value.
    pub mode: Arc<Mutex<PermissionMode>>,
    /// Tool handlers beyond the file tools: run, git, CI, web, MCP, checklist, and the rest.
    /// The session setup builds them with their own dependencies.
    pub handlers: Vec<Arc<dyn crate::ToolHandler>>,
}

/// Dependencies used when opening a session actor.
pub struct SessionActorOptions {
    /// Guarded tool runtime, when tools are enabled.
    pub tools: Option<ToolRuntime>,
    /// Prompt files and environment values.
    pub prompt_sources: PromptSources,
    /// Optional platform network state.
    pub network: Option<Arc<crate::network::NetworkControl>>,
}

/// A session command could not be accepted or the session could not open.
#[derive(Debug, Error)]
pub enum SessionActorError {
    /// The session id is absent.
    #[error("session not found")]
    NotFound,
    /// The actor has stopped.
    #[error("session actor stopped")]
    Stopped,
    /// An empty message was offered.
    #[error("message is empty")]
    EmptyMessage,
    /// The store could not open or read the session.
    #[error("session store failed: {0}")]
    Store(#[from] StoreError),
    /// The actor has no guarded workspace for restoring files.
    #[error("session has no workspace runtime")]
    NoWorkspace,
    /// A checkpoint could not be restored.
    #[error(transparent)]
    Rewind(#[from] RewindError),
    /// Prompt source data could not be loaded.
    #[error("prompt sources unavailable")]
    Prompt,
    /// A resumed turn failed after it started.
    #[error(transparent)]
    Turn(#[from] CoreError),
    /// The session is not in the interrupted state.
    #[error("session is not interrupted")]
    NotInterrupted,
}

/// Owns the database connection and the state of one session.
pub struct SessionActor {
    store: Store,
    session: SessionRecord,
    provider_for_turn: ProviderFactory,
    sink: Arc<dyn EventSink>,
    rx: mpsc::Receiver<Command>,
    active: Arc<Mutex<Option<CancellationToken>>>,
    budget: Arc<crate::budget::BudgetGate>,
    tools: Option<ToolRuntime>,
    prompt_sources: PromptSources,
    prompt_snapshot: Option<tether_store::PromptSnapshot>,
    project_name: String,
    network: Option<Arc<crate::network::NetworkControl>>,
}

impl SessionActor {
    /// Loads a saved session and returns its actor and command handle.
    ///
    /// # Errors
    /// Returns an error when the id is absent or the store cannot read it.
    pub fn new(
        store: Store,
        id: &SessionId,
        provider: Arc<dyn ChatProvider>,
        sink: Arc<dyn EventSink>,
    ) -> Result<(Self, SessionHandle), SessionActorError> {
        Self::new_with_provider_factory(
            store,
            id,
            Arc::new(move || Ok(Arc::clone(&provider))),
            sink,
        )
    }

    /// Processes messages until the caller drops every handle.
    pub async fn run(mut self) {
        while let Some(command) = self.rx.recv().await {
            self.handle_command(command).await;
        }
    }

    #[expect(
        clippy::excessive_nesting,
        reason = "cancel and failure end with distinct saved states and events"
    )]
    fn end_with_error(&mut self, error: CoreError) {
        let id = &self.session.id;
        if matches!(error, CoreError::Cancelled { .. }) {
            let paused = self
                .store
                .load_session(id)
                .ok()
                .flatten()
                .is_some_and(|session| session.status == SessionStatus::Paused);
            let status = if paused {
                SessionStatus::Interrupted
            } else {
                SessionStatus::Idle
            };
            if self.store.set_session_status(id, status, now_ms()).is_ok() {
                self.emit_status(status);
                self.sink.on_event(CoreEvent::TurnInterrupted {
                    session_id: id.clone(),
                });
                return;
            }
            self.emit_status(SessionStatus::Failed);
            self.sink.on_event(CoreEvent::TurnFailed {
                session_id: id.clone(),
                error: storage_failure(),
            });
        } else {
            let error = if self
                .store
                .set_session_status(id, SessionStatus::Failed, now_ms())
                .is_ok()
            {
                error
            } else {
                storage_failure()
            };
            self.emit_status(SessionStatus::Failed);
            self.sink.on_event(CoreEvent::TurnFailed {
                session_id: id.clone(),
                error,
            });
        }
    }

    fn emit_status(&self, status: SessionStatus) {
        self.sink.on_event(CoreEvent::SessionUpdated {
            session_id: self.session.id.clone(),
            status,
        });
    }

    fn set_bypass(&self, enabled: bool) -> Result<(), SessionActorError> {
        let Some(tools) = &self.tools else {
            return Err(SessionActorError::NoWorkspace);
        };
        let mut mode = tools.mode.lock().map_err(|_| SessionActorError::Stopped)?;
        *mode = if enabled {
            PermissionMode::Bypass
        } else {
            self.session.mode
        };
        drop(mode);
        self.emit_status(SessionStatus::Idle);
        Ok(())
    }

    fn set_working_mode(&mut self, mode: PermissionMode) -> Result<(), SessionActorError> {
        self.store
            .set_session_mode(&self.session.id, mode, now_ms())?;
        self.session.mode = mode;
        let tools = self.tools.as_ref().ok_or(SessionActorError::NoWorkspace)?;
        let mut current = tools.mode.lock().map_err(|_| SessionActorError::Stopped)?;
        *current = mode;
        Ok(())
    }

    fn resolve_mentions(&self, text: &str) -> MentionResolution {
        self.tools
            .as_ref()
            .map_or_else(MentionResolution::default, |tools| {
                resolve_mentions(&tools.guard, text)
            })
    }

    fn clear_context(&mut self) -> Result<(), SessionActorError> {
        self.store.clear_context(&self.session.id)?;
        self.sink.on_event(CoreEvent::SessionRewound {
            session_id: self.session.id.clone(),
        });
        Ok(())
    }

    async fn resume_turn(&mut self) -> Result<(), SessionActorError> {
        let session = self
            .store
            .load_session(&self.session.id)
            .map_err(SessionActorError::Store)?
            .ok_or(SessionActorError::NotFound)?;
        if session.status != SessionStatus::Interrupted {
            return Err(SessionActorError::NotInterrupted);
        }
        self.store.repair_interrupted(&self.session.id)?;
        self.session.status = SessionStatus::Idle;
        self.turn(String::new(), MentionResolution::default())
            .await
            .map_err(Into::into)
    }

    fn discard_interrupted(&mut self) -> Result<(), SessionActorError> {
        let session = self
            .store
            .load_session(&self.session.id)?
            .ok_or(SessionActorError::NotFound)?;
        if session.status != SessionStatus::Interrupted {
            return Err(SessionActorError::NotInterrupted);
        }
        self.store.repair_interrupted(&self.session.id)?;
        self.store
            .set_session_status(&self.session.id, SessionStatus::Idle, now_ms())?;
        self.session.status = SessionStatus::Idle;
        self.emit_status(SessionStatus::Idle);
        Ok(())
    }

    async fn compact(&mut self, focus: Option<String>) -> Result<(), SessionActorError> {
        crate::session::compact::compact_session(self, focus)
            .await
            .map_err(|_| SessionActorError::Prompt)
    }
}

/// A provider stream that stops without its finish event was cut off.
fn require_finished(finished: bool) -> Result<(), CoreError> {
    if finished {
        Ok(())
    } else {
        Err(CoreError::Internal {
            message: "provider stream ended without a finish event".into(),
        })
    }
}

fn store_error(_error: StoreError) -> CoreError {
    storage_failure()
}

fn storage_failure() -> CoreError {
    CoreError::Internal {
        message: "could not save the session".into(),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}
