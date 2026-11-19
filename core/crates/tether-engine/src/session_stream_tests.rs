#![expect(clippy::excessive_nesting, reason = "steps read in order")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tether_providers::{
    BoxFuture, ChatProvider, ChatRequest, ProviderError, ResponseUsage, StopReason, StreamEvent,
};
use tether_store::Store;
use tether_types::{CoreError, CoreEvent, MessagePart, Role, SessionId, SessionStatus, ToolCallId};
use tokio::sync::{Notify, mpsc};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::session_tests::{Recorder, next_event, seeded_store};
use crate::{SessionActor, SessionHandle};

enum Step {
    Event(StreamEvent),
    Sleep(Duration),
    Gate(Arc<Notify>),
}

impl From<StreamEvent> for Step {
    fn from(event: StreamEvent) -> Self {
        Step::Event(event)
    }
}

struct Script {
    steps: Mutex<Vec<Step>>,
    wait_for_cancel: bool,
    requests: Arc<Mutex<Vec<ChatRequest>>>,
}

impl ChatProvider for Script {
    fn stream_chat<'a>(
        &'a self,
        request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            self.requests.lock().unwrap().push(request.clone());
            let steps = std::mem::take(&mut *self.steps.lock().unwrap());
            for step in steps {
                match step {
                    Step::Event(event) => events.send(event).await.unwrap(),
                    Step::Sleep(time) => tokio::time::sleep(time).await,
                    Step::Gate(gate) => gate.notified().await,
                }
            }
            if !self.wait_for_cancel {
                return Ok(());
            }
            cancel.cancelled().await;
            Err(ProviderError::Cancelled)
        })
    }
}

fn script(steps: Vec<Step>, wait_for_cancel: bool) -> Arc<Script> {
    Arc::new(Script {
        steps: Mutex::new(steps),
        wait_for_cancel,
        requests: Arc::default(),
    })
}

struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

struct Guarded(Arc<AtomicBool>);

impl ChatProvider for Guarded {
    fn stream_chat<'a>(
        &'a self,
        _request: &'a ChatRequest,
        events: mpsc::Sender<StreamEvent>,
        _cancel: CancellationToken,
    ) -> BoxFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            let _guard = DropFlag(Arc::clone(&self.0));
            events.send(tool_call()).await.unwrap();
            std::future::pending().await
        })
    }
}

fn text(text: &str) -> StreamEvent {
    StreamEvent::TextDelta { text: text.into() }
}

fn thinking(text: &str) -> StreamEvent {
    StreamEvent::ThinkingDelta { text: text.into() }
}

fn tool_call() -> StreamEvent {
    StreamEvent::ToolCallStarted {
        id: ToolCallId::new("call-1"),
        name: "read_file".into(),
    }
}

fn finished() -> StreamEvent {
    StreamEvent::Finished {
        stop_reason: StopReason::EndTurn,
    }
}

struct Turn {
    handle: SessionHandle,
    task: JoinHandle<()>,
    rx: mpsc::UnboundedReceiver<CoreEvent>,
    path: std::path::PathBuf,
    id: SessionId,
    _dir: tempfile::TempDir,
}

impl Turn {
    fn start(provider: Arc<dyn ChatProvider>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.db");
        let (store, id) = seeded_store(&path);
        let (tx, rx) = mpsc::unbounded_channel();
        let (actor, handle) =
            SessionActor::new(store, &id, provider, Arc::new(Recorder(tx))).unwrap();
        let task = tokio::spawn(actor.run());
        Self {
            handle,
            task,
            rx,
            path,
            id,
            _dir: dir,
        }
    }

    async fn finish(self) -> Store {
        drop(self.handle);
        self.task.await.unwrap();
        Store::open(&self.path).unwrap()
    }

    async fn cancel_and_close(mut self) -> Store {
        self.handle.interrupt();
        until_turn_end(&mut self.rx).await;
        self.finish().await
    }
}

async fn until_turn_end(rx: &mut mpsc::UnboundedReceiver<CoreEvent>) -> Vec<CoreEvent> {
    let mut events = Vec::new();
    loop {
        let event = next_event(rx).await;
        let done = matches!(
            event,
            CoreEvent::TurnFinished { .. }
                | CoreEvent::TurnFailed { .. }
                | CoreEvent::TurnInterrupted { .. }
        );
        events.push(event);
        if done {
            return events;
        }
    }
}

async fn next_text(rx: &mut mpsc::UnboundedReceiver<CoreEvent>) -> String {
    loop {
        if let CoreEvent::TextDelta { text, .. } = next_event(rx).await {
            return text;
        }
    }
}

async fn ended_with_invalid_input(rx: &mut mpsc::UnboundedReceiver<CoreEvent>) -> bool {
    let events = until_turn_end(rx).await;
    matches!(
        events.last(),
        Some(CoreEvent::TurnFailed {
            error: CoreError::InvalidInput { .. },
            ..
        })
    )
}

async fn first_text_wait(chars: usize) -> Duration {
    let mut turn = Turn::start(script(vec![text(&"x".repeat(chars)).into()], true));
    let started = Instant::now();
    turn.handle.send_message("Hi".into()).await.unwrap();
    next_text(&mut turn.rx).await;
    let waited = started.elapsed();
    turn.cancel_and_close().await;
    waited
}

#[tokio::test]
async fn cache_tokens_count_as_input_tokens() {
    let usage = StreamEvent::Usage(ResponseUsage {
        input_tokens: 10,
        output_tokens: 5,
        cache_read_tokens: 3,
        cache_write_tokens: 7,
    });
    let mut turn = Turn::start(script(vec![usage.into(), finished().into()], false));
    let id = turn.id.clone();
    turn.handle.send_message("Hi".into()).await.unwrap();
    until_turn_end(&mut turn.rx).await;
    let totals = turn.finish().await.session_usage(&id).unwrap();
    assert_eq!(totals.tokens_in, 20);
    assert_eq!(totals.tokens_out, 5);
}

#[tokio::test(start_paused = true)]
async fn a_full_batch_is_sent_at_once_and_a_partial_one_waits_for_the_deadline() {
    assert_eq!(first_text_wait(64).await, Duration::ZERO);
    assert!(first_text_wait(63).await >= Duration::from_millis(32));
}

#[tokio::test(start_paused = true)]
async fn the_batch_counter_restarts_after_a_flush() {
    let gate = Arc::new(Notify::new());
    let steps = vec![
        text(&"a".repeat(40)).into(),
        Step::Gate(Arc::clone(&gate)),
        text(&"b".repeat(40)).into(),
    ];
    let mut turn = Turn::start(script(steps, true));
    turn.handle.send_message("Hi".into()).await.unwrap();
    next_text(&mut turn.rx).await;
    let first_at = Instant::now();
    gate.notify_one();
    next_text(&mut turn.rx).await;
    assert!(first_at.elapsed() >= Duration::from_millis(32));
    turn.cancel_and_close().await;
}

#[tokio::test]
async fn a_tool_use_stop_reason_fails_the_turn() {
    let stop = StreamEvent::Finished {
        stop_reason: StopReason::ToolUse,
    };
    for event in [stop, tool_call()] {
        let mut turn = Turn::start(script(vec![event.into()], false));
        let id = turn.id.clone();
        turn.handle.send_message("Hi".into()).await.unwrap();
        assert!(ended_with_invalid_input(&mut turn.rx).await);
        let store = turn.finish().await;
        let status = store.load_session(&id).unwrap().unwrap().status;
        assert_eq!(status, SessionStatus::Failed);
    }
}

#[tokio::test]
async fn a_failed_stream_stops_the_provider_task() {
    let dropped = Arc::new(AtomicBool::new(false));
    let mut turn = Turn::start(Arc::new(Guarded(Arc::clone(&dropped))));
    turn.handle.send_message("Hi".into()).await.unwrap();
    assert!(ended_with_invalid_input(&mut turn.rx).await);
    tokio::time::timeout(Duration::from_secs(1), async {
        while !dropped.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    turn.finish().await;
}

#[tokio::test]
async fn text_received_before_a_failure_is_saved() {
    let mut turn = Turn::start(script(
        vec![text("partial").into(), tool_call().into()],
        false,
    ));
    let id = turn.id.clone();
    turn.handle.send_message("Hi".into()).await.unwrap();
    assert!(ended_with_invalid_input(&mut turn.rx).await);
    let messages = turn.finish().await.messages(&id).unwrap();
    assert_eq!(
        messages[1].parts,
        vec![MessagePart::Text {
            text: "partial".into()
        }]
    );
}

#[tokio::test]
async fn the_saved_status_is_streaming_while_text_arrives() {
    let mut turn = Turn::start(script(vec![text("hi").into()], true));
    turn.handle.send_message("Hi".into()).await.unwrap();
    next_text(&mut turn.rx).await;
    let saved = Store::open(&turn.path).unwrap();
    let status = saved.load_session(&turn.id).unwrap().unwrap().status;
    assert_eq!(status, SessionStatus::Streaming);
    turn.cancel_and_close().await;
}

#[tokio::test(start_paused = true)]
async fn thinking_is_saved_and_emitted() {
    let steps = vec![
        thinking("pl").into(),
        Step::Sleep(Duration::from_millis(50)),
        thinking("an").into(),
        text("answer").into(),
        finished().into(),
    ];
    let mut turn = Turn::start(script(steps, false));
    let id = turn.id.clone();
    turn.handle.send_message("Hi".into()).await.unwrap();
    let events = until_turn_end(&mut turn.rx).await;

    let thinking_count = events
        .iter()
        .filter(|event| matches!(event, CoreEvent::ThinkingDelta { .. }))
        .count();
    assert_eq!(thinking_count, 2);
    let joined: String = events
        .iter()
        .filter_map(|event| match event {
            CoreEvent::ThinkingDelta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(joined, "plan");
    let text_at = events
        .iter()
        .position(|event| matches!(event, CoreEvent::TextDelta { .. }))
        .unwrap();
    let last_thinking_at = events
        .iter()
        .rposition(|event| matches!(event, CoreEvent::ThinkingDelta { .. }))
        .unwrap();
    assert!(last_thinking_at < text_at);

    let messages = turn.finish().await.messages(&id).unwrap();
    assert_eq!(
        messages[1].parts,
        vec![
            MessagePart::Thinking {
                text: "plan".into()
            },
            MessagePart::Text {
                text: "answer".into()
            },
        ]
    );
}

#[tokio::test]
async fn the_request_carries_the_session_settings() {
    let provider = script(vec![finished().into()], false);
    let mut turn = Turn::start(provider.clone());
    turn.handle.send_message("Hi".into()).await.unwrap();
    until_turn_end(&mut turn.rx).await;
    turn.finish().await;

    let requests = provider.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.max_output_tokens, 4096);
    assert_eq!(request.model_id, "test-model");
    assert_eq!(request.tools.len(), 0);
    assert_eq!(request.system, "");
    assert_eq!(request.messages[0].role, Role::User);
    assert_eq!(
        request.messages[0].parts,
        vec![MessagePart::Text { text: "Hi".into() }]
    );
}
