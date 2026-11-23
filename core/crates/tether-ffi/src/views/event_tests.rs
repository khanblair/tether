use tether_types::{
    ConnectionStatus, CoreError, CoreEvent, MessageId, Plan, Provider, SessionId, SessionStatus,
    TodoItem, TodoStatus, ToolCallId, ToolCallStatus, UsageSnapshot,
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
fn quality_report_event_keeps_every_field() {
    let event = CoreEvent::QualityReport {
        session_id: sid(),
        tool_call_id: cid(),
        counts: tether_types::QualityCounts {
            info: 1,
            warn: 2,
            block: 3,
        },
        findings: vec![tether_types::QualityFindingRecord {
            rule_id: "smell.function_length".into(),
            severity: tether_types::QualitySeverity::Block,
            path: "src/lib.rs".into(),
            line: 8,
            message: "too long".into(),
            advice: "split it".into(),
        }],
        skipped: 4,
    };

    assert_eq!(
        CoreEventView::from(event),
        CoreEventView::QualityReport {
            session_id: "s1".into(),
            tool_call_id: "c1".into(),
            counts: QualityCountsView {
                info: 1,
                warn: 2,
                block: 3,
            },
            findings: vec![QualityFindingRecordView {
                rule_id: "smell.function_length".into(),
                severity: "block".into(),
                path: "src/lib.rs".into(),
                line: 8,
                message: "too long".into(),
                advice: "split it".into(),
            }],
            skipped: 4,
        }
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
