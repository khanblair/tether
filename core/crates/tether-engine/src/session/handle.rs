//! The command queue and the handle that callers use to drive one session actor.

use std::sync::{Arc, Mutex};

use tether_types::PermissionMode;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use super::SessionActorError;
use crate::RewindScope;
use crate::commands::MentionResolution;

pub(super) enum Command {
    Send {
        text: String,
        outcome: Option<oneshot::Sender<MentionResolution>>,
    },
    Rewind {
        checkpoint_id: String,
        scope: RewindScope,
        reply: oneshot::Sender<Result<(), SessionActorError>>,
    },
    RewindToMessage {
        message_id: String,
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
    pub(super) tx: mpsc::Sender<Command>,
    pub(super) active: Arc<Mutex<Option<CancellationToken>>>,
    pub(super) budget: Arc<crate::budget::BudgetGate>,
    pub(super) bypass_mode: Option<Arc<Mutex<PermissionMode>>>,
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

    /// Cancels the running turn and hides every message from `message_id` onward.
    /// Files are not touched, so this works without a workspace runtime.
    ///
    /// # Errors
    /// Returns an error when the actor stopped or the message is not in this session.
    pub async fn rewind_to_message(&self, message_id: String) -> Result<(), SessionActorError> {
        self.interrupt();
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::RewindToMessage { message_id, reply })
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

    /// Reports whether the in-memory Bypass mode is on for this actor.
    ///
    /// # Errors
    /// Returns an error when the actor stopped or its mode lock is poisoned.
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "Keep the async handle contract while reading the shared lock directly"
    )]
    pub async fn bypass_active(&self) -> Result<bool, SessionActorError> {
        let Some(mode) = &self.bypass_mode else {
            return Ok(false);
        };
        mode.lock()
            .map(|mode| *mode == PermissionMode::Bypass)
            .map_err(|_| SessionActorError::Stopped)
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
