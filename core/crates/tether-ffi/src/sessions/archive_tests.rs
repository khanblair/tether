//! Tests for archiving, deleting, and listing archived sessions through the runtime.

use tether_store::Store;
use tether_types::SessionStatus;

use super::tests::ux_runtime;
use crate::SessionRuntimeError;

fn ids(sessions: &[crate::SessionRecordView]) -> Vec<String> {
    sessions.iter().map(|s| s.id.clone()).collect()
}

#[test]
fn archive_hides_a_session_and_restore_brings_it_back() {
    let (_dir, _db, runtime, session, project) = ux_runtime();
    let other = runtime
        .create_session(project.as_str(), crate::ProviderView::Anthropic, "model")
        .unwrap();
    runtime.archive_session(session.as_str(), true).unwrap();
    assert_eq!(
        ids(&runtime.list_sessions(project.as_str().to_owned()).unwrap()),
        vec![other.id.clone()]
    );
    assert_eq!(
        ids(&runtime.list_archived_sessions(project.as_str()).unwrap()),
        [session.as_str().to_owned()]
    );
    runtime.archive_session(session.as_str(), false).unwrap();
    assert!(
        runtime
            .list_archived_sessions(project.as_str())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        runtime
            .list_sessions(project.as_str().to_owned())
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn archive_and_delete_report_busy_and_missing_sessions() {
    let (_dir, db, runtime, session, project) = ux_runtime();
    assert!(matches!(
        runtime.archive_session("missing", true),
        Err(SessionRuntimeError::NotFound)
    ));
    assert!(matches!(
        runtime.delete_session("missing"),
        Err(SessionRuntimeError::NotFound)
    ));
    assert!(matches!(
        runtime.list_archived_sessions("missing"),
        Err(SessionRuntimeError::NotFound)
    ));
    Store::open(&db)
        .unwrap()
        .set_session_status(&session, SessionStatus::Streaming, 50)
        .unwrap();
    assert!(matches!(
        runtime.archive_session(session.as_str(), true),
        Err(SessionRuntimeError::Busy)
    ));
    assert!(matches!(
        runtime.delete_session(session.as_str()),
        Err(SessionRuntimeError::Busy)
    ));
    assert!(runtime.load_session(session.as_str()).is_ok());
    assert!(
        runtime
            .list_archived_sessions(project.as_str())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn delete_removes_the_session_and_its_archive_entry() {
    let (_dir, _db, runtime, session, project) = ux_runtime();
    runtime.archive_session(session.as_str(), true).unwrap();
    runtime.delete_session(session.as_str()).unwrap();
    assert!(matches!(
        runtime.load_session(session.as_str()),
        Err(SessionRuntimeError::NotFound)
    ));
    assert!(
        runtime
            .list_archived_sessions(project.as_str())
            .unwrap()
            .is_empty()
    );
}
