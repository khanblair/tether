//! Session-scoped Linux processes and their bounded output.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use tether_linux::{LinuxError, LinuxRunReport, LinuxRunner, RunRequest};
use tether_types::{CoreEvent, ProcessStatus, SessionId};
use tokio_util::sync::CancellationToken;

const LOG_CAP: usize = 256 * 1024;
const FIRST_OUTPUT_CAP: usize = 4_000;
const START_WAIT: Duration = Duration::from_secs(8);

mod process;

/// The Linux runner contract used by background processes and tests.
pub trait LinuxRunnerLike: Send + Sync {
    /// Runs a command until it exits or its cancellation token fires.
    ///
    /// # Errors
    /// Returns the runner's process or setup error.
    fn run(
        &self,
        request: &RunRequest,
        sink: &mut dyn tether_linux::OutputSink,
        cancel: &CancellationToken,
    ) -> Result<LinuxRunReport, LinuxError>;
}

impl LinuxRunnerLike for LinuxRunner {
    fn run(
        &self,
        request: &RunRequest,
        sink: &mut dyn tether_linux::OutputSink,
        cancel: &CancellationToken,
    ) -> Result<LinuxRunReport, LinuxError> {
        LinuxRunner::run(self, request, sink, cancel)
    }
}

#[derive(Debug)]
struct ProcessState {
    id: String,
    command: String,
    cwd: String,
    started_at_ms: i64,
    status: Mutex<ProcessStatus>,
    port: Mutex<Option<u16>>,
    logs: Mutex<OutputRing>,
    signal: Condvar,
}

#[derive(Debug)]
struct OutputRing {
    bytes: VecDeque<u8>,
    first: String,
    pending: String,
}

impl OutputRing {
    fn new() -> Self {
        Self {
            bytes: VecDeque::with_capacity(LOG_CAP),
            first: String::new(),
            pending: String::new(),
        }
    }

    fn append(&mut self, bytes: &[u8]) {
        self.bytes.extend(bytes.iter().copied());
        while self.bytes.len() > LOG_CAP {
            self.bytes.pop_front();
        }
        if self.first.chars().count() < FIRST_OUTPUT_CAP {
            let remaining = FIRST_OUTPUT_CAP - self.first.chars().count();
            self.first
                .extend(String::from_utf8_lossy(bytes).chars().take(remaining));
        }
        self.pending.push_str(&String::from_utf8_lossy(bytes));
        if self.pending.len() > LOG_CAP {
            let tail: String = self
                .pending
                .chars()
                .skip(self.pending.chars().count().saturating_sub(LOG_CAP))
                .collect();
            self.pending = tail;
        }
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes.iter().copied().collect::<Vec<_>>()).into_owned()
    }
}

#[derive(Debug)]
struct ProcessEntry {
    state: Arc<ProcessState>,
    cancel: CancellationToken,
    join: Mutex<Option<JoinHandle<()>>>,
}

/// The process state owned by one session.
pub struct BackgroundProcesses {
    session_id: SessionId,
    sink: Arc<dyn crate::EventSink>,
    next_id: AtomicUsize,
    processes: Mutex<BTreeMap<String, Arc<ProcessEntry>>>,
}

/// Data returned when a process starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundStart {
    /// Short process identifier.
    pub id: String,
    /// Initial output collected before the tool returned.
    pub first_output: String,
    /// Port discovered in initial output.
    pub port: Option<u16>,
}

impl BackgroundProcesses {
    /// Creates a process table for one session.
    #[must_use]
    pub fn new(session_id: SessionId, sink: Arc<dyn crate::EventSink>) -> Self {
        Self {
            session_id,
            sink,
            next_id: AtomicUsize::new(1),
            processes: Mutex::new(BTreeMap::new()),
        }
    }

    /// Starts a Linux command and waits briefly for a port announcement.
    ///
    /// # Errors
    /// Returns an error when the process table or worker thread cannot start.
    pub fn start(
        &self,
        runner: Arc<dyn LinuxRunnerLike>,
        request: RunRequest,
        command: String,
        cwd: String,
    ) -> Result<BackgroundStart, String> {
        let id = format!("p{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let state = Arc::new(ProcessState {
            id: id.clone(),
            command,
            cwd,
            started_at_ms: process::now_ms(),
            status: Mutex::new(ProcessStatus::Running),
            port: Mutex::new(None),
            logs: Mutex::new(OutputRing::new()),
            signal: Condvar::new(),
        });
        let cancel = CancellationToken::new();
        let entry = Arc::new(ProcessEntry {
            state: Arc::clone(&state),
            cancel: cancel.clone(),
            join: Mutex::new(None),
        });
        self.processes
            .lock()
            .map_err(|_| "process table is unavailable")?
            .insert(id.clone(), Arc::clone(&entry));
        self.emit(ProcessStatus::Running, &state);
        let worker_state = Arc::clone(&state);
        let worker_sink = Arc::clone(&self.sink);
        let session_id = self.session_id.clone();
        let worker = process::ProcessWorker {
            runner,
            request,
            cancel,
            state: worker_state,
            session_id,
            sink: worker_sink,
        };
        let worker = process::spawn(worker, format!("tether-{id}"))?;
        entry
            .join
            .lock()
            .map_err(|_| "process state is unavailable")?
            .replace(worker);
        let start = wait_for_start(&state);
        Ok(BackgroundStart {
            id,
            first_output: start.0,
            port: start.1,
        })
    }

    /// Lists the processes in creation order.
    ///
    /// # Errors
    /// Returns an error when the process table is poisoned.
    pub fn list(&self) -> Result<Vec<ProcessSnapshot>, String> {
        let processes = self
            .processes
            .lock()
            .map_err(|_| "process table is unavailable")?;
        processes
            .values()
            .map(|entry| process::snapshot(&entry.state))
            .collect()
    }

    /// Returns the last output lines for one process.
    ///
    /// # Errors
    /// Returns an error when the process is unknown or its state is unavailable.
    pub fn logs(&self, id: &str, tail: usize) -> Result<String, String> {
        let state = self.state(id)?;
        let text = state
            .logs
            .lock()
            .map_err(|_| "process logs are unavailable")?
            .text();
        let lines: Vec<&str> = text.lines().collect();
        let start = lines.len().saturating_sub(tail.min(200));
        Ok(lines[start..].join("\n"))
    }

    /// Stops a process and waits for the Linux runner to finish.
    ///
    /// # Errors
    /// Returns an error when the process is unknown, already stopped, or cannot be joined.
    pub fn stop(&self, id: &str) -> Result<ProcessSnapshot, String> {
        let entry = self.entry(id)?;
        if !process::is_running(&entry.state) {
            return Err(format!("process {id} is not running"));
        }
        entry.cancel.cancel();
        process::join_entry(&entry)?;
        process::snapshot(&entry.state)
    }

    /// Returns the running process that owns a port.
    ///
    /// # Errors
    /// Returns an error when the table is unavailable or no running process owns the port.
    pub fn owner_for_port(&self, port: u16) -> Result<String, String> {
        let processes = self
            .processes
            .lock()
            .map_err(|_| "process table is unavailable")?;
        processes
            .values()
            .find(|entry| {
                process::is_running(&entry.state) && process::port_of(&entry.state) == Some(port)
            })
            .map(|entry| entry.state.id.clone())
            .ok_or_else(|| format!("no running process owns port {port}"))
    }

    /// Emits a preview request from a process-aware tool.
    pub fn emit_preview(&self, target: tether_types::PreviewTarget) {
        self.sink.on_event(CoreEvent::PreviewRequested {
            session_id: self.session_id.clone(),
            target,
        });
    }

    /// Stops every process owned by this session.
    pub fn shutdown(&self) {
        let entries = self
            .processes
            .lock()
            .map(|processes| processes.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for entry in &entries {
            entry.cancel.cancel();
        }
        for entry in &entries {
            drop(process::join_entry(entry));
        }
    }

    fn state(&self, id: &str) -> Result<Arc<ProcessState>, String> {
        Ok(Arc::clone(&self.entry(id)?.state))
    }

    fn entry(&self, id: &str) -> Result<Arc<ProcessEntry>, String> {
        self.processes
            .lock()
            .map_err(|_| "process table is unavailable")?
            .get(id)
            .cloned()
            .ok_or_else(|| format!("unknown process id: {id}"))
    }

    fn emit(&self, status: ProcessStatus, state: &ProcessState) {
        self.sink.on_event(CoreEvent::ProcessChanged {
            session_id: self.session_id.clone(),
            process_id: state.id.clone(),
            status,
            port: process::port_of(state),
        });
    }
}

impl Drop for BackgroundProcesses {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// A process snapshot used by process tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSnapshot {
    /// Short process identifier.
    pub id: String,
    /// Original command.
    pub command: String,
    /// Workspace-relative working directory.
    pub cwd: String,
    /// Start time in Unix milliseconds.
    pub started_at_ms: i64,
    /// Current process status.
    pub status: ProcessStatus,
    /// Detected listening port.
    pub port: Option<u16>,
}

fn wait_for_start(state: &ProcessState) -> (String, Option<u16>) {
    let deadline = std::time::Instant::now() + START_WAIT;
    let mut status = state.status.lock().ok();
    while status
        .as_ref()
        .is_some_and(|value| **value == ProcessStatus::Running)
        && process::port_of(state).is_none()
        && std::time::Instant::now() < deadline
    {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let Some(guard) = status.take() else { break };
        status = state
            .signal
            .wait_timeout(guard, remaining)
            .ok()
            .map(|pair| pair.0);
    }
    let output = state
        .logs
        .lock()
        .map(|logs| logs.first.clone())
        .unwrap_or_default();
    (output, process::port_of(state))
}

/// Detects a local development server port from one output line.
#[must_use]
pub fn detect_port(line: &str) -> Option<u16> {
    [
        ("http://localhost:", false),
        ("http://127.0.0.1:", false),
        ("http://0.0.0.0:", false),
        ("http://[::1]:", false),
        ("[::1]:", false),
        ("Listening on port ", true),
        ("Serving HTTP on 0.0.0.0 port ", true),
    ]
    .into_iter()
    .find_map(|(marker, phrase)| {
        line.find(marker).and_then(|index| {
            let start = index + marker.len();
            parse_port(&line[start..], phrase)
        })
    })
}

fn parse_port(text: &str, phrase: bool) -> Option<u16> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || (!phrase && text[digits.len()..].starts_with('.')) {
        return None;
    }
    if !phrase
        && text[digits.len()..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
    {
        return None;
    }
    digits.parse().ok().filter(|port| *port > 0)
}

#[cfg(test)]
#[path = "background_tests.rs"]
mod tests;
