//! Tests for archiving, restoring, deleting, and listing archived sessions.

use rusqlite::Connection;
use tether_types::{
    MessagePart, ModelRef, PermissionMode, ProjectId, Provider, Role, SessionId, SessionStatus,
};

use crate::migrations::{MIGRATIONS, apply};
use crate::sessions_tests::seed_project;
use crate::{Store, StoreError};

fn open_store() -> (tempfile::TempDir, Store, ProjectId) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("tether.db")).unwrap();
    let project = seed_project(&store);
    (dir, store, project)
}

fn new_session(store: &mut Store, project: &ProjectId, now_ms: i64) -> SessionId {
    store
        .create_session(
            project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "model".into(),
            },
            PermissionMode::Ask,
            now_ms,
        )
        .unwrap()
        .id
}

fn say(store: &mut Store, session: &SessionId, text: &str) {
    store
        .append_message(
            session,
            Role::User,
            &[MessagePart::Text { text: text.into() }],
            5,
        )
        .unwrap();
}

fn ids(sessions: &[crate::SessionRecord]) -> Vec<String> {
    sessions.iter().map(|s| s.id.as_str().to_owned()).collect()
}

fn count(store: &Store, sql: &str, session: &SessionId) -> i64 {
    store
        .conn
        .query_row(sql, [session.as_str()], |row| row.get(0))
        .unwrap()
}

#[test]
fn archived_sessions_leave_the_lists_and_search_and_show_in_the_archive() {
    let (_dir, mut store, project) = open_store();
    let kept = new_session(&mut store, &project, 10);
    let hidden = new_session(&mut store, &project, 20);
    say(&mut store, &kept, "needle here");
    say(&mut store, &hidden, "needle there");
    store.set_session_archived(&hidden, true).unwrap();

    assert_eq!(
        ids(&store.list_sessions(&project).unwrap()),
        [kept.as_str()]
    );
    let found = store.search_sessions(&project, "needle").unwrap();
    assert_eq!(ids(&found), [kept.as_str()]);
    let hits = store.session_search("needle", 10).unwrap();
    assert!(hits.iter().all(|hit| hit.session_id != hidden));
    assert_eq!(hits.len(), 1);
    let archived = store.list_archived_sessions(&project).unwrap();
    assert_eq!(ids(&archived), [hidden.as_str()]);
    assert!(store.load_session(&hidden).unwrap().is_some());
}

#[test]
fn restoring_a_session_puts_it_back_in_its_old_place_and_archiving_keeps_activity_time() {
    let (_dir, mut store, project) = open_store();
    let first = new_session(&mut store, &project, 10);
    let middle = new_session(&mut store, &project, 20);
    let last = new_session(&mut store, &project, 30);
    let order = ids(&store.list_sessions(&project).unwrap());
    store.set_session_archived(&middle, true).unwrap();
    assert_eq!(store.load_session(&middle).unwrap().unwrap().updated_at, 20);
    store.set_session_archived(&middle, false).unwrap();
    assert_eq!(ids(&store.list_sessions(&project).unwrap()), order);
    assert_eq!(
        ids(&store.list_archived_sessions(&project).unwrap()),
        Vec::<String>::new()
    );
    assert_eq!(order, [last.as_str(), middle.as_str(), first.as_str()]);
}

#[test]
fn an_active_session_is_refused_for_archive_and_delete() {
    let (_dir, mut store, project) = open_store();
    let session = new_session(&mut store, &project, 10);
    store
        .set_session_status(&session, SessionStatus::Streaming, 11)
        .unwrap();
    assert!(matches!(
        store.set_session_archived(&session, true),
        Err(StoreError::SessionBusy)
    ));
    assert!(matches!(
        store.delete_session(&session),
        Err(StoreError::SessionBusy)
    ));
    assert!(store.load_session(&session).unwrap().is_some());
    assert_eq!(
        ids(&store.list_archived_sessions(&project).unwrap()),
        Vec::<String>::new()
    );
}

#[test]
fn deleting_a_session_removes_its_rows_and_leaves_another_session_alone() {
    let (_dir, mut store, project) = open_store();
    let doomed = new_session(&mut store, &project, 10);
    let other = new_session(&mut store, &project, 20);
    say(&mut store, &doomed, "gone words");
    say(&mut store, &other, "kept words");
    for session in [&doomed, &other] {
        store
            .conn
            .execute_batch(&format!(
                "INSERT INTO checkpoints (id, session_id, created_at) VALUES ('c-{d}', '{d}', 1);
                 INSERT INTO session_todos (session_id, position, text, status, updated_at)
                 VALUES ('{d}', 0, 'todo', 'pending', 1);",
                d = session.as_str()
            ))
            .unwrap();
    }
    let rows = [
        "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
        "SELECT COUNT(*) FROM message_parts p JOIN messages m ON m.id = p.message_id WHERE m.session_id = ?1",
        "SELECT COUNT(*) FROM checkpoints WHERE session_id = ?1",
        "SELECT COUNT(*) FROM session_todos WHERE session_id = ?1",
        "SELECT COUNT(*) FROM message_fts WHERE session_id = ?1",
    ];
    for sql in rows {
        assert_eq!(count(&store, sql, &doomed), 1, "{sql}");
    }
    store.delete_session(&doomed).unwrap();
    assert!(store.load_session(&doomed).unwrap().is_none());
    for sql in rows {
        assert_eq!(count(&store, sql, &doomed), 0, "{sql}");
        assert_eq!(count(&store, sql, &other), 1, "{sql}");
    }
    assert_eq!(
        ids(&store.list_sessions(&project).unwrap()),
        [other.as_str()]
    );
}

#[test]
fn deleting_or_archiving_a_missing_session_is_not_found() {
    let (_dir, store, _project) = open_store();
    let missing = SessionId::new("missing");
    assert!(matches!(
        store.delete_session(&missing),
        Err(StoreError::MissingSessionRow)
    ));
    assert!(matches!(
        store.set_session_archived(&missing, true),
        Err(StoreError::MissingSessionRow)
    ));
}

#[test]
fn migration_fifteen_keeps_existing_sessions_unarchived() {
    let mut conn = Connection::open_in_memory().unwrap();
    apply(&mut conn, &MIGRATIONS[..14]).unwrap();
    conn.execute_batch(
        "INSERT INTO projects (id, name, path, created_at) VALUES ('p', 'p', 'p', 1);
         INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
         VALUES ('s', 'p', 'openai', 'm', 'ask', 'idle', 1, 1);",
    )
    .unwrap();
    assert_eq!(apply(&mut conn, MIGRATIONS).unwrap(), 15);
    let archived: i64 = conn
        .query_row("SELECT archived FROM sessions WHERE id = 's'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(archived, 0);
}
