use tether_types::{ModelRef, PermissionMode, Provider, SessionId, TodoItem, TodoStatus};

use crate::Store;

fn item(text: &str, status: TodoStatus) -> TodoItem {
    TodoItem {
        text: text.into(),
        status,
    }
}

fn session(store: &mut Store) -> (tether_types::ProjectId, SessionId) {
    let project = store.create_project("Todos", 1).unwrap();
    let session = store
        .create_session(
            &project.id,
            &ModelRef {
                provider: Provider::OpenAi,
                model_id: "model".into(),
            },
            PermissionMode::Plan,
            1,
        )
        .unwrap();
    (project.id, session.id)
}

#[test]
fn replaces_and_reopens_the_session_checklist() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tether.db");
    let session_id = {
        let mut store = Store::open(&path).unwrap();
        let (_, session_id) = session(&mut store);
        store
            .replace_todos(
                &session_id,
                &[
                    item("read", TodoStatus::Completed),
                    item("write", TodoStatus::InProgress),
                ],
                20,
            )
            .unwrap();
        session_id
    };
    let store = Store::open(&path).unwrap();
    assert_eq!(
        store.todos(&session_id).unwrap(),
        vec![
            item("read", TodoStatus::Completed),
            item("write", TodoStatus::InProgress)
        ]
    );
}

#[test]
fn deleting_the_parent_project_deletes_session_todos() {
    let mut store = Store::open_in_memory().unwrap();
    let (project_id, session_id) = session(&mut store);
    store
        .replace_todos(&session_id, &[item("x", TodoStatus::Pending)], 2)
        .unwrap();
    store.delete_project(&project_id).unwrap();
    assert_eq!(store.todos(&session_id).unwrap(), Vec::<TodoItem>::new());
}

#[test]
fn migration_six_is_applied() {
    let store = Store::open_in_memory().unwrap();
    assert!(store.schema_version().unwrap() >= 6);
}
