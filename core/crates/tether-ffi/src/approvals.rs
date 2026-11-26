//! User-facing approval resolution for session actors.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tether_engine::BoxFuture;
use tether_engine::{ApprovalAnswer, ApprovalRequest, Approver};
use tether_types::{ProjectId, SessionId, ToolCallId};
use tokio::sync::oneshot;

use crate::questions::QuestionBroker;
use crate::views::{PendingApprovalView, PermissionRuleView};
use crate::{SessionRuntime, SessionRuntimeError};

struct Pending {
    request: ApprovalRequest,
    answer: oneshot::Sender<ApprovalAnswer>,
}

/// Coordinates approval requests between the actor and the app.
pub(crate) struct ApprovalBroker {
    interactive: AtomicBool,
    pending: Mutex<HashMap<ToolCallId, Pending>>,
    questions: Arc<QuestionBroker>,
}

impl ApprovalBroker {
    pub(crate) fn new() -> Self {
        Self {
            interactive: AtomicBool::new(false),
            pending: Mutex::new(HashMap::new()),
            questions: Arc::new(QuestionBroker::new()),
        }
    }

    pub(crate) fn questions(&self) -> Arc<QuestionBroker> {
        Arc::clone(&self.questions)
    }

    pub(crate) fn set_interactive(&self, enabled: bool) {
        self.interactive.store(enabled, Ordering::Release);
    }

    fn answer(
        &self,
        session_id: &SessionId,
        tool_call_id: &ToolCallId,
        answer: ApprovalAnswer,
    ) -> Result<(), SessionRuntimeError> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        if pending
            .get(tool_call_id)
            .is_none_or(|pending| pending.request.session_id != *session_id)
        {
            return Err(SessionRuntimeError::NotFound);
        }
        let pending = pending
            .remove(tool_call_id)
            .ok_or(SessionRuntimeError::NotFound)?;
        drop(pending.answer.send(answer));
        Ok(())
    }

    fn answer_from_notification(
        &self,
        session_id: &SessionId,
        tool_call_id: &ToolCallId,
        allow: bool,
    ) -> Result<(), SessionRuntimeError> {
        let answer = if allow {
            ApprovalAnswer::AllowFromNotification { always: false }
        } else {
            ApprovalAnswer::DenyFromNotification { reason: None }
        };
        self.answer(session_id, tool_call_id, answer)
    }

    fn pending_for(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<PendingApprovalView>, SessionRuntimeError> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        Ok(pending
            .values()
            .filter(|pending| pending.request.session_id == *session_id)
            .map(|pending| PendingApprovalView {
                tool_call_id: pending.request.tool_call_id.to_string(),
                tool: pending.request.tool.clone(),
                target: pending.request.target.clone(),
            })
            .collect())
    }

    fn remove(&self, tool_call_id: &ToolCallId) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(tool_call_id);
        }
    }
}

impl Approver for ApprovalBroker {
    fn interactive(&self) -> bool {
        self.interactive.load(Ordering::Acquire)
    }

    fn decide(
        &self,
        request: ApprovalRequest,
        cancel: tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'_, ApprovalAnswer> {
        if !self.interactive() {
            return Box::pin(std::future::ready(ApprovalAnswer::Deny {
                reason: Some("approvals need a newer app version".into()),
            }));
        }
        let (sender, receiver) = oneshot::channel();
        let inserted = self
            .pending
            .lock()
            .map(|mut pending| {
                pending.insert(
                    request.tool_call_id.clone(),
                    Pending {
                        request: request.clone(),
                        answer: sender,
                    },
                );
            })
            .is_ok();
        if !inserted {
            return Box::pin(std::future::ready(ApprovalAnswer::Deny {
                reason: Some("approval is unavailable".into()),
            }));
        }
        let tool_call_id = request.tool_call_id;
        Box::pin(async move {
            tokio::select! {
                answer = receiver => answer.unwrap_or(ApprovalAnswer::Deny { reason: Some("cancelled".into()) }),
                () = cancel.cancelled() => {
                    self.remove(&tool_call_id);
                    ApprovalAnswer::Deny { reason: Some("cancelled".into()) }
                }
            }
        })
    }
}

#[uniffi::export]
impl SessionRuntime {
    /// Enables in-memory Bypass for an already opened workspace session.
    ///
    /// # Errors
    /// Returns `NotFound` when the session is not open or `Unavailable` when its actor stopped.
    pub fn enable_bypass(&self, session_id: &str) -> Result<(), SessionRuntimeError> {
        let runner = self.runner(session_id)?;
        self.runtime
            .block_on(runner.handle.enable_bypass())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Disables in-memory Bypass for an already opened workspace session.
    ///
    /// # Errors
    /// Returns `NotFound` when the session is not open or `Unavailable` when its actor stopped.
    pub fn disable_bypass(&self, session_id: &str) -> Result<(), SessionRuntimeError> {
        let runner = self.runner(session_id)?;
        self.runtime
            .block_on(runner.handle.disable_bypass())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Reports whether in-memory Bypass is on for an opened session.
    ///
    /// # Errors
    /// Returns `NotFound` when the session is not open or `Unavailable` when its actor stopped.
    pub fn bypass_active(&self, session_id: &str) -> Result<bool, SessionRuntimeError> {
        let runner = self.runner(session_id)?;
        self.runtime
            .block_on(runner.handle.bypass_active())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Answers a pending approval with Allow.
    ///
    /// # Errors
    /// Returns `NotFound` when the call is unknown, answered, or belongs to another session.
    pub fn approve(
        &self,
        session_id: &str,
        tool_call_id: &str,
        always: bool,
    ) -> Result<(), SessionRuntimeError> {
        self.broker.answer(
            &SessionId::new(session_id),
            &ToolCallId::new(tool_call_id),
            ApprovalAnswer::Allow { always },
        )
    }

    /// Answers a pending approval with Deny.
    ///
    /// # Errors
    /// Returns `NotFound` when the call is unknown, answered, or belongs to another session.
    pub fn deny(
        &self,
        session_id: &str,
        tool_call_id: &str,
        reason: Option<String>,
    ) -> Result<(), SessionRuntimeError> {
        self.broker.answer(
            &SessionId::new(session_id),
            &ToolCallId::new(tool_call_id),
            ApprovalAnswer::Deny { reason },
        )
    }

    /// Answers a pending approval from a background notification.
    ///
    /// # Errors
    /// Returns `NotFound` when the call is unknown or already answered.
    pub fn approve_from_notification(
        &self,
        session_id: &str,
        tool_call_id: &str,
    ) -> Result<(), SessionRuntimeError> {
        self.broker.answer_from_notification(
            &SessionId::new(session_id),
            &ToolCallId::new(tool_call_id),
            true,
        )
    }

    /// Denies a pending approval from a background notification.
    ///
    /// # Errors
    /// Returns `NotFound` when the call is unknown or already answered.
    pub fn deny_from_notification(
        &self,
        session_id: &str,
        tool_call_id: &str,
    ) -> Result<(), SessionRuntimeError> {
        self.broker.answer_from_notification(
            &SessionId::new(session_id),
            &ToolCallId::new(tool_call_id),
            false,
        )
    }

    /// Lists approvals waiting for one session.
    ///
    /// # Errors
    /// Returns `NotFound` for an unknown session or an unavailable store.
    pub fn pending_approvals(
        &self,
        session_id: &str,
    ) -> Result<Vec<PendingApprovalView>, SessionRuntimeError> {
        let id = SessionId::new(session_id);
        if self
            .store()?
            .load_session(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .is_none()
        {
            return Err(SessionRuntimeError::NotFound);
        }
        self.broker.pending_for(&id)
    }

    /// Lists permission rules for one project.
    ///
    /// # Errors
    /// Returns `NotFound` for an unknown project or an unavailable store.
    pub fn permission_rules(
        &self,
        project_id: &str,
    ) -> Result<Vec<PermissionRuleView>, SessionRuntimeError> {
        let id = ProjectId::new(project_id);
        let store = self.store()?;
        if store
            .load_project(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .is_none()
        {
            return Err(SessionRuntimeError::NotFound);
        }
        store
            .permission_rules(&id)
            .map(|rules| rules.into_iter().map(Into::into).collect())
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Deletes one permission rule from one project.
    ///
    /// # Errors
    /// Returns `NotFound` when the project or rule is absent.
    pub fn delete_permission_rule(
        &self,
        project_id: &str,
        rule_id: &str,
    ) -> Result<(), SessionRuntimeError> {
        let deleted = self
            .store()?
            .delete_permission_rule(&ProjectId::new(project_id), rule_id)
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        if deleted {
            Ok(())
        } else {
            Err(SessionRuntimeError::NotFound)
        }
    }

    /// Turns app-delivered approval waiting on or off.
    pub fn set_interactive_approvals(&self, enabled: bool) {
        self.broker.set_interactive(enabled);
        self.broker.questions().set_interactive(enabled);
    }
}

#[cfg(test)]
#[path = "approvals_tests.rs"]
mod tests;

impl SessionRuntime {
    pub(crate) fn runner(
        &self,
        session_id: &str,
    ) -> Result<Arc<crate::SessionRunner>, SessionRuntimeError> {
        self.runners
            .lock()
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .get(&SessionId::new(session_id))
            .cloned()
            .ok_or(SessionRuntimeError::NotFound)
    }
}
