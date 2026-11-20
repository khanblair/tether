//! Rules of the session store that the mutation run found unpinned.

use rusqlite::params;
use tether_types::{
    MessagePart, ModelRef, PermissionMode, Provider, Role, SessionId, SessionStatus, UsageSnapshot,
};

use crate::Store;
use crate::sessions_tests::seed_project;

fn anthropic() -> ModelRef {
    ModelRef {
        provider: Provider::Anthropic,
        model_id: "test".into(),
    }
}

fn text(value: &str) -> MessagePart {
    MessagePart::Text { text: value.into() }
}

#[test]
fn title_is_the_first_80_characters_of_the_trimmed_first_message() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let long = store
        .create_session(&project, &anthropic(), PermissionMode::Ask, 1)
        .unwrap();
    let padded = format!("  {}  ", "a".repeat(200));
    store.begin_turn(&long.id, &padded, 2).unwrap();
    assert_eq!(
        store.load_session(&long.id).unwrap().unwrap().title,
        "a".repeat(80)
    );

    let short = store
        .create_session(&project, &anthropic(), PermissionMode::Ask, 3)
        .unwrap();
    store.begin_turn(&short.id, "  hello  ", 4).unwrap();
    assert_eq!(
        store.load_session(&short.id).unwrap().unwrap().title,
        "hello"
    );

    let accented = store
        .create_session(&project, &anthropic(), PermissionMode::Ask, 5)
        .unwrap();
    store.begin_turn(&accented.id, &"é".repeat(81), 6).unwrap();
    assert_eq!(
        store.load_session(&accented.id).unwrap().unwrap().title,
        "é".repeat(80)
    );
}

#[test]
fn a_later_message_does_not_change_the_title() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let session = store
        .create_session(&project, &anthropic(), PermissionMode::Ask, 1)
        .unwrap();
    store.begin_turn(&session.id, "First", 2).unwrap();
    store.begin_turn(&session.id, "Second", 3).unwrap();
    assert_eq!(
        store.load_session(&session.id).unwrap().unwrap().title,
        "First"
    );
}

#[test]
fn message_numbers_count_up_by_one_across_roles() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let session = store
        .create_session(&project, &anthropic(), PermissionMode::Ask, 1)
        .unwrap();
    store.begin_turn(&session.id, "one", 2).unwrap();
    store.create_assistant_message(&session.id, 3).unwrap();
    store.begin_turn(&session.id, "two", 4).unwrap();
    store.create_assistant_message(&session.id, 5).unwrap();

    let messages = store.messages(&session.id).unwrap();
    let roles: Vec<Role> = messages.iter().map(|m| m.role).collect();
    let seqs: Vec<u32> = messages.iter().map(|m| m.seq).collect();
    assert_eq!(
        roles,
        vec![Role::User, Role::Assistant, Role::User, Role::Assistant]
    );
    assert_eq!(seqs, vec![0, 1, 2, 3]);
}

#[test]
fn parts_come_back_in_part_order_not_id_order() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let session = store
        .create_session(&project, &anthropic(), PermissionMode::Ask, 1)
        .unwrap();
    let answer = store.create_assistant_message(&session.id, 2).unwrap();
    let thinking = serde_json::to_string(&MessagePart::Thinking {
        text: "pondering".into(),
    })
    .unwrap();
    let reply = serde_json::to_string(&text("answer")).unwrap();
    // The id "z" sorts after "a", but its seq 0 must still come first.
    store
        .conn
        .execute(
            "INSERT INTO message_parts (id, message_id, seq, kind, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["z", answer.as_str(), 0, "thinking", thinking],
        )
        .unwrap();
    store
        .conn
        .execute(
            "INSERT INTO message_parts (id, message_id, seq, kind, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["a", answer.as_str(), 1, "text", reply],
        )
        .unwrap();

    let messages = store.messages(&session.id).unwrap();
    assert_eq!(
        messages[0].parts,
        vec![
            MessagePart::Thinking {
                text: "pondering".into()
            },
            text("answer"),
        ]
    );
}

#[test]
fn recovery_marks_only_unfinished_sessions_interrupted() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let all = [
        SessionStatus::Idle,
        SessionStatus::Requesting,
        SessionStatus::Streaming,
        SessionStatus::CheckingTool,
        SessionStatus::RunningTool,
        SessionStatus::AwaitingApproval,
        SessionStatus::Paused,
        SessionStatus::Interrupted,
        SessionStatus::Failed,
    ];
    let mut ids: Vec<(SessionId, SessionStatus)> = Vec::new();
    for (index, status) in (1_i64..).zip(all) {
        let session = store
            .create_session(&project, &anthropic(), PermissionMode::Ask, index)
            .unwrap();
        store.set_session_status(&session.id, status, 100).unwrap();
        ids.push((session.id, status));
    }

    assert_eq!(store.recover_active_sessions().unwrap(), 6);

    let expected = [
        SessionStatus::Idle,
        SessionStatus::Interrupted,
        SessionStatus::Interrupted,
        SessionStatus::Interrupted,
        SessionStatus::Interrupted,
        SessionStatus::Interrupted,
        SessionStatus::Interrupted,
        SessionStatus::Interrupted,
        SessionStatus::Failed,
    ];
    for ((id, _), want) in ids.iter().zip(expected) {
        assert_eq!(store.load_session(id).unwrap().unwrap().status, want);
    }
}

#[test]
fn usage_adds_up_across_turns() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let session = store
        .create_session(&project, &anthropic(), PermissionMode::Ask, 1)
        .unwrap();
    store
        .add_session_usage(
            &session.id,
            UsageSnapshot {
                tokens_in: 10,
                tokens_out: 5,
                cost_micros: 100,
            },
            2,
        )
        .unwrap();
    store
        .add_session_usage(
            &session.id,
            UsageSnapshot {
                tokens_in: 3,
                tokens_out: 2,
                cost_micros: 40,
            },
            3,
        )
        .unwrap();
    assert_eq!(
        store.session_usage(&session.id).unwrap(),
        UsageSnapshot {
            tokens_in: 13,
            tokens_out: 7,
            cost_micros: 140,
        }
    );
}

#[test]
fn search_finds_old_message_text() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let first = store
        .create_session(&project, &anthropic(), PermissionMode::Ask, 1)
        .unwrap();
    let second = store
        .create_session(&project, &anthropic(), PermissionMode::Ask, 2)
        .unwrap();
    store.begin_turn(&first.id, "needle from an old turn", 3).unwrap();
    store.begin_turn(&first.id, "latest turn", 4).unwrap();
    store.begin_turn(&second.id, "another session", 5).unwrap();

    let found = store.search_sessions(&project, "NEEDLE").unwrap();
    assert_eq!(found.iter().map(|row| &row.id).collect::<Vec<_>>(), vec![&first.id]);
    assert_eq!(store.list_sessions(&project).unwrap()[0].id, second.id);
    assert!(store.search_sessions(&project, "not here").unwrap().is_empty());
}

#[test]
fn rename_changes_the_title_without_losing_searchable_messages() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let session = store
        .create_session(&project, &anthropic(), PermissionMode::Ask, 1)
        .unwrap();
    store.begin_turn(&session.id, "original message", 2).unwrap();
    store.rename_session(&session.id, "  New title  ").unwrap();
    assert_eq!(store.load_session(&session.id).unwrap().unwrap().title, "New title");
    assert_eq!(store.search_sessions(&project, "original").unwrap().len(), 1);
    assert!(matches!(
        store.rename_session(&session.id, "  "),
        Err(crate::StoreError::EmptySessionTitle)
    ));
}
