use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{BackgroundProcesses, LinuxRunnerLike, detect_port};
use tether_linux::{LinuxError, LinuxRunReport, OutputSink, RunOutcome, RunRequest};
use tether_types::{CoreEvent, ProcessStatus, SessionId};
use tokio_util::sync::CancellationToken;

struct Recorder(Mutex<Vec<CoreEvent>>);

impl crate::EventSink for Recorder {
    fn on_event(&self, event: CoreEvent) {
        if !matches!(event, CoreEvent::ProcessChanged { .. }) {
            return;
        }
        self.0.lock().expect("recorder lock").push(event);
    }
}

struct FakeRunner;

impl LinuxRunnerLike for FakeRunner {
    fn run(
        &self,
        _request: &RunRequest,
        sink: &mut dyn OutputSink,
        cancel: &CancellationToken,
    ) -> Result<LinuxRunReport, LinuxError> {
        sink.stdout(b"Local: http://localhost:5173/\nready\n");
        wait_for_cancel(cancel);
        Ok(LinuxRunReport {
            outcome: RunOutcome::Cancelled,
            elapsed: Duration::from_millis(1),
            output_bytes: 32,
            output_truncated: false,
        })
    }
}

fn wait_for_cancel(cancel: &CancellationToken) {
    while !cancel.is_cancelled() {
        std::thread::park_timeout(Duration::from_millis(1));
    }
}

fn process_table() -> (Arc<BackgroundProcesses>, Arc<Recorder>) {
    let recorder = Arc::new(Recorder(Mutex::new(Vec::new())));
    let table = Arc::new(BackgroundProcesses::new(
        SessionId::new("session"),
        Arc::clone(&recorder) as Arc<dyn crate::EventSink>,
    ));
    (table, recorder)
}

#[test]
fn detects_real_world_server_lines() {
    let cases = [
        ("Local: http://localhost:5173/", Some(5173)),
        ("▲ Next.js ready on http://127.0.0.1:3000", Some(3000)),
        (
            "Serving HTTP on 0.0.0.0 port 8000 (http://0.0.0.0:8000/) ...",
            Some(8000),
        ),
        (" npx serve: http://localhost:5000", Some(5000)),
        ("Available on: http://127.0.0.1:8080", Some(8080)),
        ("Listening on port 4000", Some(4000)),
        (" * Running on http://127.0.0.1:5000", Some(5000)),
        ("server listening at [::1]:3001", Some(3001)),
    ];
    for (line, expected) in cases {
        assert_eq!(detect_port(line), expected, "{line}");
    }
}

#[test]
fn ignores_numbers_that_are_not_ports() {
    for line in ["took 3000ms", "vite v5173.0", "http://example.com:3000"] {
        assert_eq!(detect_port(line), None, "{line}");
    }
}

#[test]
fn background_process_returns_port_logs_tail_and_stops_once() {
    let (table, recorder) = process_table();
    let started = table
        .start(
            Arc::new(FakeRunner),
            RunRequest::default(),
            "npm run dev".into(),
            String::new(),
        )
        .expect("start process");

    assert_eq!(started.id, "p1");
    assert_eq!(started.port, Some(5173));
    assert!(started.first_output.contains("ready"));
    assert_eq!(table.logs("p1", 1).expect("logs"), "ready");

    let stopped = table.stop("p1").expect("stop process");
    assert_eq!(stopped.status, ProcessStatus::Stopped);
    assert!(table.stop("p1").is_err());
    let events = recorder.0.lock().expect("recorder lock");
    assert!(matches!(
        events.first(),
        Some(CoreEvent::ProcessChanged {
            status: ProcessStatus::Running,
            port: None,
            ..
        })
    ));
    assert!(matches!(
        events.get(1),
        Some(CoreEvent::ProcessChanged {
            status: ProcessStatus::Running,
            port: Some(5173),
            ..
        })
    ));
    assert!(matches!(
        events.last(),
        Some(CoreEvent::ProcessChanged {
            status: ProcessStatus::Stopped,
            ..
        })
    ));
}

#[test]
fn dropping_process_table_stops_all_processes() {
    let (table, recorder) = process_table();
    table
        .start(
            Arc::new(FakeRunner),
            RunRequest::default(),
            "npm run dev".into(),
            String::new(),
        )
        .expect("start process");
    drop(table);

    let events = recorder.0.lock().expect("recorder lock");
    assert!(matches!(
        events.last(),
        Some(CoreEvent::ProcessChanged {
            status: ProcessStatus::Stopped,
            ..
        })
    ));
}

#[test]
fn output_ring_keeps_only_the_last_256_kibibytes() {
    let (table, _) = process_table();
    let state = super::ProcessState {
        id: "p1".into(),
        command: "echo".into(),
        cwd: String::new(),
        started_at_ms: 0,
        status: Mutex::new(ProcessStatus::Exited(0)),
        port: Mutex::new(None),
        logs: Mutex::new(super::OutputRing::new()),
        signal: std::sync::Condvar::new(),
    };
    state
        .logs
        .lock()
        .expect("logs lock")
        .append(&vec![b'x'; super::LOG_CAP + 1]);
    assert_eq!(
        state.logs.lock().expect("logs lock").bytes.len(),
        super::LOG_CAP
    );
    drop(table);
}
