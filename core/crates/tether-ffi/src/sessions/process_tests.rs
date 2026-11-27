use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::*;
use crate::{CoreEventView, EventSink, ProcessStatusView, ProcessView, SecretsPlatform};
use tether_engine::{
    BoxFuture, EventSink as EngineEventSink, LinuxRunnerLike, ProviderFactory, SessionActor,
    ToolRuntime,
};
use tether_linux::{LinuxError, LinuxRunReport, OutputSink, RunOutcome, RunRequest};
use tether_providers::{ChatProvider, ChatRequest, ProviderError, StopReason, StreamEvent};
use tether_types::{CoreEvent, ModelRef, PermissionMode, SessionId};
use tether_workspace::PathGuard;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

struct RecordingSink(Mutex<Vec<CoreEventView>>);

impl EventSink for RecordingSink {
    fn on_event(&self, event: CoreEventView) {
        self.0.lock().expect("event lock").push(event);
    }
}

struct EngineNoopSink;

impl EngineEventSink for EngineNoopSink {
    fn on_event(&self, _event: CoreEvent) {}
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

struct BlockingProvider {
    started: Arc<Notify>,
    gate: Arc<Notify>,
}

impl ChatProvider for BlockingProvider {
    fn stream_chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
        events: tokio::sync::mpsc::Sender<StreamEvent>,
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

struct BackgroundRunner;

impl LinuxRunnerLike for BackgroundRunner {
    fn run(
        &self,
        _request: &RunRequest,
        sink: &mut dyn OutputSink,
        cancel: &CancellationToken,
    ) -> Result<LinuxRunReport, LinuxError> {
        sink.stdout(b"Local: http://localhost:5173/\n");
        while !cancel.is_cancelled() {
            std::thread::park_timeout(Duration::from_millis(1));
        }
        Ok(LinuxRunReport {
            outcome: RunOutcome::Cancelled,
            elapsed: Duration::from_millis(1),
            output_bytes: 32,
            output_truncated: false,
        })
    }
}

struct Fixture {
    temp: tempfile::TempDir,
    runtime: Arc<SessionRuntime>,
    runner: Arc<SessionRunner>,
    session_id: SessionId,
    started: Arc<Notify>,
    gate: Arc<Notify>,
    sink: Arc<RecordingSink>,
    task: tokio::task::JoinHandle<()>,
}

#[expect(
    clippy::too_many_lines,
    reason = "The fixture wires the same runtime pieces as a real session open"
)]
fn fixture(with_background: bool) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("tether.db");
    let mut store = tether_store::Store::open(&database).unwrap();
    let project = store.create_project("Project", 1).unwrap();
    let session = store
        .create_session(
            &project.id,
            &ModelRef {
                provider: tether_types::Provider::Anthropic,
                model_id: "test-model".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    let sink = Arc::new(RecordingSink(Mutex::new(Vec::new())));
    let vault = SecretVault::new(
        temp.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = SessionRuntime::new(
        database.to_str().unwrap(),
        vault,
        Arc::clone(&sink) as Arc<dyn EventSink>,
    )
    .unwrap();
    let workspace = temp.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let started = Arc::new(Notify::new());
    let gate = Arc::new(Notify::new());
    let provider: Arc<dyn ChatProvider> = Arc::new(BlockingProvider {
        started: Arc::clone(&started),
        gate: Arc::clone(&gate),
    });
    let provider_factory: ProviderFactory = Arc::new(move || Ok(Arc::clone(&provider)));
    let (actor, handle) = SessionActor::new_with_tool_runtime(
        tether_store::Store::open(&database).unwrap(),
        &session.id,
        provider_factory,
        Arc::new(EngineNoopSink),
        ToolRuntime {
            guard: PathGuard::new(&workspace).unwrap(),
            blobs: tether_store::BlobStore::open(temp.path().join("blobs")).unwrap(),
            approver: Arc::new(|_: &str, _: &str| false),
            mode: Arc::new(Mutex::new(PermissionMode::Ask)),
            handlers: Vec::new(),
        },
    )
    .unwrap();
    let background = with_background.then(|| {
        Arc::new(tether_engine::BackgroundProcesses::new(
            session.id.clone(),
            runtime.session_sink(),
        ))
    });
    let task = runtime.runtime.spawn(actor.run());
    let runner = Arc::new(SessionRunner {
        handle,
        runtime: Arc::clone(&runtime.runtime),
        valid: AtomicBool::new(true),
        background,
    });
    runtime
        .runners
        .lock()
        .unwrap()
        .insert(session.id.clone(), Arc::clone(&runner));
    Fixture {
        temp,
        runtime,
        runner,
        session_id: session.id,
        started,
        gate,
        sink,
        task,
    }
}

async fn finish(fixture: Fixture) {
    let Fixture {
        temp,
        runtime,
        runner,
        session_id,
        gate,
        task,
        ..
    } = fixture;
    gate.notify_one();
    runtime.runners.lock().unwrap().remove(&session_id);
    drop(runner);
    task.await.unwrap();
    tokio::task::spawn_blocking(move || drop(runtime))
        .await
        .unwrap();
    drop(temp);
}

#[tokio::test]
async fn list_processes_returns_port_and_stop_emits_stopped_event() {
    let fixture = fixture(true);
    let processes = fixture.runner.background.as_ref().unwrap();
    processes
        .start(
            Arc::new(BackgroundRunner),
            RunRequest::default(),
            "npm run dev".into(),
            String::new(),
        )
        .unwrap();

    let listed = fixture.runner.list_processes();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, "p1");
    assert_eq!(listed[0].command, "npm run dev");
    assert_eq!(listed[0].status, ProcessStatusView::Running);
    assert_eq!(listed[0].port, Some(5173));
    assert!(listed[0].started_at_ms > 0);

    fixture.runner.stop_process("p1".into()).unwrap();
    assert_eq!(
        fixture.runner.list_processes()[0].status,
        ProcessStatusView::Stopped
    );
    fixture.runner.stop_process("p1".into()).unwrap();
    assert!(fixture.sink.0.lock().unwrap().iter().any(|event| matches!(
        event,
        CoreEventView::ProcessChanged {
            process_id,
            status: ProcessStatusView::Stopped,
            ..
        } if process_id == "p1"
    )));
    finish(fixture).await;
}

#[tokio::test]
async fn process_calls_do_not_wait_for_a_slow_turn() {
    let fixture = fixture(true);
    let processes = fixture.runner.background.as_ref().unwrap();
    processes
        .start(
            Arc::new(BackgroundRunner),
            RunRequest::default(),
            "npm run dev".into(),
            String::new(),
        )
        .unwrap();
    let sender = Arc::clone(&fixture.runner);
    std::thread::spawn(move || sender.send_message("Hi".into()).unwrap())
        .join()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), fixture.started.notified())
        .await
        .unwrap();

    let started = Instant::now();
    assert_eq!(fixture.runner.list_processes().len(), 1);
    assert!(started.elapsed() < Duration::from_millis(200));
    let started = Instant::now();
    fixture.runner.stop_process("p1".into()).unwrap();
    assert!(started.elapsed() < Duration::from_millis(200));
    finish(fixture).await;
}

#[tokio::test]
async fn unknown_process_id_is_not_found() {
    let fixture = fixture(true);
    assert!(matches!(
        fixture.runner.stop_process("missing".into()),
        Err(SessionRuntimeError::NotFound)
    ));
    finish(fixture).await;
}

#[tokio::test]
async fn plain_chat_has_no_processes() {
    let fixture = fixture(false);
    assert_eq!(fixture.runner.list_processes(), Vec::<ProcessView>::new());
    assert!(matches!(
        fixture.runner.stop_process("p1".into()),
        Err(SessionRuntimeError::NotFound)
    ));
    finish(fixture).await;
}
