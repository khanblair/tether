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

use crate::commands::{MentionResolution, resolve_mentions};
use crate::{
    Approver, EventSink, RewindError, RewindScope, ToolLoop, prompt::PromptSources,
    tool_loop::ToolLoopContext,
};

mod prompt;
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

enum Command {
    Send {
        text: String,
        outcome: Option<oneshot::Sender<MentionResolution>>,
    },
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
    RunPlan {
        mode: PermissionMode,
        reply: oneshot::Sender<Result<(), SessionActorError>>,
    },
    ClearContext {
        reply: oneshot::Sender<Result<(), SessionActorError>>,
    },
    Resume {
        reply: oneshot::Sender<Result<(), SessionActorError>>,
    },
    DiscardInterrupted {
        reply: oneshot::Sender<Result<(), SessionActorError>>,
    },
    Compact {
        focus: Option<String>,
        reply: oneshot::Sender<Result<(), SessionActorError>>,
    },
}

/// Commands for one running session. Clones share its queue and interrupt token.
#[derive(Clone)]
pub struct SessionHandle {
    tx: mpsc::Sender<Command>,
    active: Arc<Mutex<Option<CancellationToken>>>,
    budget: Arc<crate::budget::BudgetGate>,
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
            .send(Command::Send {
                text,
                outcome: None,
            })
            .await
            .map_err(|_| SessionActorError::Stopped)
    }

    /// Queues a turn and returns the file mentions resolved before the provider call.
    ///
    /// # Errors
    /// Returns an error when the actor stopped or the message is empty.
    pub async fn send_message_with_mentions(
        &self,
        text: String,
    ) -> Result<MentionResolution, SessionActorError> {
        if text.trim().is_empty() {
            return Err(SessionActorError::EmptyMessage);
        }
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Send {
                text,
                outcome: Some(reply),
            })
            .await
            .map_err(|_| SessionActorError::Stopped)?;
        result.await.map_err(|_| SessionActorError::Stopped)
    }

    /// Cancels the active provider request, if one is running.
    pub fn interrupt(&self) {
        if let Ok(active) = self.active.lock()
            && let Some(token) = active.as_ref()
        {
            token.cancel();
        }
    }

    /// Continues a paused turn. The same limit pauses again after another 20% of
    /// that limit has been spent.
    pub fn continue_past_budget(&self) {
        self.budget.continue_past_budget();
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

    /// Switches to a working mode and starts the plan turn.
    ///
    /// # Errors
    /// Returns an error when the actor stopped or the saved mode cannot change.
    pub async fn run_plan(&self, mode: PermissionMode) -> Result<(), SessionActorError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::RunPlan { mode, reply })
            .await
            .map_err(|_| SessionActorError::Stopped)?;
        result.await.map_err(|_| SessionActorError::Stopped)?
    }

    /// Clears live model context and keeps the saved transcript.
    ///
    /// # Errors
    /// Returns an error when the actor stopped or storage failed.
    pub async fn clear_context(&self) -> Result<(), SessionActorError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::ClearContext { reply })
            .await
            .map_err(|_| SessionActorError::Stopped)?;
        result.await.map_err(|_| SessionActorError::Stopped)?
    }

    /// Repairs an interrupted transcript and resumes its last turn.
    ///
    /// # Errors
    /// Returns an error when the session is not interrupted or the actor stopped.
    pub async fn resume(&self) -> Result<(), SessionActorError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Resume { reply })
            .await
            .map_err(|_| SessionActorError::Stopped)?;
        result.await.map_err(|_| SessionActorError::Stopped)?
    }

    /// Repairs an interrupted transcript and leaves the session idle.
    ///
    /// # Errors
    /// Returns an error when the session is not interrupted or the actor stopped.
    pub async fn discard_interrupted(&self) -> Result<(), SessionActorError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::DiscardInterrupted { reply })
            .await
            .map_err(|_| SessionActorError::Stopped)?;
        result.await.map_err(|_| SessionActorError::Stopped)?
    }

    /// Compacts the saved transcript with an optional focus.
    ///
    /// # Errors
    /// Returns an error when storage or the summary request fails.
    pub async fn compact(&self, focus: Option<String>) -> Result<(), SessionActorError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Compact { focus, reply })
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
    #[expect(
        clippy::excessive_nesting,
        reason = "the serialized actor loop handles each turn's result"
    )]
    pub async fn run(mut self) {
        while let Some(command) = self.rx.recv().await {
            match command {
                Command::Send { text, outcome } => {
                    let resolution = self.resolve_mentions(&text);
                    if let Some(reply) = outcome {
                        drop(reply.send(resolution.clone()));
                    }
                    if let Err(error) = self.turn(text, resolution).await {
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
                Command::RunPlan { mode, reply } => {
                    let result = self.set_working_mode(mode);
                    let started = result.is_ok();
                    drop(reply.send(result));
                    if started
                        && let Err(error) = self
                            .turn("Run the plan.".into(), MentionResolution::default())
                            .await
                    {
                        self.end_with_error(error);
                    }
                }
                Command::ClearContext { reply } => {
                    let result = self.clear_context();
                    drop(reply.send(result));
                }
                Command::Resume { reply } => {
                    let result = self.resume_turn().await;
                    if let Err(SessionActorError::Turn(error)) = &result {
                        self.end_with_error(error.clone());
                    }
                    drop(reply.send(result));
                }
                Command::DiscardInterrupted { reply } => {
                    let result = self.discard_interrupted();
                    drop(reply.send(result));
                }
                Command::Compact { focus, reply } => {
                    let result = self.compact(focus).await;
                    drop(reply.send(result));
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
        crate::session::turn::compact_session(self, focus)
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
