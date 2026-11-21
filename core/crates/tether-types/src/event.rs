//! Events the core sends to the platform layer.

use crate::{
    ConnectionStatus, CoreError, MessageId, Plan, Provider, SessionId, SessionStatus, TodoItem,
    ToolCallId, ToolCallStatus, UsageSnapshot,
};

/// The limit scope that produced a budget event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetScope {
    /// The current session's limit.
    Session,
    /// The selected UTC day's limit.
    Day,
}

/// One event emitted by the core. Changing this enum changes the FFI surface;
/// regenerate the Kotlin bindings with `just bindings`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreEvent {
    /// The core finished starting and reports its version.
    Ready {
        /// Core version, equal to the app version.
        version: String,
    },
    /// A test event used by P0 to prove a Rust event reaches a Compose screen.
    Test {
        /// Free text chosen by the caller.
        message: String,
    },
    /// The session moved to a new state.
    SessionUpdated {
        /// The session that changed.
        session_id: SessionId,
        /// Its new state.
        status: SessionStatus,
    },
    /// A saved checkpoint restored files, conversation, or both.
    SessionRewound {
        /// The session whose state changed.
        session_id: SessionId,
    },
    /// A batch of reply text, sent at most every 32 ms or 64 characters.
    TextDelta {
        /// The session the text belongs to.
        session_id: SessionId,
        /// The assistant message being written.
        message_id: MessageId,
        /// The new text.
        text: String,
    },
    /// A batch of model reasoning.
    ThinkingDelta {
        /// The session the text belongs to.
        session_id: SessionId,
        /// The assistant message being written.
        message_id: MessageId,
        /// The new text.
        text: String,
    },
    /// The model proposed a tool call.
    ToolProposed {
        /// The session that owns the call.
        session_id: SessionId,
        /// The proposed call.
        tool_call_id: ToolCallId,
        /// Tool name.
        tool: String,
        /// The exact command, path, or URL the tool will touch.
        target: String,
    },
    /// A tool call waits for the user to allow or deny it.
    ApprovalRequired {
        /// The session that owns the call.
        session_id: SessionId,
        /// The call that needs approval.
        tool_call_id: ToolCallId,
        /// Tool name.
        tool: String,
        /// The exact command, path, or URL the tool will touch.
        target: String,
    },
    /// A tool call started running.
    ToolStarted {
        /// The session that owns the call.
        session_id: SessionId,
        /// The call that started.
        tool_call_id: ToolCallId,
    },
    /// A chunk of output from a running tool call.
    ToolOutput {
        /// The session that owns the call.
        session_id: SessionId,
        /// The call that produced the output.
        tool_call_id: ToolCallId,
        /// The new output text.
        chunk: String,
    },
    /// A tool call reached a final state.
    ToolFinished {
        /// The session that owns the call.
        session_id: SessionId,
        /// The call that finished.
        tool_call_id: ToolCallId,
        /// How it ended.
        status: ToolCallStatus,
    },
    /// The session checklist changed.
    TodoUpdated {
        /// The session whose checklist changed.
        session_id: SessionId,
        /// The complete replacement checklist.
        items: Vec<TodoItem>,
    },
    /// The model asked the user a question.
    QuestionAsked {
        /// The session that owns the question.
        session_id: SessionId,
        /// The call waiting for an answer.
        tool_call_id: ToolCallId,
        /// The question text.
        question: String,
        /// Optional tappable answers.
        options: Vec<String>,
        /// Whether the user may type an answer.
        allow_free_text: bool,
    },
    /// The model submitted a plan for review.
    PlanReady {
        /// The session that owns the plan.
        session_id: SessionId,
        /// The call that submitted the plan.
        tool_call_id: ToolCallId,
        /// The proposed plan.
        plan: Plan,
    },
    /// Session totals changed.
    UsageUpdated {
        /// The session the totals belong to.
        session_id: SessionId,
        /// The new totals.
        usage: UsageSnapshot,
    },
    /// The turn ended normally.
    TurnFinished {
        /// The session whose turn ended.
        session_id: SessionId,
    },
    /// The user or the system stopped the turn.
    TurnInterrupted {
        /// The session whose turn stopped.
        session_id: SessionId,
    },
    /// The turn ended with an error.
    TurnFailed {
        /// The session whose turn failed.
        session_id: SessionId,
        /// What went wrong.
        error: CoreError,
    },
    /// The network went down and the session paused.
    NetworkPaused {
        /// The session that paused.
        session_id: SessionId,
    },
    /// The network came back and the session resumed.
    NetworkResumed {
        /// The session that resumed.
        session_id: SessionId,
    },
    /// A provider connection changed state.
    ProviderStatus {
        /// The provider that changed.
        provider: Provider,
        /// Its new state.
        status: ConnectionStatus,
    },
    /// A budget crossed its warning threshold.
    BudgetWarning {
        /// The session about to make a request.
        session_id: SessionId,
        /// The limit that was crossed.
        scope: BudgetScope,
        /// Integer percentage of the limit already spent.
        percent: u8,
        /// Amount spent in USD micros.
        spent_micros: u64,
        /// Configured limit in USD micros.
        limit_micros: u64,
    },
    /// A budget reached its limit and the session is paused.
    BudgetReached {
        /// The session that was paused.
        session_id: SessionId,
        /// The limit that was reached.
        scope: BudgetScope,
        /// Amount spent in USD micros.
        spent_micros: u64,
        /// Configured limit in USD micros.
        limit_micros: u64,
    },
    /// A context range was replaced by a saved summary.
    ContextCompacted {
        /// The session that was compacted.
        session_id: SessionId,
        /// The saved summary message.
        summary_message_id: MessageId,
        /// Estimated or reported tokens before compaction.
        tokens_before: u32,
        /// Estimated tokens after compaction.
        tokens_after: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::CoreEvent;

    #[test]
    fn events_compare_by_value() {
        let a = CoreEvent::Test {
            message: "hi".into(),
        };
        assert_eq!(
            a,
            CoreEvent::Test {
                message: "hi".into()
            }
        );
        assert_ne!(
            a,
            CoreEvent::Ready {
                version: "0.0.1".into()
            }
        );
    }
}
