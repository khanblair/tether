use super::*;
use crate::SecretsPlatform;
use crate::SessionStatusView;
use tether_types::{ModelRef, SessionStatus};

#[test]
fn session_list_orders_activity_and_rejects_unknown_project() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let mut store = Store::open(&db).unwrap();
    let project = store.create_project("Project", 1).unwrap();
    let old = store
        .create_session(
            &project.id,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "saved".into(),
            },
            PermissionMode::Ask,
            10,
        )
        .unwrap();
    let new = store
        .create_session(
            &project.id,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "saved".into(),
            },
            PermissionMode::Ask,
            20,
        )
        .unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    let listed = runtime.list_sessions(project.id.as_str().into()).unwrap();
    assert_eq!(
        listed.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        [new.id.as_str(), old.id.as_str()]
    );
    assert!(matches!(
        runtime.list_sessions("missing".into()),
        Err(SessionRuntimeError::NotFound)
    ));
}

#[test]
fn default_model_reads_first_saved_model_or_none() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let mut store = Store::open(&db).unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    assert_eq!(
        runtime.default_model(ProviderView::Anthropic).unwrap(),
        None
    );
    let model = |id: &str| tether_types::ModelInfo {
        provider: Provider::Anthropic,
        id: id.into(),
        display_name: id.into(),
        context_window: None,
        max_output_tokens: None,
        price: None,
        caps: tether_types::ModelCaps::UNKNOWN,
    };
    store
        .replace_models(Provider::Anthropic, &[model("first"), model("second")], 1)
        .unwrap();
    assert_eq!(
        runtime.default_model(ProviderView::Anthropic).unwrap(),
        Some("first".into())
    );
}

struct PlainPlatform;

impl SecretsPlatform for PlainPlatform {
    fn wrap(
        &self,
        plaintext: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, crate::SecretsPlatformFailure> {
        Ok(plaintext)
    }

    fn unwrap(
        &self,
        wrapped: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, crate::SecretsPlatformFailure> {
        Ok(wrapped)
    }
}

struct NoopSink;

impl EventSink for NoopSink {
    fn on_event(&self, _event: crate::CoreEventView) {}
}

fn seeded_session(db: &Path) -> SessionId {
    let mut store = Store::open(db).unwrap();
    let project = ProjectId::new("01TESTPROJECT00000000000000");
    rusqlite::Connection::open(db)
        .unwrap()
        .execute(
            "INSERT INTO projects (id, name, path, created_at) VALUES (?1, 'Test', 'test', 1)",
            [project.as_str()],
        )
        .unwrap();
    let session = store
        .create_session(
            &project,
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "test-model".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    store.begin_turn(&session.id, "Hi", 2).unwrap();
    session.id
}

#[test]
fn reopening_recovers_an_active_turn_and_reuses_its_actor() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let session_id = seeded_session(&db);

    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime =
        SessionRuntime::new(db.to_str().unwrap(), Arc::clone(&vault), Arc::new(NoopSink)).unwrap();
    let view = runtime.load_session(session_id.as_str()).unwrap();
    assert_eq!(view.status, SessionStatusView::Interrupted);
    assert_eq!(runtime.transcript(session_id.as_str()).unwrap().len(), 1);
    assert!(matches!(
        runtime.open_session(session_id.as_str()),
        Err(SessionRuntimeError::MissingKey)
    ));
    vault.put("provider-anthropic", "test-key").unwrap();
    let first = runtime.open_session(session_id.as_str()).unwrap();
    let again = runtime.open_session(session_id.as_str()).unwrap();
    assert!(Arc::ptr_eq(&first, &again));
    assert_eq!(
        Store::open(&db)
            .unwrap()
            .load_session(&session_id)
            .unwrap()
            .unwrap()
            .status,
        SessionStatus::Interrupted
    );
}

#[test]
fn workspace_session_requires_a_saved_project_folder() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let session_id = seeded_session(&db);
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    vault.put("provider-anthropic", "test-key").unwrap();
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    let root = dir.path().join("workspaces");
    std::fs::create_dir(&root).unwrap();
    assert!(matches!(
        runtime.open_session_with_workspace(session_id.as_str(), root.to_str().unwrap()),
        Err(SessionRuntimeError::Unavailable)
    ));
    std::fs::create_dir(root.join("test")).unwrap();
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute(
            "UPDATE sessions SET mode = 'accept_edits' WHERE id = ?1",
            [session_id.as_str()],
        )
        .unwrap();
    let store = Store::open(&db).unwrap();
    let saved = store.load_session(&session_id).unwrap().unwrap();
    let tools = session_tool_runtime(&store, &saved, &root).unwrap();
    assert!(!(tools.approve)("write_file", "test/note.txt"));
    assert!(
        runtime
            .open_session_with_workspace(session_id.as_str(), root.to_str().unwrap())
            .is_ok()
    );
}

#[test]
fn checkpoint_list_exposes_saved_paths_and_message_anchor() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let session_id = seeded_session(&db);
    let mut store = Store::open(&db).unwrap();
    let anchor = store.messages(&session_id).unwrap()[0].id.clone();
    let blobs = BlobStore::open(dir.path().join("blobs")).unwrap();
    let checkpoint = store
        .save_checkpoint(
            &session_id,
            Some(&anchor),
            &blobs,
            &[tether_store::SnapshotInput {
                path: "note.txt".into(),
                content: None,
            }],
            4,
        )
        .unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    let listed = runtime.list_checkpoints(session_id.as_str()).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, checkpoint.id);
    assert_eq!(listed[0].message_id.as_deref(), Some(anchor.as_str()));
    assert_eq!(listed[0].paths, vec!["note.txt"]);
}

#[test]
fn provider_factory_reads_the_current_key_after_removal() {
    let dir = tempfile::tempdir().unwrap();
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    vault.put("provider-anthropic", "old-key").unwrap();
    let factory = session_provider_factory(Arc::clone(&vault), Provider::Anthropic);
    assert!(factory().is_ok());
    vault.delete("provider-anthropic").unwrap();
    assert!(matches!(
        factory(),
        Err(tether_types::CoreError::Auth { .. })
    ));
    vault.put("provider-anthropic", "new-key").unwrap();
    assert!(factory().is_ok());
}
