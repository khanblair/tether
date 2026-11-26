//! Tests for the saved reasoning effort of a session.

use rusqlite::Connection;
use tether_types::{ModelRef, PermissionMode, Provider, ReasoningEffort, SessionId, SessionStatus};

use crate::migrations::{MIGRATIONS, apply};
use crate::sessions_tests::seed_project;
use crate::{Store, StoreError};

fn open_store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("tether.db")).unwrap();
    (dir, store)
}

fn new_session(store: &mut Store) -> SessionId {
    let project = seed_project(store);
    store
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "model".into(),
            },
            PermissionMode::Ask,
            10,
        )
        .unwrap()
        .id
}

#[test]
fn migration_fourteen_keeps_existing_sessions_without_an_effort() {
    let mut conn = Connection::open_in_memory().unwrap();
    apply(&mut conn, &MIGRATIONS[..13]).unwrap();
    conn.execute_batch(
        "INSERT INTO projects (id, name, path, created_at) VALUES ('p', 'p', 'p', 1);
         INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
         VALUES ('s', 'p', 'openai', 'm', 'ask', 'idle', 1, 1);",
    )
    .unwrap();
    assert_eq!(apply(&mut conn, MIGRATIONS).unwrap(), 14);
    let effort: Option<String> = conn
        .query_row(
            "SELECT reasoning_effort FROM sessions WHERE id = 's'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(effort, None);
}

#[test]
fn the_check_rejects_an_effort_outside_the_three_levels() {
    let mut conn = Connection::open_in_memory().unwrap();
    apply(&mut conn, MIGRATIONS).unwrap();
    conn.execute_batch(
        "INSERT INTO projects (id, name, path, created_at) VALUES ('p', 'p', 'p', 1);
         INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
         VALUES ('s', 'p', 'openai', 'm', 'ask', 'idle', 1, 1);",
    )
    .unwrap();
    let result = conn.execute(
        "UPDATE sessions SET reasoning_effort = 'extreme' WHERE id = 's'",
        [],
    );
    assert!(result.is_err());
}

#[test]
fn each_effort_and_none_survive_a_reopen() {
    let (dir, mut store) = open_store();
    let id = new_session(&mut store);
    let path = dir.path().join("tether.db");
    let cases = [
        Some(ReasoningEffort::Low),
        Some(ReasoningEffort::Medium),
        Some(ReasoningEffort::High),
        None,
    ];
    for (index, effort) in cases.into_iter().enumerate() {
        let now = 20 + i64::try_from(index).unwrap();
        store.set_session_reasoning(&id, effort, now).unwrap();
        drop(store);
        store = Store::open(&path).unwrap();
        let saved = store.load_session(&id).unwrap().unwrap();
        assert_eq!(saved.reasoning_effort, effort);
    }
}

#[test]
fn a_new_session_starts_with_no_effort() {
    let (_dir, mut store) = open_store();
    let id = new_session(&mut store);
    let saved = store.load_session(&id).unwrap().unwrap();
    assert_eq!(saved.reasoning_effort, None);
}

#[test]
fn a_busy_session_refuses_a_new_effort() {
    let (_dir, mut store) = open_store();
    let id = new_session(&mut store);
    store
        .set_session_reasoning(&id, Some(ReasoningEffort::Low), 20)
        .unwrap();
    store
        .set_session_status(&id, SessionStatus::Streaming, 21)
        .unwrap();
    let result = store.set_session_reasoning(&id, Some(ReasoningEffort::High), 22);
    assert!(matches!(result, Err(StoreError::SessionBusy)));
    store
        .set_session_status(&id, SessionStatus::Idle, 23)
        .unwrap();
    let saved = store.load_session(&id).unwrap().unwrap();
    assert_eq!(saved.reasoning_effort, Some(ReasoningEffort::Low));
}

#[test]
fn a_missing_session_refuses_a_new_effort() {
    let (_dir, store) = open_store();
    let missing = SessionId::new("01MISSINGSESSION0000000000");
    let result = store.set_session_reasoning(&missing, Some(ReasoningEffort::Medium), 1);
    assert!(matches!(result, Err(StoreError::MissingSessionRow)));
}

#[test]
fn setting_an_effort_bumps_the_session_activity_time() {
    let (_dir, mut store) = open_store();
    let id = new_session(&mut store);
    store
        .set_session_reasoning(&id, Some(ReasoningEffort::High), 5)
        .unwrap();
    let saved = store.load_session(&id).unwrap().unwrap();
    assert_eq!(saved.updated_at, 10 + 1);
}
