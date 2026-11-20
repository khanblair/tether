//! One actor per session serializes turns and commits streamed text before emitting it.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_providers::{ChatProvider, ChatRequest};
use tether_store::{BlobStore, SessionRecord, Store, StoreError};
use tether_types::{CoreError, CoreEvent, SessionId, SessionStatus, UsageSnapshot};
use tether_workspace::PathGuard;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::{
    EventSink, RewindError, RewindScope, ToolLoop, rewind_checkpoint, rewind_conversation_only,
    tool_loop::ToolLoopContext,
};

mod stream;

const TEXT_BATCH_MS: u64 = 32;
const TEXT_BATCH_CHARS: usize = 64;
const MAX_OUTPUT_TOKENS: u32 = 4096;

/// Resolves the provider and its current credentials when a turn begins.
pub type ProviderFactory = Arc<dyn Fn() -> Result<Arc<dyn ChatProvider>, CoreError> + Send + Sync>;

/// Approval callback for a proposed tool mutation.
pub type ToolApproval = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;

/// A guarded workspace and checkpoint store used by tool turns.
pub struct ToolRuntime {
    /// Restricts tools to one project workspace.
    pub guard: PathGuard,
    /// Stores prior bytes before file mutations.
    pub blobs: BlobStore,
    /// Decides whether a proposed file mutation may proceed.
    pub approve: ToolApproval,
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
            }
        }
    }

    fn rewind(&mut self, checkpoint_id: &str, scope: RewindScope) -> Result<(), SessionActorError> {
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

    #[expect(
        clippy::too_many_lines,
        reason = "one serialized turn owns persistence, tool execution, usage, and status"
    )]
    async fn turn(&mut self, text: String) -> Result<(), CoreError> {
        let id = self.session.id.clone();
        self.store
            .begin_turn(&id, &text, now_ms())
            .map_err(store_error)?;
        let messages = self
            .store
            .messages(&id)
            .map_err(store_error)?
            .into_iter()
            .filter(|message| !message.rewound)
            .collect();
        let request = ChatRequest {
            model_id: self.session.model_id.clone(),
            system: String::new(),
            messages,
            tools: Vec::new(),
            max_output_tokens: MAX_OUTPUT_TOKENS,
            reasoning: None,
        };
        let provider = (self.provider_for_turn)()?;
        let cancel = CancellationToken::new();
        if let Ok(mut active) = self.active.lock() {
            *active = Some(cancel.clone());
        }
        self.emit_status(SessionStatus::Requesting);
        let result = if let Some(tools) = &self.tools {
            let loop_runner = ToolLoop::new(tools.guard.clone());
            let mut context = ToolLoopContext {
                session_id: &id,
                store: &mut self.store,
                blobs: &tools.blobs,
                cancel: cancel.clone(),
                approve: tools.approve.as_ref(),
                sink: Some(self.sink.as_ref()),
            };
            let completed = loop_runner.run(provider, request, &mut context).await;
            drop(context);
            match completed {
                Ok(_) => (UsageSnapshot::default(), Ok(())),
                Err(error) => (UsageSnapshot::default(), Err(error)),
            }
        } else {
            self.stream(provider, request, cancel).await
        };
        if let Ok(mut active) = self.active.lock() {
            *active = None;
        }
        let (usage, result) = result;
        self.store
            .add_session_usage(&id, usage, now_ms())
            .map_err(store_error)?;
        let totals = self.store.session_usage(&id).map_err(store_error)?;
        self.sink.on_event(CoreEvent::UsageUpdated {
            session_id: id.clone(),
            usage: totals,
        });
        result?;
        self.store
            .set_session_status(&id, SessionStatus::Idle, now_ms())
            .map_err(store_error)?;
        self.emit_status(SessionStatus::Idle);
        self.sink.on_event(CoreEvent::TurnFinished {
            session_id: id.clone(),
        });
        Ok(())
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
