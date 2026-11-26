use std::sync::Arc;

use tether_store::Store;
use tether_types::{ModelRef, PermissionMode, ProjectId, Provider, SessionId};

use crate::{
    CoreEventView, EventSink, SecretVault, SecretsPlatform, SecretsPlatformFailure, SessionRuntime,
    SessionRuntimeError,
};

struct PlainPlatform;

impl SecretsPlatform for PlainPlatform {
    fn wrap(
        &self,
        plaintext: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, SecretsPlatformFailure> {
        Ok(plaintext)
    }

    fn unwrap(
        &self,
        wrapped: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, SecretsPlatformFailure> {
        Ok(wrapped)
    }
}

struct NoopSink;

impl EventSink for NoopSink {
    fn on_event(&self, _event: CoreEventView) {}
}

/// Builds a runtime with one saved session in a project whose folder is `test`.
fn fixture() -> (tempfile::TempDir, Arc<SessionRuntime>, SessionId) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("tether.db");
    let mut store = Store::open(&db).unwrap();
    let project = ProjectId::new("01TESTPROJECT00000000000000");
    rusqlite::Connection::open(&db)
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
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    vault.put("provider-anthropic", "test-key").unwrap();
    let runtime = SessionRuntime::new(db.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    (dir, runtime, session.id)
}

#[test]
fn bypass_active_is_false_for_a_session_without_tools() {
    let (_dir, runtime, session) = fixture();
    runtime.open_session(session.as_str()).unwrap();
    assert!(!runtime.bypass_active(session.as_str()).unwrap());
}

#[test]
fn bypass_active_follows_enable_and_disable_through_the_runtime() {
    let (dir, runtime, session) = fixture();
    let root = dir.path().join("workspaces");
    std::fs::create_dir_all(root.join("test")).unwrap();
    runtime
        .open_session_with_workspace(session.as_str(), root.to_str().unwrap())
        .unwrap();
    assert!(!runtime.bypass_active(session.as_str()).unwrap());
    runtime.enable_bypass(session.as_str()).unwrap();
    assert!(runtime.bypass_active(session.as_str()).unwrap());
    runtime.disable_bypass(session.as_str()).unwrap();
    assert!(!runtime.bypass_active(session.as_str()).unwrap());
}

#[test]
fn bypass_active_reports_not_found_for_a_saved_session_that_is_not_open() {
    let (_dir, runtime, session) = fixture();
    assert!(matches!(
        runtime.bypass_active(session.as_str()),
        Err(SessionRuntimeError::NotFound)
    ));
}
