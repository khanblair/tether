//! Tests that schema 0001 accepts what tether-types writes and rejects what it should.

use std::collections::BTreeSet;

use rusqlite::params;
use tether_types::{ConnectionStatus, MessagePart, PermissionMode, Provider, Role, SessionStatus};

use crate::Store;

fn seeded() -> Store {
    let store = Store::open_in_memory().unwrap();
    store
        .conn
        .execute_batch(
            "INSERT INTO projects (id, name, path, created_at) VALUES ('p1', 'demo', 'demo', 1);
             INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
             VALUES ('s1', 'p1', 'anthropic', 'm', 'ask', 'idle', 1, 1);
             INSERT INTO messages (id, session_id, role, seq, created_at) VALUES ('m1', 's1', 'user', 0, 1);",
        )
        .unwrap();
    store
}

fn count(store: &Store, table: &str) -> i64 {
    store
        .conn
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn texts<T: Copy>(all: &[T], as_str: fn(T) -> &'static str) -> BTreeSet<String> {
    all.iter().map(|&v| as_str(v).to_owned()).collect()
}

/// The values in `column IN (...)` of the table's CREATE statement.
fn check_list(store: &Store, table: &str, column: &str) -> BTreeSet<String> {
    let sql: String = store
        .conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .unwrap();
    let marker = format!("({column} IN (");
    let start = sql
        .find(&marker)
        .unwrap_or_else(|| panic!("no IN list for {table}.{column}"))
        + marker.len();
    let end = start + sql[start..].find(')').unwrap();
    sql[start..end]
        .split(',')
        .map(|item| item.trim().trim_matches('\'').to_owned())
        .collect()
}

#[test]
fn check_lists_equal_the_rust_text_forms() {
    let store = Store::open_in_memory().unwrap();
    let providers = texts(Provider::ALL, Provider::as_str);
    let modes = texts(PermissionMode::ALL, PermissionMode::as_str);
    let statuses = texts(SessionStatus::ALL, SessionStatus::as_str);
    let connections = texts(ConnectionStatus::ALL, ConnectionStatus::as_str);
    let roles = texts(Role::ALL, Role::as_str);
    let kinds: BTreeSet<String> = MessagePart::KINDS.iter().map(|k| (*k).to_owned()).collect();
    let cases = [
        ("projects", "default_provider", &providers),
        ("projects", "default_mode", &modes),
        ("sessions", "provider", &providers),
        ("sessions", "mode", &modes),
        ("sessions", "status", &statuses),
        ("messages", "role", &roles),
        ("message_parts", "kind", &kinds),
        ("provider_configs", "provider", &providers),
        ("provider_configs", "status", &connections),
        ("models", "provider", &providers),
        ("usage_daily", "provider", &providers),
    ];
    for (table, column, expected) in cases {
        assert_eq!(
            &check_list(&store, table, column),
            expected,
            "{table}.{column}"
        );
    }
}

#[test]
fn a_bogus_value_is_rejected_in_every_enum_column() {
    let store = seeded();
    store
        .conn
        .execute(
            "INSERT INTO provider_configs (provider) VALUES ('gemini')",
            [],
        )
        .unwrap();
    let updates = [
        "UPDATE projects SET default_mode = ?1",
        "UPDATE sessions SET mode = ?1",
        "UPDATE sessions SET status = ?1",
        "UPDATE sessions SET provider = ?1",
        "UPDATE messages SET role = ?1",
        "UPDATE provider_configs SET status = ?1",
        "UPDATE provider_configs SET provider = ?1",
    ];
    for sql in updates {
        assert!(
            store.conn.execute(sql, ["bogus"]).is_err(),
            "`{sql}` took a bogus value"
        );
    }
}

#[test]
fn every_message_part_kind_can_be_stored_with_its_json() {
    let store = seeded();
    let parts = [
        MessagePart::Text { text: "t".into() },
        MessagePart::Thinking { text: "t".into() },
        MessagePart::CompactionSummary { text: "s".into() },
    ];
    for (seq, part) in (0_i64..).zip(&parts) {
        store
            .conn
            .execute(
                "INSERT INTO message_parts (id, message_id, seq, kind, body_json) VALUES (?1, 'm1', ?2, ?3, ?4)",
                params![format!("p{seq}"), seq, part.kind(), serde_json::to_string(part).unwrap()],
            )
            .unwrap();
    }
    assert_eq!(count(&store, "message_parts"), 3);
}

#[test]
fn a_default_provider_needs_a_default_model_id() {
    let store = seeded();
    let provider_only = store
        .conn
        .execute("UPDATE projects SET default_provider = 'gemini'", []);
    assert!(provider_only.is_err());
}

#[test]
fn a_project_path_must_stay_inside_the_workspace_folder() {
    let store = Store::open_in_memory().unwrap();
    let insert = "INSERT INTO projects (id, name, path, created_at) VALUES (?1, 'n', ?2, 1)";
    for (id, path) in [("a", "repo"), ("b", "team/repo"), ("c", "my..repo")] {
        store.conn.execute(insert, [id, path]).unwrap();
    }
    for (id, path) in [
        ("d", ""),
        ("e", "/etc"),
        ("f", "../x"),
        ("g", "a/../x"),
        ("h", "a/.."),
    ] {
        assert!(
            store.conn.execute(insert, [id, path]).is_err(),
            "took path {path:?}"
        );
    }
}

#[test]
fn two_projects_cannot_share_a_path() {
    let store = seeded();
    let twin = store.conn.execute(
        "INSERT INTO projects (id, name, path, created_at) VALUES ('p2', 'n', 'demo', 1)",
        [],
    );
    assert!(twin.is_err());
}

#[test]
fn a_session_cannot_have_two_messages_with_the_same_seq() {
    let store = seeded();
    let twin = store.conn.execute(
        "INSERT INTO messages (id, session_id, role, seq, created_at) VALUES ('m2', 's1', 'user', 0, 1)",
        [],
    );
    assert!(twin.is_err());
}

#[test]
fn a_message_cannot_have_two_parts_with_the_same_seq() {
    let store = seeded();
    let insert = "INSERT INTO message_parts (id, message_id, seq, kind, body_json) VALUES (?1, 'm1', 0, 'text', '{}')";
    store.conn.execute(insert, ["a"]).unwrap();
    assert!(store.conn.execute(insert, ["b"]).is_err());
}

#[test]
fn rewound_is_zero_or_one() {
    let store = seeded();
    assert!(
        store
            .conn
            .execute("UPDATE messages SET rewound = 2", [])
            .is_err()
    );
    store
        .conn
        .execute("UPDATE messages SET rewound = 1", [])
        .unwrap();
}

#[test]
fn deleting_a_project_removes_its_sessions_messages_and_parts() {
    let store = seeded();
    store
        .conn
        .execute_batch(
            "INSERT INTO message_parts (id, message_id, seq, kind, body_json) VALUES ('a', 'm1', 0, 'text', '{}');
             DELETE FROM projects WHERE id = 'p1';",
        )
        .unwrap();
    for table in ["sessions", "messages", "message_parts"] {
        assert_eq!(count(&store, table), 0, "{table} was not removed");
    }
}

#[test]
fn a_session_needs_an_existing_project() {
    let store = Store::open_in_memory().unwrap();
    let orphan = store.conn.execute(
        "INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
         VALUES ('s1', 'nope', 'anthropic', 'm', 'ask', 'idle', 1, 1)",
        [],
    );
    assert!(orphan.is_err());
}

#[test]
fn every_table_is_strict() {
    let store = Store::open_in_memory().unwrap();
    let mut statement = store
        .conn
        .prepare("SELECT name FROM pragma_table_list WHERE schema = 'main' AND type = 'table' AND strict = 0 AND name NOT LIKE 'sqlite_%'")
        .unwrap();
    let loose: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(loose.is_empty(), "tables without STRICT: {loose:?}");
}

#[test]
fn strict_tables_reject_text_in_an_integer_column() {
    let store = seeded();
    let text_seq = store.conn.execute(
        "INSERT INTO messages (id, session_id, role, seq, created_at) VALUES ('m2', 's1', 'user', 'zero', 1)",
        [],
    );
    let text_window = store.conn.execute(
        "INSERT INTO models (provider, model_id, context_window, fetched_at) VALUES ('gemini', 'm', 'big', 1)",
        [],
    );
    assert!(text_seq.is_err());
    assert!(text_window.is_err());
}

#[test]
fn negative_totals_are_rejected() {
    let store = seeded();
    assert!(
        store
            .conn
            .execute("UPDATE sessions SET cost_micros = -1", [])
            .is_err()
    );
    assert!(
        store
            .conn
            .execute("UPDATE sessions SET tokens_in = -1", [])
            .is_err()
    );
}

#[test]
fn a_usage_day_must_look_like_a_date() {
    let store = seeded();
    let insert = "INSERT INTO usage_daily (day, provider, model_id, project_id) VALUES (?1, 'gemini', 'm', 'p1')";
    store.conn.execute(insert, ["2026-11-19"]).unwrap();
    assert!(store.conn.execute(insert, ["2026-1-1"]).is_err());
}

#[test]
fn the_bundled_sqlite_has_fts5() {
    let store = Store::open_in_memory().unwrap();
    store
        .conn
        .execute_batch(
            "CREATE VIRTUAL TABLE probe USING fts5(body);
             INSERT INTO probe (body) VALUES ('rename the parser module');",
        )
        .unwrap();
    let hits: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM probe WHERE probe MATCH 'parser'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(hits, 1);
}
