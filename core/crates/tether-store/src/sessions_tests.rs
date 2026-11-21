use tether_types::{
    MessagePart, ModelRef, PermissionMode, ProjectId, Provider, Role, SessionStatus, ToolCallId,
};

use crate::{Store, StoreError};

#[test]
fn session_model_and_mode_updates_persist_and_bump_activity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tether.db");
    let mut store = Store::open(&path).unwrap();
    let project = seed_project(&store);
    let session = store
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "first".into(),
            },
            PermissionMode::Ask,
            10,
        )
        .unwrap();
    store
        .set_session_model(&session.id, Provider::Gemini, "second", 20)
        .unwrap();
    store
        .set_session_mode(&session.id, PermissionMode::Auto, 30)
        .unwrap();
    drop(store);
    let saved = Store::open(&path)
        .unwrap()
        .load_session(&session.id)
        .unwrap()
        .unwrap();
    assert_eq!(saved.provider, Provider::Gemini);
    assert_eq!(saved.model_id, "second");
    assert_eq!(saved.mode, PermissionMode::Auto);
    assert_eq!(saved.updated_at, 30);
}

#[test]
fn settings_updates_advance_activity_within_one_millisecond() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let session = store
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "first".into(),
            },
            PermissionMode::Ask,
            10,
        )
        .unwrap();
    store
        .set_session_model(&session.id, Provider::Gemini, "second", 10)
        .unwrap();
    assert_eq!(
        store.load_session(&session.id).unwrap().unwrap().updated_at,
        11
    );
    store
        .set_session_mode(&session.id, PermissionMode::Auto, 10)
        .unwrap();
    assert_eq!(
        store.load_session(&session.id).unwrap().unwrap().updated_at,
        12
    );
}

#[test]
fn active_status_blocks_model_and_mode_updates() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let session = store
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "first".into(),
            },
            PermissionMode::Ask,
            10,
        )
        .unwrap();
    for status in [
        SessionStatus::Requesting,
        SessionStatus::Streaming,
        SessionStatus::CheckingTool,
        SessionStatus::RunningTool,
        SessionStatus::AwaitingApproval,
    ] {
        store.set_session_status(&session.id, status, 11).unwrap();
        assert!(matches!(
            store.set_session_model(&session.id, Provider::Gemini, "second", 20),
            Err(StoreError::SessionBusy)
        ));
        assert!(matches!(
            store.set_session_mode(&session.id, PermissionMode::Auto, 20),
            Err(StoreError::SessionBusy)
        ));
    }
    let saved = store.load_session(&session.id).unwrap().unwrap();
    assert_eq!(saved.provider, Provider::Anthropic);
    assert_eq!(saved.model_id, "first");
    assert_eq!(saved.mode, PermissionMode::Ask);
    assert_eq!(saved.updated_at, 11);
}

#[test]
fn rename_trims_title_and_rejects_blank_text() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let session = store
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "first".into(),
            },
            PermissionMode::Ask,
            10,
        )
        .unwrap();
    store.rename_session(&session.id, "  New name  ").unwrap();
    assert_eq!(
        store.load_session(&session.id).unwrap().unwrap().title,
        "New name"
    );
    assert!(matches!(
        store.rename_session(&session.id, " \n "),
        Err(StoreError::EmptySessionTitle)
    ));
    assert_eq!(
        store.load_session(&session.id).unwrap().unwrap().title,
        "New name"
    );
}

#[test]
fn session_search_finds_titles_and_old_text_but_skips_rewound_text() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let session = store
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "first".into(),
            },
            PermissionMode::Ask,
            10,
        )
        .unwrap();
    store.rename_session(&session.id, "Alpha title").unwrap();
    let message = store.begin_turn(&session.id, "Old needle", 11).unwrap();
    assert_eq!(
        store.search_sessions(&project, "alpha").unwrap()[0].id,
        session.id
    );
    assert_eq!(
        store.search_sessions(&project, "needle").unwrap()[0].id,
        session.id
    );
    store
        .conn
        .execute(
            "UPDATE messages SET rewound = 1 WHERE id = ?1",
            [message.as_str()],
        )
        .unwrap();
    assert_eq!(store.search_sessions(&project, "needle").unwrap(), []);
}

pub(crate) fn seed_project(store: &Store) -> ProjectId {
    let id = ProjectId::new("01TESTPROJECT00000000000000");
    store
        .conn
        .execute(
            "INSERT INTO projects (id, name, path, created_at) VALUES (?1, 'Test', 'test', 1)",
            [id.as_str()],
        )
        .unwrap();
    id
}

#[test]
fn a_streamed_turn_survives_reopening_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tether.db");
    let session_id = {
        let mut store = Store::open(&path).unwrap();
        let project = seed_project(&store);
        let session = store
            .create_session(
                &project,
                &ModelRef {
                    provider: Provider::Anthropic,
                    model_id: "claude-test".into(),
                },
                PermissionMode::Ask,
                100,
            )
            .unwrap();
        store.begin_turn(&session.id, "Hello", 101).unwrap();
        let answer = store.create_assistant_message(&session.id, 102).unwrap();
        store.save_assistant_parts(&answer, "Hel", "").unwrap();
        store
            .save_assistant_parts(&answer, "Hello there", "")
            .unwrap();
        store
            .set_session_status(&session.id, SessionStatus::Idle, 103)
            .unwrap();
        session.id
    };

    let store = Store::open(&path).unwrap();
    let session = store.load_session(&session_id).unwrap().unwrap();
    assert_eq!(session.title, "Hello");
    assert_eq!(session.status, SessionStatus::Idle);
    let messages = store.messages(&session_id).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, Role::User);
    assert_eq!(
        messages[0].parts,
        vec![MessagePart::Text {
            text: "Hello".into()
        }]
    );
    assert_eq!(messages[1].role, Role::Assistant);
    assert_eq!(
        messages[1].parts,
        vec![MessagePart::Text {
            text: "Hello there".into()
        }]
    );
}

#[test]
fn an_unfinished_turn_reopens_as_interrupted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tether.db");
    let session_id = {
        let mut store = Store::open(&path).unwrap();
        let project = seed_project(&store);
        let session = store
            .create_session(
                &project,
                &ModelRef {
                    provider: Provider::Gemini,
                    model_id: "gemini-test".into(),
                },
                PermissionMode::Ask,
                100,
            )
            .unwrap();
        store.begin_turn(&session.id, "Hello", 101).unwrap();
        session.id
    };
    let store = Store::open(&path).unwrap();
    assert_eq!(store.recover_active_sessions().unwrap(), 1);
    assert_eq!(
        store.load_session(&session_id).unwrap().unwrap().status,
        SessionStatus::Interrupted
    );
    assert_eq!(store.messages(&session_id).unwrap().len(), 1);
}

#[test]
fn begin_turn_waits_for_another_writer_before_reading_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tether.db");
    let mut seed = Store::open(&path).unwrap();
    let project = seed_project(&seed);
    let session = seed
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "test".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    let mut worker_store = Store::open(&path).unwrap();
    let writer = Store::open(&path).unwrap();
    writer
        .conn
        .execute_batch("BEGIN IMMEDIATE; UPDATE projects SET name = 'Busy'")
        .unwrap();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let id = session.id.clone();
    let worker = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        done_tx
            .send(worker_store.begin_turn(&id, "Hello", 2))
            .unwrap();
    });
    started_rx.recv().unwrap();
    assert!(
        done_rx
            .recv_timeout(std::time::Duration::from_millis(100))
            .is_err()
    );
    writer.conn.execute_batch("COMMIT").unwrap();
    assert!(done_rx.recv().unwrap().is_ok());
    worker.join().unwrap();
}

#[test]
fn assistant_message_waits_for_another_writer_before_reading_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tether.db");
    let mut seed = Store::open(&path).unwrap();
    let project = seed_project(&seed);
    let session = seed
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "test".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    seed.begin_turn(&session.id, "Hello", 2).unwrap();
    let mut worker_store = Store::open(&path).unwrap();
    let writer = Store::open(&path).unwrap();
    writer
        .conn
        .execute_batch("BEGIN IMMEDIATE; UPDATE projects SET name = 'Busy'")
        .unwrap();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let id = session.id.clone();
    let worker = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        done_tx
            .send(worker_store.create_assistant_message(&id, 3))
            .unwrap();
    });
    started_rx.recv().unwrap();
    assert!(
        done_rx
            .recv_timeout(std::time::Duration::from_millis(100))
            .is_err()
    );
    writer.conn.execute_batch("COMMIT").unwrap();
    assert!(done_rx.recv().unwrap().is_ok());
    worker.join().unwrap();
}

#[test]
fn tool_exchange_messages_survive_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tether.db");
    let mut store = Store::open(&path).unwrap();
    let project = seed_project(&store);
    let session = store
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "fixture".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    let call = ToolCallId::new("call-1");
    let assistant = store
        .append_message(
            &session.id,
            Role::Assistant,
            &[MessagePart::ToolCall {
                id: call.clone(),
                tool: "read_file".into(),
                args_json: "{}".into(),
            }],
            2,
        )
        .unwrap();
    let result = store
        .append_message(
            &session.id,
            Role::Tool,
            &[MessagePart::ToolResult {
                tool_call_id: call,
                is_error: false,
                text: "ok".into(),
                result_ref: None,
            }],
            3,
        )
        .unwrap();
    assert_eq!(assistant.seq, 0);
    assert_eq!(result.seq, 1);
    drop(store);
    assert_eq!(
        Store::open(&path).unwrap().messages(&session.id).unwrap(),
        vec![assistant, result]
    );
}

#[test]
fn session_times_are_saved_and_follow_the_last_change() {
    let mut store = Store::open_in_memory().unwrap();
    let project = seed_project(&store);
    let model = ModelRef {
        provider: Provider::Anthropic,
        model_id: "model".into(),
    };
    let session = store
        .create_session(&project, &model, PermissionMode::Ask, 1_000)
        .unwrap();
    assert_eq!((session.created_at, session.updated_at), (1_000, 1_000));
    store
        .set_session_status(&session.id, SessionStatus::Streaming, 5_000)
        .unwrap();
    let loaded = store.load_session(&session.id).unwrap().unwrap();
    assert_eq!((loaded.created_at, loaded.updated_at), (1_000, 5_000));
}
