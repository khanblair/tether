use tether_types::{ModelRef, PermissionMode, ProjectId, Provider};

use crate::{Store, StoreError};

fn model() -> ModelRef {
    ModelRef {
        provider: Provider::Anthropic,
        model_id: "saved-model".into(),
    }
}

#[test]
fn created_project_loads_with_the_same_fields_and_valid_path() {
    let mut store = Store::open_in_memory().unwrap();
    let created = store.create_project("  My App  ", 100).unwrap();
    assert_eq!(created.name, "My App");
    assert_eq!(created.created_at, 100);
    assert_eq!(created.last_activity_at, 100);
    assert_eq!(created.session_count, 0);
    assert!(created.path.starts_with("my-app-"));
    assert_eq!(created.path.len(), "my-app-".len() + 8);
    assert_eq!(store.load_project(&created.id).unwrap(), Some(created));
}

#[test]
fn duplicate_project_names_get_distinct_paths() {
    let mut store = Store::open_in_memory().unwrap();
    let first = store.create_project("My App", 100).unwrap();
    let second = store.create_project("My App", 101).unwrap();
    assert_ne!(first.path, second.path);
    assert_ne!(first.id, second.id);
}

#[test]
fn project_path_from_traversal_name_has_no_parent_or_separator() {
    let mut store = Store::open_in_memory().unwrap();
    let project = store.create_project("../../etc/passwd", 100).unwrap();
    assert!(!project.path.contains('/'));
    assert!(!project.path.contains(".."));
    assert!(store.load_project(&project.id).unwrap().is_some());
}

#[test]
fn invalid_project_names_are_rejected() {
    let mut store = Store::open_in_memory().unwrap();
    for name in ["", "  ", &"a".repeat(81), "bad\nname"] {
        assert!(matches!(
            store.create_project(name, 100),
            Err(StoreError::InvalidProjectName)
        ));
    }
}

#[test]
fn projects_order_by_session_activity_and_count_sessions() {
    let mut store = Store::open_in_memory().unwrap();
    let older = store.create_project("Older", 100).unwrap();
    let newer = store.create_project("Newer", 200).unwrap();
    assert_eq!(store.list_projects().unwrap()[0].id, newer.id);
    store
        .create_session(&older.id, &model(), PermissionMode::Ask, 300)
        .unwrap();
    store
        .create_session(&older.id, &model(), PermissionMode::Ask, 301)
        .unwrap();
    let listed = store.list_projects().unwrap();
    assert_eq!(listed[0].id, older.id);
    assert_eq!(listed[0].last_activity_at, 301);
    assert_eq!(listed[0].session_count, 2);
    assert_eq!(listed[1].id, newer.id);
    assert_eq!(
        store.load_project(&older.id).unwrap(),
        Some(listed[0].clone())
    );
    assert_eq!(
        store.load_project(&ProjectId::new("missing")).unwrap(),
        None
    );
}

#[test]
fn project_defaults_round_trip_through_load_list_and_session_defaults() {
    let mut store = Store::open_in_memory().unwrap();
    let project = store.create_project("Defaults", 100).unwrap();
    assert_eq!(project.default_provider, None);
    assert_eq!(project.default_model_id, None);
    assert_eq!(project.default_mode, None);
    assert_eq!(store.session_defaults(&project.id).unwrap().model, None);
    assert_eq!(store.session_defaults(&project.id).unwrap().mode, None);

    store
        .set_project_defaults(
            &project.id,
            Some(model()),
            Some(PermissionMode::AcceptEdits),
        )
        .unwrap();
    let loaded = store.load_project(&project.id).unwrap().unwrap();
    assert_eq!(loaded.default_provider, Some(Provider::Anthropic));
    assert_eq!(loaded.default_model_id.as_deref(), Some("saved-model"));
    assert_eq!(loaded.default_mode, Some(PermissionMode::AcceptEdits));
    assert_eq!(store.list_projects().unwrap(), vec![loaded]);
    assert_eq!(
        store.session_defaults(&project.id).unwrap().model,
        Some(model())
    );
    assert_eq!(
        store.session_defaults(&project.id).unwrap().mode,
        Some(PermissionMode::AcceptEdits)
    );
}

#[test]
fn clearing_project_defaults_clears_the_pair_and_mode() {
    let mut store = Store::open_in_memory().unwrap();
    let project = store.create_project("Defaults", 100).unwrap();
    store
        .set_project_defaults(&project.id, Some(model()), Some(PermissionMode::Auto))
        .unwrap();
    store.set_project_defaults(&project.id, None, None).unwrap();
    let loaded = store.load_project(&project.id).unwrap().unwrap();
    assert_eq!(loaded.default_provider, None);
    assert_eq!(loaded.default_model_id, None);
    assert_eq!(loaded.default_mode, None);
    assert_eq!(store.session_defaults(&project.id).unwrap().model, None);
    assert_eq!(store.session_defaults(&project.id).unwrap().mode, None);
}

#[test]
fn project_model_columns_must_be_set_or_cleared_together() {
    let mut store = Store::open_in_memory().unwrap();
    let project = store.create_project("Defaults", 100).unwrap();
    let provider_only = store.conn.execute(
        "UPDATE projects SET default_provider = 'anthropic' WHERE id = ?1",
        [project.id.as_str()],
    );
    assert!(matches!(
        provider_only,
        Err(rusqlite::Error::SqliteFailure(_, _))
    ));
    let model_only = store.conn.execute(
        "UPDATE projects SET default_model_id = 'saved-model' WHERE id = ?1",
        [project.id.as_str()],
    );
    assert!(matches!(
        model_only,
        Err(rusqlite::Error::SqliteFailure(_, _))
    ));
    assert_eq!(store.session_defaults(&project.id).unwrap().model, None);
}

#[test]
fn unknown_project_has_no_defaults_to_update_or_read() {
    let store = Store::open_in_memory().unwrap();
    let missing = ProjectId::new("missing");
    assert!(matches!(
        store.set_project_defaults(&missing, Some(model()), Some(PermissionMode::Ask)),
        Err(StoreError::MissingProject)
    ));
    assert!(matches!(
        store.session_defaults(&missing),
        Err(StoreError::MissingProject)
    ));
}
