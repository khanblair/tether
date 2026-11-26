//! Tests for setting, clearing, and reading the saved reasoning effort through the runtime.

use tether_store::Store;
use tether_types::SessionStatus;

use super::tests::ux_runtime;
use crate::{ReasoningEffortView, SessionRuntimeError};

#[test]
fn reasoning_effort_is_set_read_back_and_cleared() {
    let (_dir, db, runtime, session, _project) = ux_runtime();
    assert_eq!(
        runtime
            .load_session(session.as_str())
            .unwrap()
            .reasoning_effort,
        None
    );
    runtime
        .set_session_reasoning(session.as_str(), Some(ReasoningEffortView::High))
        .unwrap();
    let reopened = Store::open(&db)
        .unwrap()
        .load_session(&session)
        .unwrap()
        .unwrap();
    assert_eq!(
        crate::SessionRecordView::from(reopened).reasoning_effort,
        Some(ReasoningEffortView::High)
    );
    runtime
        .set_session_reasoning(session.as_str(), None)
        .unwrap();
    assert_eq!(
        runtime
            .load_session(session.as_str())
            .unwrap()
            .reasoning_effort,
        None
    );
}

#[test]
fn reasoning_effort_reports_busy_and_missing_sessions() {
    let (_dir, db, runtime, session, _project) = ux_runtime();
    assert!(matches!(
        runtime.set_session_reasoning("missing", Some(ReasoningEffortView::Low)),
        Err(SessionRuntimeError::NotFound)
    ));
    Store::open(&db)
        .unwrap()
        .set_session_status(&session, SessionStatus::Streaming, 50)
        .unwrap();
    assert!(matches!(
        runtime.set_session_reasoning(session.as_str(), Some(ReasoningEffortView::Medium)),
        Err(SessionRuntimeError::Busy)
    ));
    assert_eq!(
        runtime
            .load_session(session.as_str())
            .unwrap()
            .reasoning_effort,
        None
    );
}
