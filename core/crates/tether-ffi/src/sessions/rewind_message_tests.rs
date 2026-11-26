//! Tests for rewinding a session to one of its messages through the runner.

use tether_store::{BlobStore, SnapshotInput, Store};
use tether_types::{MessagePart, Role};

use super::tests::ux_runtime;

#[test]
fn rewind_to_message_hides_that_message_and_its_checkpoints() {
    let (dir, db, runtime, session, _project) = ux_runtime();
    runtime.vault.put("provider-anthropic", "test-key").unwrap();
    let mut store = Store::open(&db).unwrap();
    let first = store.begin_turn(&session, "first", 20).unwrap();
    let reply = store
        .append_message(
            &session,
            Role::Assistant,
            &[MessagePart::Text {
                text: "reply".into(),
            }],
            21,
        )
        .unwrap();
    let blobs = BlobStore::open(dir.path().join("blobs")).unwrap();
    let input = [SnapshotInput {
        path: "note.txt".into(),
        content: None,
    }];
    store
        .save_checkpoint(&session, Some(&reply.id), &blobs, &input, 22)
        .unwrap();
    let runner = runtime.open_session(session.as_str()).unwrap();

    runner
        .rewind_to_message(reply.id.as_str().to_owned())
        .unwrap();

    let messages = Store::open(&db).unwrap().messages(&session).unwrap();
    assert_eq!(messages[0].id.as_str(), first.as_str());
    assert!(!messages[0].rewound);
    assert!(messages[1].rewound);
    assert!(
        runtime
            .list_checkpoints(session.as_str())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn rewind_to_an_unknown_message_fails_and_hides_nothing() {
    let (_dir, db, runtime, session, _project) = ux_runtime();
    runtime.vault.put("provider-anthropic", "test-key").unwrap();
    let mut store = Store::open(&db).unwrap();
    store.begin_turn(&session, "first", 20).unwrap();
    let runner = runtime.open_session(session.as_str()).unwrap();

    assert!(matches!(
        runner.rewind_to_message("missing".into()),
        Err(crate::sessions::SessionRuntimeError::NotFound)
    ));
    let messages = Store::open(&db).unwrap().messages(&session).unwrap();
    assert!(messages.iter().all(|message| !message.rewound));
}
