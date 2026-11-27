use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use tether_engine::{
    BoxFuture, EventSink as EngineEventSink, ProviderFactory, SessionActor, ToolRuntime,
};
use tether_providers::{ChatProvider, ChatRequest, ProviderError, StopReason, StreamEvent};
use tether_types::PermissionMode;
use tether_workspace::PathGuard;
use tokio::sync::{Notify, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

struct NoopSink;

impl crate::EventSink for NoopSink {
    fn on_event(&self, _event: crate::CoreEventView) {}
}

impl EngineEventSink for NoopSink {
    fn on_event(&self, _event: tether_types::CoreEvent) {}
}

struct PlainPlatform;

impl crate::SecretsPlatform for PlainPlatform {
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

struct BlockingProvider {
    started: Arc<Notify>,
    gate: Arc<Notify>,
}

impl ChatProvider for BlockingProvider {
    fn stream_chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            self.started.notify_one();
            self.gate.notified().await;
            events
                .send(StreamEvent::Finished {
                    stop_reason: StopReason::EndTurn,
                })
                .await
                .map_err(|_| ProviderError::Cancelled)
        })
    }
}

fn seed_session(dir: &tempfile::TempDir) -> (std::path::PathBuf, SessionId) {
    let database = dir.path().join("tether.db");
    let mut store = tether_store::Store::open(&database).unwrap();
    let project = store.create_project("Project", 1).unwrap();
    let session = store
        .create_session(
            &project.id,
            &tether_types::ModelRef {
                provider: tether_types::Provider::Anthropic,
                model_id: "test-model".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    (database, session.id)
}

fn fake_runtime(
    dir: &tempfile::TempDir,
    started: Arc<Notify>,
    gate: Arc<Notify>,
) -> (Arc<SessionRuntime>, SessionId, tokio::task::JoinHandle<()>) {
    let (database, session_id) = seed_session(dir);
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime =
        SessionRuntime::new(database.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let provider: Arc<dyn ChatProvider> = Arc::new(BlockingProvider { started, gate });
    let provider_factory: ProviderFactory = Arc::new(move || Ok(Arc::clone(&provider)));
    let mode = Arc::new(Mutex::new(PermissionMode::Ask));
    let (actor, handle) = SessionActor::new_with_tool_runtime(
        tether_store::Store::open(&database).unwrap(),
        &session_id,
        provider_factory,
        Arc::new(NoopSink),
        ToolRuntime {
            guard: PathGuard::new(&workspace).unwrap(),
            blobs: tether_store::BlobStore::open(dir.path().join("blobs")).unwrap(),
            approver: Arc::new(|_: &str, _: &str| false),
            mode,
            handlers: Vec::new(),
        },
    )
    .unwrap();
    let task = runtime.runtime.spawn(actor.run());
    let runner = Arc::new(SessionRunner {
        handle,
        runtime: Arc::clone(&runtime.runtime),
        valid: AtomicBool::new(true),
        background: None,
    });
    runtime
        .runners
        .lock()
        .unwrap()
        .insert(session_id.clone(), runner);
    (runtime, session_id, task)
}

#[tokio::test]
async fn ffi_bypass_active_does_not_wait_for_a_running_turn() {
    let dir = tempfile::tempdir().unwrap();
    let started = Arc::new(Notify::new());
    let gate = Arc::new(Notify::new());
    let (runtime, session_id, task) = fake_runtime(&dir, Arc::clone(&started), Arc::clone(&gate));
    let sender = runtime
        .runners
        .lock()
        .unwrap()
        .get(&session_id)
        .cloned()
        .unwrap();
    std::thread::spawn(move || sender.send_message("Hi".into()).unwrap())
        .join()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), started.notified())
        .await
        .unwrap();

    let (reply, result) = oneshot::channel();
    let id = session_id.as_str().to_owned();
    let check = Arc::clone(&runtime);
    let thread = std::thread::spawn(move || drop(reply.send(check.bypass_active(&id))));
    let timed = tokio::time::timeout(Duration::from_millis(200), result).await;
    gate.notify_one();
    runtime.runners.lock().unwrap().remove(&session_id);
    task.await.unwrap();
    thread.join().unwrap();
    let passed = matches!(timed, Ok(Ok(Ok(false))));
    tokio::task::spawn_blocking(move || drop(runtime))
        .await
        .unwrap();
    assert!(passed);
}
