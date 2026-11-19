//! One actor per session serializes turns and commits streamed text before emitting it.

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tether_providers::{ChatProvider, ChatRequest, ResponseUsage, StopReason, StreamEvent};
use tether_store::{SessionRecord, Store, StoreError};
use tether_types::{CoreError, CoreEvent, MessageId, SessionId, SessionStatus, UsageSnapshot};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};
use tokio_util::sync::CancellationToken;

use crate::EventSink;

const TEXT_BATCH_MS: u64 = 32;
const TEXT_BATCH_CHARS: usize = 64;
const MAX_OUTPUT_TOKENS: u32 = 4096;

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
}

enum Command {
    Send(String),
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
}

/// Owns the database connection and the state of one session.
pub struct SessionActor {
    store: Store,
    session: SessionRecord,
    provider: Arc<dyn ChatProvider>,
    sink: Arc<dyn EventSink>,
    rx: mpsc::Receiver<Command>,
    active: Arc<Mutex<Option<CancellationToken>>>,
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
                provider,
                sink,
                rx,
                active,
            },
            handle,
        ))
    }

    /// Processes messages until the caller drops every handle.
    #[expect(
        clippy::excessive_nesting,
        reason = "the serialized actor loop handles each turn's result"
    )]
    pub async fn run(mut self) {
        while let Some(Command::Send(text)) = self.rx.recv().await {
            if let Err(error) = self.turn(text).await {
                self.end_with_error(error);
            }
        }
    }

    async fn turn(&mut self, text: String) -> Result<(), CoreError> {
        let id = self.session.id.clone();
        self.store
            .begin_turn(&id, &text, now_ms())
            .map_err(store_error)?;
        let messages = self.store.messages(&id).map_err(store_error)?;
        let request = ChatRequest {
            model_id: self.session.model_id.clone(),
            system: String::new(),
            messages,
            tools: Vec::new(),
            max_output_tokens: MAX_OUTPUT_TOKENS,
            reasoning: None,
        };
        let cancel = CancellationToken::new();
        if let Ok(mut active) = self.active.lock() {
            *active = Some(cancel.clone());
        }
        self.emit_status(SessionStatus::Requesting);
        let result = self.stream(request, cancel).await;
        if let Ok(mut active) = self.active.lock() {
            *active = None;
        }
        let usage = result?;
        self.store
            .add_session_usage(&id, usage, now_ms())
            .map_err(store_error)?;
        let totals = self.store.session_usage(&id).map_err(store_error)?;
        self.store
            .set_session_status(&id, SessionStatus::Idle, now_ms())
            .map_err(store_error)?;
        self.sink.on_event(CoreEvent::UsageUpdated {
            session_id: id.clone(),
            usage: totals,
        });
        self.emit_status(SessionStatus::Idle);
        self.sink.on_event(CoreEvent::TurnFinished {
            session_id: id.clone(),
        });
        Ok(())
    }

    async fn stream(
        &mut self,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> Result<UsageSnapshot, CoreError> {
        let (tx, mut rx) = mpsc::channel(32);
        let provider = Arc::clone(&self.provider);
        let task = tokio::spawn(async move { provider.stream_chat(&request, tx, cancel).await });
        let mut output = Output::default();
        let result = self.collect(&mut rx, &mut output).await;
        if let Err(error) = result {
            task.abort();
            return Err(error);
        }
        let provider_result = task.await.map_err(|_| CoreError::Internal {
            message: "provider task stopped".into(),
        })?;
        provider_result.map_err(CoreError::from)?;
        if !output.finished {
            return Err(CoreError::Internal {
                message: "provider stream ended without a finish event".into(),
            });
        }
        Ok(output.usage)
    }

    #[expect(
        clippy::excessive_nesting,
        reason = "the stream loop selects between events and the text batch deadline"
    )]
    async fn collect(
        &mut self,
        rx: &mut mpsc::Receiver<StreamEvent>,
        output: &mut Output,
    ) -> Result<(), CoreError> {
        loop {
            let event = if let Some(deadline) = output.deadline {
                tokio::select! {
                    item = rx.recv() => item,
                    () = sleep_until(deadline) => { self.flush(output)?; continue; }
                }
            } else {
                rx.recv().await
            };
            let Some(event) = event else { break };
            if let Err(error) = Self::accept_event(event, output) {
                self.flush(output)?;
                return Err(error);
            }
            if output.pending_chars >= TEXT_BATCH_CHARS {
                self.flush(output)?;
            }
        }
        self.flush(output)
    }

    #[expect(
        clippy::excessive_nesting,
        reason = "each provider event variant updates one output state"
    )]
    fn accept_event(event: StreamEvent, output: &mut Output) -> Result<(), CoreError> {
        match event {
            StreamEvent::TextDelta { text } => {
                output.text.push_str(&text);
                output.pending_text.push_str(&text);
                output.pending_chars += text.chars().count();
                output
                    .deadline
                    .get_or_insert_with(|| Instant::now() + Duration::from_millis(TEXT_BATCH_MS));
            }
            StreamEvent::ThinkingDelta { text } => {
                output.thinking.push_str(&text);
                output.pending_thinking.push_str(&text);
                output.pending_chars += text.chars().count();
                output
                    .deadline
                    .get_or_insert_with(|| Instant::now() + Duration::from_millis(TEXT_BATCH_MS));
            }
            StreamEvent::Usage(usage) => output.usage = usage_snapshot(usage),
            StreamEvent::Finished { stop_reason } => {
                if stop_reason == StopReason::ToolUse {
                    return Err(CoreError::InvalidInput {
                        message: "this plain chat turn cannot run tools".into(),
                    });
                }
                output.finished = true;
            }
            StreamEvent::ToolCallStarted { .. }
            | StreamEvent::ToolCallArgsDelta { .. }
            | StreamEvent::ToolCallFinished { .. } => {
                return Err(CoreError::InvalidInput {
                    message: "this plain chat turn cannot run tools".into(),
                });
            }
        }
        Ok(())
    }

    fn flush(&mut self, output: &mut Output) -> Result<(), CoreError> {
        if output.pending_text.is_empty() && output.pending_thinking.is_empty() {
            output.deadline = None;
            return Ok(());
        }
        let id = &self.session.id;
        let message_id = if let Some(id) = &output.message_id {
            id.clone()
        } else {
            let created = self
                .store
                .create_assistant_message(id, now_ms())
                .map_err(store_error)?;
            output.message_id = Some(created.clone());
            self.store
                .set_session_status(id, SessionStatus::Streaming, now_ms())
                .map_err(store_error)?;
            self.emit_status(SessionStatus::Streaming);
            created
        };
        self.store
            .save_assistant_parts(&message_id, &output.text, &output.thinking)
            .map_err(store_error)?;
        if !output.pending_thinking.is_empty() {
            self.sink.on_event(CoreEvent::ThinkingDelta {
                session_id: id.clone(),
                message_id: message_id.clone(),
                text: std::mem::take(&mut output.pending_thinking),
            });
        }
        if !output.pending_text.is_empty() {
            self.sink.on_event(CoreEvent::TextDelta {
                session_id: id.clone(),
                message_id,
                text: std::mem::take(&mut output.pending_text),
            });
        }
        output.pending_chars = 0;
        output.deadline = None;
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

#[derive(Default)]
struct Output {
    text: String,
    thinking: String,
    pending_text: String,
    pending_thinking: String,
    pending_chars: usize,
    deadline: Option<Instant>,
    message_id: Option<MessageId>,
    usage: UsageSnapshot,
    finished: bool,
}

fn usage_snapshot(usage: ResponseUsage) -> UsageSnapshot {
    UsageSnapshot {
        tokens_in: usage
            .input_tokens
            .saturating_add(usage.cache_read_tokens)
            .saturating_add(usage.cache_write_tokens),
        tokens_out: usage.output_tokens,
        cost_micros: 0,
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
