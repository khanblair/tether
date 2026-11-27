//! Conversion from the core event into the FFI event view.

use tether_types::CoreEvent;

use super::CoreEventView;

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
            CoreEvent::QualityReport {
                session_id,
                tool_call_id,
                counts,
                findings,
                skipped,
            } => Self::QualityReport {
                session_id: session_id.into(),
                tool_call_id: tool_call_id.into(),
                counts: counts.into(),
                findings: findings.into_iter().map(Into::into).collect(),
                skipped,
            },
            CoreEvent::ProcessChanged {
                session_id,
                process_id,
                status,
                port,
            } => Self::ProcessChanged {
                session_id: session_id.into(),
                process_id,
                status: status.into(),
                port,
            },
            CoreEvent::PreviewRequested { session_id, target } => Self::PreviewRequested {
                session_id: session_id.into(),
                target: target.into(),
            },
        }
    }
}
