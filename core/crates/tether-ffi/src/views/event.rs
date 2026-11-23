//! The event view the platform layer receives.

use super::{
    BudgetScopeView, ConnectionStatusView, CoreErrorView, PlanView, ProviderView,
    SessionStatusView, TodoItemView, ToolCallStatusView, UsageSnapshotView,
};

#[path = "event_from.rs"]
mod event_from;

/// FFI view of quality finding counts.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct QualityCountsView {
    /// Informational findings.
    pub info: u32,
    /// Warning findings.
    pub warn: u32,
    /// Blocking findings.
    pub block: u32,
}

/// FFI view of one quality finding.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct QualityFindingRecordView {
    /// Stable rule identifier.
    pub rule_id: String,
    /// Finding severity.
    pub severity: String,
    /// Repository-relative path.
    pub path: String,
    /// First affected line.
    pub line: u32,
    /// Short explanation.
    pub message: String,
    /// Suggested action.
    pub advice: String,
}

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
    /// A quality gate report for one edit batch.
    QualityReport {
        /// Session id.
        session_id: String,
        /// Last tool call id in the batch.
        tool_call_id: String,
        /// Finding counts.
        counts: QualityCountsView,
        /// Findings for the diff card.
        findings: Vec<QualityFindingRecordView>,
        /// Number of skipped changed files.
        skipped: u32,
    },
}

impl From<tether_types::QualityCounts> for QualityCountsView {
    fn from(counts: tether_types::QualityCounts) -> Self {
        Self {
            info: counts.info,
            warn: counts.warn,
            block: counts.block,
        }
    }
}

impl From<tether_types::QualityFindingRecord> for QualityFindingRecordView {
    fn from(finding: tether_types::QualityFindingRecord) -> Self {
        Self {
            rule_id: finding.rule_id,
            severity: match finding.severity {
                tether_types::QualitySeverity::Info => "info",
                tether_types::QualitySeverity::Warn => "warn",
                tether_types::QualitySeverity::Block => "block",
            }
            .into(),
            path: finding.path,
            line: finding.line,
            message: finding.message,
            advice: finding.advice,
        }
    }
}

#[cfg(test)]
#[path = "event_tests.rs"]
mod tests;
