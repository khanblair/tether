//! User-facing question resolution for session actors.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tether_engine::{Answer, Asker, BoxFuture, QuestionRequest};
use tether_types::{SessionId, ToolCallId};
use tokio::sync::oneshot;

use crate::views::PendingQuestionView;
use crate::{SessionRuntime, SessionRuntimeError};

struct PendingQuestion {
    request: QuestionRequest,
    answer: oneshot::Sender<Answer>,
}

/// Coordinates question requests between an actor and the app.
pub(crate) struct QuestionBroker {
    interactive: AtomicBool,
    pending: Mutex<HashMap<ToolCallId, PendingQuestion>>,
}

impl QuestionBroker {
    pub(crate) fn new() -> Self {
        Self {
            interactive: AtomicBool::new(false),
            pending: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn set_interactive(&self, enabled: bool) {
        self.interactive.store(enabled, Ordering::Release);
    }

    fn answer(
        &self,
        session_id: &SessionId,
        tool_call_id: &ToolCallId,
        answer: String,
    ) -> Result<(), SessionRuntimeError> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        let belongs = pending
            .get(tool_call_id)
            .is_some_and(|question| question.request.session_id == *session_id);
        if !belongs {
            return Err(SessionRuntimeError::NotFound);
        }
        let question = pending
            .remove(tool_call_id)
            .ok_or(SessionRuntimeError::NotFound)?;
        drop(question.answer.send(Answer::Text(answer)));
        Ok(())
    }

    fn pending_for(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<PendingQuestionView>, SessionRuntimeError> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        let mut questions: Vec<_> = pending
            .values()
            .filter(|question| question.request.session_id == *session_id)
            .map(|question| PendingQuestionView {
                tool_call_id: question.request.tool_call_id.to_string(),
                question: question.request.question.clone(),
                options: question.request.options.clone(),
                allow_free_text: question.request.allow_free_text,
            })
            .collect();
        questions.sort_by(|left, right| left.tool_call_id.cmp(&right.tool_call_id));
        Ok(questions)
    }

    fn remove(&self, tool_call_id: &ToolCallId) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(tool_call_id);
        }
    }
}

impl Asker for QuestionBroker {
    fn interactive(&self) -> bool {
        self.interactive.load(Ordering::Acquire)
    }

    fn ask(
        &self,
        request: QuestionRequest,
        cancel: tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'_, Answer> {
        let (sender, receiver) = oneshot::channel();
        let inserted = self
            .pending
            .lock()
            .map(|mut pending| {
                pending.insert(
                    request.tool_call_id.clone(),
                    PendingQuestion {
                        request: request.clone(),
                        answer: sender,
                    },
                );
            })
            .is_ok();
        if !inserted {
            return Box::pin(std::future::ready(Answer::Cancelled));
        }
        let tool_call_id = request.tool_call_id;
        Box::pin(async move {
            tokio::select! {
                answer = receiver => answer.unwrap_or(Answer::Cancelled),
                () = cancel.cancelled() => {
                    self.remove(&tool_call_id);
                    Answer::Cancelled
                }
            }
        })
    }
}

#[uniffi::export]
impl SessionRuntime {
    /// Answers a pending question.
    ///
    /// # Errors
    /// Returns `NotFound` when the question is unknown, answered, or belongs to another session.
    pub fn answer_question(
        &self,
        session_id: &str,
        tool_call_id: &str,
        answer: String,
    ) -> Result<(), SessionRuntimeError> {
        self.broker.questions().answer(
            &SessionId::new(session_id),
            &ToolCallId::new(tool_call_id),
            answer,
        )
    }

    /// Lists questions currently waiting for a session answer.
    ///
    /// # Errors
    /// Returns `Unavailable` when the broker lock is poisoned.
    pub fn pending_questions(
        &self,
        session_id: &str,
    ) -> Result<Vec<PendingQuestionView>, SessionRuntimeError> {
        self.broker
            .questions()
            .pending_for(&SessionId::new(session_id))
    }
}
