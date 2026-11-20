use super::*;
use crate::sessions_tests::seed_project;
use tether_types::{ModelRef, PermissionMode, Provider};

#[test]
fn finds_text_and_tracks_replacements_and_rewinds() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let session = store.create_session(&project, &ModelRef {
        provider: Provider::Anthropic, model_id: "test".into(),
    }, PermissionMode::Ask, 1).unwrap();
    store.begin_turn(&session.id, "Find the blue lantern", 2).unwrap();
    let hits = store.session_search("blue lantern", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].session_id, session.id);
    let answer = store.create_assistant_message(&session.id, 3).unwrap();
    store.save_assistant_parts(&answer, "A red lantern", "").unwrap();
    assert_eq!(store.session_search("red lantern", 10).unwrap().len(), 1);
    store.save_assistant_parts(&answer, "A green lantern", "").unwrap();
    assert!(store.session_search("red lantern", 10).unwrap().is_empty());
    assert_eq!(store.session_search("green lantern", 10).unwrap().len(), 1);
    store.conn.execute("UPDATE messages SET rewound = 1 WHERE id = ?1", [answer.as_str()]).unwrap();
    assert!(store.session_search("green lantern", 10).unwrap().is_empty());
}

#[test]
fn migration_indexes_existing_messages() {
    use crate::migrations::{self, MIGRATIONS};
    use rusqlite::Connection;
    let mut conn = Connection::open_in_memory().unwrap();
    migrations::apply(&mut conn, &MIGRATIONS[..1]).unwrap();
    conn.execute_batch("INSERT INTO projects (id, name, path, created_at) VALUES ('p', 'P', 'p', 1);
        INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
            VALUES ('s', 'p', 'anthropic', 'test', 'ask', 'idle', 1, 1);
        INSERT INTO messages (id, session_id, role, seq, created_at) VALUES ('m', 's', 'user', 0, 1);
        INSERT INTO message_parts (id, message_id, seq, kind, body_json)
            VALUES ('part', 'm', 0, 'text', '{\"kind\":\"text\",\"text\":\"old phrase\"}');").unwrap();
    migrations::apply(&mut conn, MIGRATIONS).unwrap();
    let count: i64 = conn.query_row("SELECT count(*) FROM message_fts WHERE message_fts MATCH 'old phrase'", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);
}
