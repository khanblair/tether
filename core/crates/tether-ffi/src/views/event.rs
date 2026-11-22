//! The event view the platform layer receives.

use tether_types::CoreEvent;

use super::{
    BudgetScopeView, ConnectionStatusView, CoreErrorView, PlanView, ProviderView,
    SessionStatusView, TodoItemView, ToolCallStatusView, UsageSnapshotView,
};

/// FFI view of [`CoreEvent`]. Ids are plain strings.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum CoreEventView {
    /// The core finished starting.
    Ready {
        /// Core version.
        version: String,
    },
    /// A test event.
    Test {
        /// Free text chosen by the caller.
        message: String,
    },
    /// The session moved to a new state.
    SessionUpdated {
        /// The session that changed.
        session_id: String,
        /// Its new state.
        status: SessionStatusView,
    },
    /// A checkpoint restored part or all of a session.
    SessionRewound {
        /// The session whose state changed.
        session_id: String,
    },
    /// A batch of reply text.
    TextDelta {
        /// The session the text belongs to.
        session_id: String,
        /// The assistant message being written.
        message_id: String,
        /// The new text.
        text: String,
    },
    /// A batch of model reasoning.
    ThinkingDelta {
        /// The session the text belongs to.
        session_id: String,
        /// The assistant message being written.
        message_id: String,
        /// The new text.
        text: String,
    },
    /// The model proposed a tool call.
    ToolProposed {
        /// The session that owns the call.
        session_id: String,
        /// The proposed call.
        tool_call_id: String,
        /// Tool name.
        tool: String,
        /// The exact command, path, or URL the tool will touch.
        target: String,
    },
    /// A tool call waits for the user to allow or deny it.
    ApprovalRequired {
        /// The session that owns the call.
        session_id: String,
        /// The call that needs approval.
        tool_call_id: String,
        /// Tool name.
        tool: String,
        /// The exact command, path, or URL the tool will touch.
        target: String,
    },
    /// A tool call started running.
    ToolStarted {
        /// The session that owns the call.
        session_id: String,
        /// The call that started.
        tool_call_id: String,
    },
    /// A chunk of output from a running tool call.
    ToolOutput {
        /// The session that owns the call.
        session_id: String,
        /// The call that produced the output.
        tool_call_id: String,
        /// The new output text.
        chunk: String,
    },
    /// A tool call reached a final state.
    ToolFinished {
        /// The session that owns the call.
        session_id: String,
        /// The call that finished.
        tool_call_id: String,
        /// How it ended.
        status: ToolCallStatusView,
    },
    /// The session checklist changed.
    TodoUpdated {
        /// Session id.
        session_id: String,
        /// Complete checklist.
        items: Vec<TodoItemView>,
    },
    /// The model asked the user a question.
    QuestionAsked {
        /// Session id.
        session_id: String,
        /// Tool call id.
        tool_call_id: String,
        /// Question text.
        question: String,
        /// Answer options.
        options: Vec<String>,
        /// Whether typed answers are accepted.
        allow_free_text: bool,
    },
    /// The model submitted a plan.
    PlanReady {
        /// Session id.
        session_id: String,
        /// Tool call id.
        tool_call_id: String,
        /// Submitted plan.
        plan: PlanView,
    },
    /// Session totals changed.
    UsageUpdated {
        /// The session the totals belong to.
        session_id: String,
        /// The new totals.
        usage: UsageSnapshotView,
    },
    /// The turn ended normally.
    TurnFinished {
        /// The session whose turn ended.
        session_id: String,
    },
    /// The user or the system stopped the turn.
    TurnInterrupted {
        /// The session whose turn stopped.
        session_id: String,
    },
    /// The turn ended with an error.
    TurnFailed {
        /// The session whose turn failed.
        session_id: String,
        /// What went wrong.
        error: CoreErrorView,
    },
    /// The network went down and the session paused.
    NetworkPaused {
        /// The session that paused.
        session_id: String,
    },
    /// The network came back and the session resumed.
    NetworkResumed {
        /// The session that resumed.
        session_id: String,
    },
    /// A provider connection changed state.
    ProviderStatus {
        /// The provider that changed.
        provider: ProviderView,
        /// Its new state.
        status: ConnectionStatusView,
    },
    /// A budget crossed its warning threshold.
    BudgetWarning {
        /// Session id.
        session_id: String,
        /// Limit scope.
        scope: BudgetScopeView,
        /// Integer percentage already spent.
        percent: u8,
        /// Amount spent in USD micros.
        spent_micros: u64,
        /// Configured limit in USD micros.
        limit_micros: u64,
    },
    /// A budget reached its limit.
    BudgetReached {
        /// Session id.
        session_id: String,
        /// Limit scope.
        scope: BudgetScopeView,
        /// Amount spent in USD micros.
        spent_micros: u64,
        /// Configured limit in USD micros.
        limit_micros: u64,
    },
    /// A context range was replaced by a saved summary.
    ContextCompacted {
        /// Session id.
        session_id: String,
        /// Summary message id.
        summary_message_id: String,
        /// Tokens before compaction.
        tokens_before: u32,
        /// Tokens after compaction.
        tokens_after: u32,
    },
}

impl From<CoreEvent> for CoreEventView {
    #[expect(
        clippy::too_many_lines,
        reason = "one flat arm per event; splitting the match would need unreachable arms"
    )]
    fn from(event: CoreEvent) -> Self {
        match event {
            CoreEvent::Ready { version } => Self::Ready { version },
            CoreEvent::Test { message } => Self::Test { message },
            CoreEvent::SessionUpdated { session_id, status } => Self::SessionUpdated {
                session_id: session_id.into(),
                status: status.into(),
            },
            CoreEvent::SessionRewound { session_id } => Self::SessionRewound {
                session_id: session_id.into(),
            },
            CoreEvent::TextDelta {
                session_id,
                message_id,
                text,
            } => Self::TextDelta {
                session_id: session_id.into(),
                message_id: message_id.into(),
                text,
            },
            CoreEvent::ThinkingDelta {
                session_id,
                message_id,
                text,
            } => Self::ThinkingDelta {
                session_id: session_id.into(),
                message_id: message_id.into(),
                text,
            },
            CoreEvent::ToolProposed {
                session_id,
                tool_call_id,
                tool,
                target,
            } => Self::ToolProposed {
                session_id: session_id.into(),
                tool_call_id: tool_call_id.into(),
                tool,
                target,
            },
            CoreEvent::ApprovalRequired {
                session_id,
                tool_call_id,
                tool,
                target,
            } => Self::ApprovalRequired {
                session_id: session_id.into(),
                tool_call_id: tool_call_id.into(),
                tool,
                target,
            },
            CoreEvent::ToolStarted {
                session_id,
                tool_call_id,
            } => Self::ToolStarted {
                session_id: session_id.into(),
                tool_call_id: tool_call_id.into(),
            },
            CoreEvent::ToolOutput {
                session_id,
                tool_call_id,
                chunk,
            } => Self::ToolOutput {
                session_id: session_id.into(),
                tool_call_id: tool_call_id.into(),
                chunk,
            },
            CoreEvent::ToolFinished {
                session_id,
                tool_call_id,
                status,
            } => Self::ToolFinished {
                session_id: session_id.into(),
                tool_call_id: tool_call_id.into(),
                status: status.into(),
            },
            CoreEvent::TodoUpdated { session_id, items } => Self::TodoUpdated {
                session_id: session_id.into(),
                items: items.into_iter().map(Into::into).collect(),
            },
            CoreEvent::QuestionAsked {
                session_id,
                tool_call_id,
                question,
                options,
                allow_free_text,
            } => Self::QuestionAsked {
                session_id: session_id.into(),
                tool_call_id: tool_call_id.into(),
                question,
                options,
                allow_free_text,
            },
            CoreEvent::PlanReady {
                session_id,
                tool_call_id,
                plan,
            } => Self::PlanReady {
                session_id: session_id.into(),
                tool_call_id: tool_call_id.into(),
                plan: plan.into(),
            },
            CoreEvent::UsageUpdated { session_id, usage } => Self::UsageUpdated {
                session_id: session_id.into(),
                usage: usage.into(),
            },
            CoreEvent::TurnFinished { session_id } => Self::TurnFinished {
                session_id: session_id.into(),
            },
            CoreEvent::TurnInterrupted { session_id } => Self::TurnInterrupted {
                session_id: session_id.into(),
            },
            CoreEvent::TurnFailed { session_id, error } => Self::TurnFailed {
                session_id: session_id.into(),
                error: error.into(),
            },
            CoreEvent::NetworkPaused { session_id } => Self::NetworkPaused {
                session_id: session_id.into(),
            },
            CoreEvent::NetworkResumed { session_id } => Self::NetworkResumed {
                session_id: session_id.into(),
            },
            CoreEvent::ProviderStatus { provider, status } => Self::ProviderStatus {
                provider: provider.into(),
                status: status.into(),
            },
            CoreEvent::BudgetWarning {
                session_id,
                scope,
                percent,
                spent_micros,
                limit_micros,
            } => Self::BudgetWarning {
                session_id: session_id.into(),
                scope: scope.into(),
                percent,
                spent_micros,
                limit_micros,
            },
            CoreEvent::BudgetReached {
                session_id,
                scope,
                spent_micros,
                limit_micros,
            } => Self::BudgetReached {
                session_id: session_id.into(),
                scope: scope.into(),
                spent_micros,
                limit_micros,
            },
            CoreEvent::ContextCompacted {
                session_id,
                summary_message_id,
                tokens_before,
                tokens_after,
            } => Self::ContextCompacted {
                session_id: session_id.into(),
                summary_message_id: summary_message_id.into(),
                tokens_before,
                tokens_after,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use tether_types::{
        ConnectionStatus, CoreError, MessageId, Plan, Provider, SessionId, SessionStatus, TodoItem,
        TodoStatus, ToolCallId, ToolCallStatus, UsageSnapshot,
    };

    use super::*;
    use crate::views::ErrorKindView;

    fn sid() -> SessionId {
        SessionId::new("s1")
    }

    fn cid() -> ToolCallId {
        ToolCallId::new("c1")
    }

    #[test]
    fn lifecycle_and_status_events_convert() {
        let pairs = [
            (
                CoreEvent::Ready {
                    version: "0.0.1".into(),
                },
                CoreEventView::Ready {
                    version: "0.0.1".into(),
                },
            ),
            (
                CoreEvent::Test {
                    message: "m".into(),
                },
                CoreEventView::Test {
                    message: "m".into(),
                },
            ),
            (
                CoreEvent::SessionUpdated {
                    session_id: sid(),
                    status: SessionStatus::AwaitingApproval,
                },
                CoreEventView::SessionUpdated {
                    session_id: "s1".into(),
                    status: SessionStatusView::AwaitingApproval,
                },
            ),
            (
                CoreEvent::ProviderStatus {
                    provider: Provider::DeepSeek,
                    status: ConnectionStatus::BadKey,
                },
                CoreEventView::ProviderStatus {
                    provider: ProviderView::DeepSeek,
                    status: ConnectionStatusView::BadKey,
                },
            ),
        ];
        for (event, view) in pairs {
            assert_eq!(CoreEventView::from(event), view);
        }
    }

    #[test]
    fn usage_event_converts_its_totals() {
        let pairs = [(
            CoreEvent::UsageUpdated {
                session_id: sid(),
                usage: UsageSnapshot {
                    tokens_in: 1,
                    tokens_out: 2,
                    cost_micros: 3,
                },
            },
            CoreEventView::UsageUpdated {
                session_id: "s1".into(),
                usage: UsageSnapshotView {
                    tokens_in: 1,
                    tokens_out: 2,
                    cost_micros: 3,
                },
            },
        )];
        for (event, view) in pairs {
            assert_eq!(CoreEventView::from(event), view);
        }
    }

    #[test]
    fn text_events_keep_session_message_and_text_apart() {
        let text = CoreEventView::from(CoreEvent::TextDelta {
            session_id: sid(),
            message_id: MessageId::new("m1"),
            text: "hello".into(),
        });
        let thinking = CoreEventView::from(CoreEvent::ThinkingDelta {
            session_id: sid(),
            message_id: MessageId::new("m1"),
            text: "hmm".into(),
        });
        assert_eq!(
            text,
            CoreEventView::TextDelta {
                session_id: "s1".into(),
                message_id: "m1".into(),
                text: "hello".into()
            }
        );
        assert_eq!(
            thinking,
            CoreEventView::ThinkingDelta {
                session_id: "s1".into(),
                message_id: "m1".into(),
                text: "hmm".into()
            }
        );
    }

    #[test]
    fn tool_request_events_convert() {
        let pairs = [
            (
                CoreEvent::ToolProposed {
                    session_id: sid(),
                    tool_call_id: cid(),
                    tool: "run".into(),
                    target: "pytest".into(),
                },
                CoreEventView::ToolProposed {
                    session_id: "s1".into(),
                    tool_call_id: "c1".into(),
                    tool: "run".into(),
                    target: "pytest".into(),
                },
            ),
            (
                CoreEvent::ApprovalRequired {
                    session_id: sid(),
                    tool_call_id: cid(),
                    tool: "edit_file".into(),
                    target: "src/a.rs".into(),
                },
                CoreEventView::ApprovalRequired {
                    session_id: "s1".into(),
                    tool_call_id: "c1".into(),
                    tool: "edit_file".into(),
                    target: "src/a.rs".into(),
                },
            ),
        ];
        for (event, view) in pairs {
            assert_eq!(CoreEventView::from(event), view);
        }
    }

    #[test]
    fn tool_progress_events_convert() {
        let pairs = [
            (
                CoreEvent::ToolStarted {
                    session_id: sid(),
                    tool_call_id: cid(),
                },
                CoreEventView::ToolStarted {
                    session_id: "s1".into(),
                    tool_call_id: "c1".into(),
                },
            ),
            (
                CoreEvent::ToolOutput {
                    session_id: sid(),
                    tool_call_id: cid(),
                    chunk: "ok".into(),
                },
                CoreEventView::ToolOutput {
                    session_id: "s1".into(),
                    tool_call_id: "c1".into(),
                    chunk: "ok".into(),
                },
            ),
            (
                CoreEvent::ToolFinished {
                    session_id: sid(),
                    tool_call_id: cid(),
                    status: ToolCallStatus::Succeeded,
                },
                CoreEventView::ToolFinished {
                    session_id: "s1".into(),
                    tool_call_id: "c1".into(),
                    status: ToolCallStatusView::Succeeded,
                },
            ),
        ];
        for (event, view) in pairs {
            assert_eq!(CoreEventView::from(event), view);
        }
    }

    #[test]
    fn turn_and_network_events_convert() {
        let failed = CoreEventView::from(CoreEvent::TurnFailed {
            session_id: sid(),
            error: CoreError::RateLimited {
                message: "slow down".into(),
            },
        });
        assert_eq!(
            failed,
            CoreEventView::TurnFailed {
                session_id: "s1".into(),
                error: CoreErrorView {
                    kind: ErrorKindView::RateLimited,
                    message: "slow down".into(),
                    retryable: true
                }
            }
        );
        let session_only = [
            CoreEvent::TurnFinished { session_id: sid() },
            CoreEvent::TurnInterrupted { session_id: sid() },
            CoreEvent::NetworkPaused { session_id: sid() },
            CoreEvent::NetworkResumed { session_id: sid() },
        ];
        let views: Vec<CoreEventView> = session_only.into_iter().map(Into::into).collect();
        assert_eq!(
            views,
            vec![
                CoreEventView::TurnFinished {
                    session_id: "s1".into()
                },
                CoreEventView::TurnInterrupted {
                    session_id: "s1".into()
                },
                CoreEventView::NetworkPaused {
                    session_id: "s1".into()
                },
                CoreEventView::NetworkResumed {
                    session_id: "s1".into()
                },
            ]
        );
    }

    #[test]
    fn session_control_events_convert_every_field() {
        let todo = CoreEventView::from(CoreEvent::TodoUpdated {
            session_id: sid(),
            items: vec![TodoItem {
                text: "read".into(),
                status: TodoStatus::Completed,
            }],
        });
        assert_eq!(
            todo,
            CoreEventView::TodoUpdated {
                session_id: "s1".into(),
                items: vec![crate::views::TodoItemView {
                    text: "read".into(),
                    status: crate::views::TodoStatusView::Completed,
                }],
            }
        );
        let question = CoreEventView::from(CoreEvent::QuestionAsked {
            session_id: sid(),
            tool_call_id: cid(),
            question: "Pick".into(),
            options: vec!["A".into(), "B".into()],
            allow_free_text: false,
        });
        assert_eq!(
            question,
            CoreEventView::QuestionAsked {
                session_id: "s1".into(),
                tool_call_id: "c1".into(),
                question: "Pick".into(),
                options: vec!["A".into(), "B".into()],
                allow_free_text: false,
            }
        );
    }

    #[test]
    fn plan_event_converts_every_field() {
        let plan = CoreEventView::from(CoreEvent::PlanReady {
            session_id: sid(),
            tool_call_id: cid(),
            plan: Plan {
                summary: "Do it".into(),
                steps: vec!["one".into()],
                files: vec!["a.rs".into()],
                commands: vec!["check".into()],
                open_questions: vec!["ready?".into()],
            },
        });
        assert_eq!(
            plan,
            CoreEventView::PlanReady {
                session_id: "s1".into(),
                tool_call_id: "c1".into(),
                plan: crate::views::PlanView {
                    summary: "Do it".into(),
                    steps: vec!["one".into()],
                    files: vec!["a.rs".into()],
                    commands: vec!["check".into()],
                    open_questions: vec!["ready?".into()],
                },
            }
        );
    }
}
