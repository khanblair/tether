use std::time::{Duration, Instant};

use tether_types::{MessagePart, ModelRef, PermissionMode, ProjectId, Provider, Role, ToolCallId};

use crate::{Store, run_history::RunCallRow};

fn project(store: &mut Store, name: &str) -> ProjectId {
    store.create_project(name, 1).unwrap().id
}

fn session(store: &mut Store, project: &ProjectId, title: &str) -> tether_types::SessionId {
    let session = store
        .create_session(
            project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "model".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    if !title.is_empty() {
        store.rename_session(&session.id, title).unwrap();
    }
    session.id
}

fn call(store: &mut Store, session: &tether_types::SessionId, id: &str, args: &str, at: i64) {
    store
        .append_message(
            session,
            Role::Assistant,
            &[MessagePart::ToolCall {
                id: ToolCallId::new(id),
                tool: "run".into(),
                args_json: args.into(),
            }],
            at,
        )
        .unwrap();
}

fn result(store: &mut Store, session: &tether_types::SessionId, id: &str, text: &str, at: i64) {
    store
        .append_message(
            session,
            Role::Tool,
            &[MessagePart::ToolResult {
                tool_call_id: ToolCallId::new(id),
                is_error: false,
                text: text.into(),
                result_ref: None,
            }],
            at,
        )
        .unwrap();
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The history filter test keeps all exclusion cases in one scenario."
)]
fn run_calls_filter_live_project_history_and_pair_results_by_session() {
    let mut store = Store::open_in_memory().unwrap();
    let project_id = project(&mut store, "Project");
    let other_project = project(&mut store, "Other");
    let first = session(&mut store, &project_id, "First");
    let second = session(&mut store, &project_id, "Second");
    let archived = session(&mut store, &project_id, "Archived");
    let rewound = session(&mut store, &project_id, "Rewound");
    let other = session(&mut store, &other_project, "Other");

    call(&mut store, &first, "call_1", r#"{"command":"first"}"#, 10);
    result(&mut store, &first, "call_1", "first result", 11);
    call(&mut store, &second, "call_1", r#"{"command":"second"}"#, 20);
    result(&mut store, &second, "call_1", "second result", 21);
    call(&mut store, &first, "read", r#"{"command":"ignored"}"#, 30);
    store
        .conn
        .execute(
            "UPDATE message_parts SET body_json = json_set(body_json, '$.tool', 'read_file') WHERE id = (SELECT p.id FROM message_parts p JOIN messages m ON m.id = p.message_id WHERE m.session_id = ?1 AND json_extract(p.body_json, '$.id') = 'read')",
            [first.as_str()],
        )
        .unwrap();
    call(
        &mut store,
        &archived,
        "archived",
        r#"{"command":"ignored"}"#,
        40,
    );
    store
        .conn
        .execute(
            "UPDATE sessions SET archived = 1 WHERE id = ?1",
            [archived.as_str()],
        )
        .unwrap();
    call(
        &mut store,
        &rewound,
        "rewound",
        r#"{"command":"ignored"}"#,
        50,
    );
    store
        .conn
        .execute(
            "UPDATE messages SET rewound = 1 WHERE id = (SELECT message_id FROM message_parts WHERE json_extract(body_json, '$.id') = 'rewound')",
            [],
        )
        .unwrap();
    call(&mut store, &other, "other", r#"{"command":"ignored"}"#, 60);

    let rows = store.run_calls(&project_id, 200).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].session_id, second);
    assert_eq!(rows[0].result_text.as_deref(), Some("second result"));
    assert_eq!(rows[1].session_id, first);
    assert_eq!(rows[1].result_text.as_deref(), Some("first result"));
}

#[test]
fn run_calls_keep_missing_results_and_clamp_limit() {
    let mut store = Store::open_in_memory().unwrap();
    let project_id = project(&mut store, "Project");
    let session_id = session(&mut store, &project_id, "Runs");
    call(&mut store, &session_id, "one", r#"{"command":"one"}"#, 10);
    call(&mut store, &session_id, "two", r#"{"command":"two"}"#, 20);
    call(
        &mut store,
        &session_id,
        "three",
        r#"{"command":"three"}"#,
        30,
    );

    assert_eq!(store.run_calls(&project_id, 0).unwrap().len(), 1);
    assert_eq!(store.run_calls(&project_id, 1_000).unwrap().len(), 3);
    let rows = store.run_calls(&project_id, 200).unwrap();
    assert_eq!(rows[0].call_id, ToolCallId::new("three"));
    assert!(rows[0].result_text.is_none());
}

#[test]
fn run_calls_scan_a_few_thousand_rows_quickly() {
    let mut store = Store::open_in_memory().unwrap();
    let project_id = project(&mut store, "Project");
    let session_id = session(&mut store, &project_id, "Many runs");
    for number in 0..3_000 {
        call(
            &mut store,
            &session_id,
            &format!("call_{number}"),
            r#"{"command":"echo"}"#,
            number,
        );
    }

    let started = Instant::now();
    let rows = store.run_calls(&project_id, 200).unwrap();
    assert_eq!(rows.len(), 200);
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn run_call_row_keeps_the_saved_session_title_and_arguments() {
    let mut store = Store::open_in_memory().unwrap();
    let project_id = project(&mut store, "Project");
    let session_id = session(&mut store, &project_id, "Build session");
    call(
        &mut store,
        &session_id,
        "call",
        r#"{"command":"cargo test","executor":"ci"}"#,
        10,
    );

    let row: RunCallRow = store.run_calls(&project_id, 1).unwrap().remove(0);
    assert_eq!(row.session_title, "Build session");
    assert_eq!(row.args_json, r#"{"command":"cargo test","executor":"ci"}"#);
    assert_eq!(row.called_at_ms, 10);
}
