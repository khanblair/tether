//! Tests that schema 0001 accepts what tether-types writes and rejects what it should.

use rusqlite::params;
use tether_types::{
    ConnectionStatus, MessagePart, PermissionMode, Provider, Role, SessionStatus, ToolCallId,
};

use crate::Store;

fn seeded() -> Store {
    let store = Store::open_in_memory().unwrap();
    store
        .conn
        .execute_batch(
            "INSERT INTO projects (id, name, path, created_at) VALUES ('p1', 'demo', 'demo', 1);
             INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
             VALUES ('s1', 'p1', 'anthropic', 'm', 'ask', 'idle', 1, 1);",
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

/// Runs `sql` once for each text form and once for a bogus value. Every text form must
/// pass and the bogus value must fail.
fn check_column(store: &Store, sql: &str, forms: &[&str]) {
    for form in forms {
        store
            .conn
            .execute(sql, [form])
            .unwrap_or_else(|e| panic!("{form} rejected by `{sql}`: {e}"));
    }
    assert!(
        store.conn.execute(sql, ["bogus"]).is_err(),
        "`{sql}` took a bogus value"
    );
}

fn texts<T: Copy>(all: &[T], as_str: fn(T) -> &'static str) -> Vec<&'static str> {
    all.iter().map(|&v| as_str(v)).collect()
}

#[test]
fn session_columns_take_every_text_form_and_nothing_else() {
    let store = seeded();
    let sql = |col: &str| format!("UPDATE sessions SET {col} = ?1 WHERE id = 's1'");
    check_column(
        &store,
        &sql("mode"),
        &texts(PermissionMode::ALL, PermissionMode::as_str),
    );
    check_column(
        &store,
        &sql("status"),
        &texts(SessionStatus::ALL, SessionStatus::as_str),
    );
    check_column(
        &store,
        &sql("provider"),
        &texts(Provider::ALL, Provider::as_str),
    );
}

#[test]
fn project_default_mode_and_provider_take_every_text_form() {
    let store = seeded();
    check_column(
        &store,
        "UPDATE projects SET default_mode = ?1 WHERE id = 'p1'",
        &texts(PermissionMode::ALL, PermissionMode::as_str),
    );
    let provider_only = store.conn.execute(
        "UPDATE projects SET default_provider = 'gemini' WHERE id = 'p1'",
        [],
    );
    assert!(
        provider_only.is_err(),
        "a default provider needs a default model id"
    );
}

#[test]
fn provider_settings_take_every_provider_and_status() {
    let store = seeded();
    for provider in Provider::ALL {
        store
            .conn
            .execute(
                "INSERT INTO provider_configs (provider) VALUES (?1)",
                [provider.as_str()],
            )
            .unwrap();
    }
    check_column(
        &store,
        "UPDATE provider_configs SET status = ?1 WHERE provider = 'gemini'",
        &texts(ConnectionStatus::ALL, ConnectionStatus::as_str),
    );
}

#[test]
fn message_roles_take_every_text_form_and_nothing_else() {
    let store = seeded();
    let insert =
        "INSERT INTO messages (id, session_id, role, seq, created_at) VALUES (?1, 's1', ?2, ?3, 1)";
    for (seq, role) in (0_i64..).zip(Role::ALL) {
        store
            .conn
            .execute(insert, params![format!("m{seq}"), role.as_str(), seq])
            .unwrap();
    }
    assert!(
        store
            .conn
            .execute(insert, params!["mx", "bogus", 9])
            .is_err()
    );
}

fn sample_parts() -> Vec<MessagePart> {
    vec![
        MessagePart::Text { text: "t".into() },
        MessagePart::ImageRef {
            blob_hash: "h".into(),
            media_type: "image/png".into(),
        },
        MessagePart::ToolCall {
            id: ToolCallId::new("c1"),
            tool: "x".into(),
            args_json: "{}".into(),
        },
        MessagePart::ToolResult {
            tool_call_id: ToolCallId::new("c1"),
            is_error: false,
            text: String::new(),
            result_ref: None,
        },
        MessagePart::Thinking { text: "t".into() },
        MessagePart::CompactionSummary { text: "s".into() },
    ]
}

#[test]
fn every_message_part_kind_can_be_stored_with_its_json() {
    let store = seeded();
    store
        .conn
        .execute(
            "INSERT INTO messages (id, session_id, role, seq, created_at) VALUES ('m1', 's1', 'assistant', 0, 1)",
            [],
        )
        .unwrap();
    for (seq, part) in (0_i64..).zip(sample_parts().iter()) {
        let body = serde_json::to_string(part).unwrap();
        store
            .conn
            .execute(
                "INSERT INTO message_parts (id, message_id, seq, kind, body_json) VALUES (?1, 'm1', ?2, ?3, ?4)",
                params![format!("part{seq}"), seq, part.kind(), body],
            )
            .unwrap();
    }
    assert_eq!(count(&store, "message_parts"), 6);
}

#[test]
fn deleting_a_project_removes_its_sessions_messages_and_parts() {
    let store = seeded();
    store
        .conn
        .execute_batch(
            "INSERT INTO messages (id, session_id, role, seq, created_at) VALUES ('m1', 's1', 'user', 0, 1);
             INSERT INTO message_parts (id, message_id, seq, kind, body_json) VALUES ('a', 'm1', 0, 'text', '{}');
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
fn tables_are_strict_and_reject_negative_totals_and_bad_days() {
    let store = seeded();
    let text_in_integer = store.conn.execute(
        "INSERT INTO messages (id, session_id, role, seq, created_at) VALUES ('m1', 's1', 'user', 'zero', 1)",
        [],
    );
    let negative = store
        .conn
        .execute("UPDATE sessions SET cost_micros = -1 WHERE id = 's1'", []);
    let bad_day = store.conn.execute(
        "INSERT INTO usage_daily (day, provider, model_id, project_id) VALUES ('2026-1-1', 'gemini', 'm', 'p1')",
        [],
    );
    assert!(text_in_integer.is_err() && negative.is_err() && bad_day.is_err());
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
