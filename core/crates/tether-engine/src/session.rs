//! One actor per session serializes turns and commits streamed text before emitting it.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_providers::ChatProvider;
use tether_store::{BlobStore, SessionRecord, Store, StoreError};
use tether_types::{CoreError, CoreEvent, PermissionMode, SessionId, SessionStatus};
use tether_workspace::PathGuard;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::{Approver, EventSink, RewindError, RewindScope, ToolLoop, tool_loop::ToolLoopContext};

mod rewind;
mod stream;
mod turn;

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
}

enum Command {
    Send(String),
    Rewind {
        checkpoint_id: String,
        scope: RewindScope,
        reply: oneshot::Sender<Result<(), SessionActorError>>,
    },
    EnableBypass {
        reply: oneshot::Sender<Result<(), SessionActorError>>,
    },
    DisableBypass {
        reply: oneshot::Sender<Result<(), SessionActorError>>,
    },
}

/// Commands for one running session. Clones share its queue and interrupt token.
#[derive(Clone)]
pub struct SessionHandle {
    tx: mpsc::Sender<Command>,
    active: Arc<Mutex<Option<CancellationToken>>>,
}

impl SessionHandle {
    /// Queues one plain text turn. The actor serializes turns.
    ///
    /// # Errors
    /// Returns an error for empty text or a stopped actor.
    pub async fn send_message(&self, text: String) -> Result<(), SessionActorError> {
        if text.trim().is_empty() {
            return Err(SessionActorError::EmptyMessage);
        }
        self.tx
            .send(Command::Send(text))
            .await
            .map_err(|_| SessionActorError::Stopped)
    }

    /// Cancels the active provider request, if one is running.
    pub fn interrupt(&self) {
        if let Ok(active) = self.active.lock()
            && let Some(token) = active.as_ref()
        {
            token.cancel();
        }
    }

    /// Cancels the running turn and queues a serialized checkpoint restore.
    ///
    /// # Errors
    /// Returns an error when the actor stopped or the checkpoint cannot be restored.
    pub async fn rewind(
        &self,
        checkpoint_id: String,
        scope: RewindScope,
    ) -> Result<(), SessionActorError> {
        self.interrupt();
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Rewind {
                checkpoint_id,
                scope,
                reply,
            })
            .await
            .map_err(|_| SessionActorError::Stopped)?;
        result.await.map_err(|_| SessionActorError::Stopped)?
    }

    /// Enables the in-memory Bypass mode on this actor.
    ///
    /// # Errors
    /// Returns an error when the actor stopped or has no guarded workspace.
    pub async fn enable_bypass(&self) -> Result<(), SessionActorError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::EnableBypass { reply })
            .await
            .map_err(|_| SessionActorError::Stopped)?;
        result.await.map_err(|_| SessionActorError::Stopped)?
    }

    /// Disables the in-memory Bypass mode on this actor.
    ///
    /// # Errors
    /// Returns an error when the actor stopped or has no guarded workspace.
    pub async fn disable_bypass(&self) -> Result<(), SessionActorError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::DisableBypass { reply })
            .await
            .map_err(|_| SessionActorError::Stopped)?;
        result.await.map_err(|_| SessionActorError::Stopped)?
    }
}

/// Owns the database connection and the state of one session.
pub struct SessionActor {
    store: Store,
    session: SessionRecord,
    provider_for_turn: ProviderFactory,
    sink: Arc<dyn EventSink>,
    rx: mpsc::Receiver<Command>,
    active: Arc<Mutex<Option<CancellationToken>>>,
    tools: Option<ToolRuntime>,
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

    /// Loads a session with credentials resolved anew for each turn.
    ///
    /// # Errors
    /// Returns an error when the id is absent or the store cannot read it.
    pub fn new_with_provider_factory(
        store: Store,
        id: &SessionId,
        provider_for_turn: ProviderFactory,
        sink: Arc<dyn EventSink>,
    ) -> Result<(Self, SessionHandle), SessionActorError> {
        let session = store.load_session(id)?.ok_or(SessionActorError::NotFound)?;
        let (tx, rx) = mpsc::channel(8);
        let active = Arc::new(Mutex::new(None));
        let handle = SessionHandle {
            tx,
            active: Arc::clone(&active),
        };
        Ok((
            Self {
                store,
                session,
                provider_for_turn,
                sink,
                rx,
                active,
                tools: None,
            },
            handle,
        ))
    }

    /// Loads a session that can run guarded file tools and save checkpoints.
    ///
    /// # Errors
    /// Returns an error when the id is absent or the store cannot read it.
    pub fn new_with_tool_runtime(
        store: Store,
        id: &SessionId,
        provider_for_turn: ProviderFactory,
        sink: Arc<dyn EventSink>,
        tools: ToolRuntime,
    ) -> Result<(Self, SessionHandle), SessionActorError> {
        let (mut actor, handle) =
            Self::new_with_provider_factory(store, id, provider_for_turn, sink)?;
        actor.tools = Some(tools);
        Ok((actor, handle))
    }

    /// Processes messages until the caller drops every handle.
    #[expect(
        clippy::excessive_nesting,
        reason = "the serialized actor loop handles each turn's result"
    )]
    pub async fn run(mut self) {
        while let Some(command) = self.rx.recv().await {
            match command {
                Command::Send(text) => {
                    if let Err(error) = self.turn(text).await {
                        self.end_with_error(error);
                    }
                }
                Command::Rewind {
                    checkpoint_id,
                    scope,
                    reply,
                } => {
                    let result = self.rewind(&checkpoint_id, scope);
                    drop(reply.send(result));
                }
                Command::EnableBypass { reply } => {
                    drop(reply.send(self.set_bypass(true)));
                }
                Command::DisableBypass { reply } => {
                    drop(reply.send(self.set_bypass(false)));
                }
            }
        }
    }

    #[expect(
        clippy::excessive_nesting,
        reason = "cancel and failure end with distinct saved states and events"
    )]
    fn end_with_error(&mut self, error: CoreError) {
        let id = &self.session.id;
        if matches!(error, CoreError::Cancelled { .. }) {
            if self
                .store
                .set_session_status(id, SessionStatus::Idle, now_ms())
                .is_ok()
            {
                self.emit_status(SessionStatus::Idle);
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
