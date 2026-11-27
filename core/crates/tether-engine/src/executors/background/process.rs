use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use tether_linux::{LinuxError, LinuxRunReport, OutputSink, RunOutcome, RunRequest};
use tether_types::{CoreEvent, ProcessStatus, SessionId};
use tokio_util::sync::CancellationToken;

use super::{LinuxRunnerLike, ProcessEntry, ProcessSnapshot, ProcessState, detect_port};

pub(super) fn spawn(worker: ProcessWorker, name: String) -> Result<JoinHandle<()>, String> {
    std::thread::Builder::new()
        .name(name)
        .spawn(move || run_process(worker))
        .map_err(|_| "could not start background process".to_owned())
}

pub(super) struct ProcessWorker {
    pub(super) runner: Arc<dyn LinuxRunnerLike>,
    pub(super) request: RunRequest,
    pub(super) cancel: CancellationToken,
    pub(super) state: Arc<ProcessState>,
    pub(super) session_id: SessionId,
    pub(super) sink: Arc<dyn crate::EventSink>,
}

fn run_process(worker: ProcessWorker) {
    let ProcessWorker {
        runner,
        request,
        cancel,
        state,
        session_id,
        sink,
    } = worker;
    let mut output = BackgroundOutput {
        state: Arc::clone(&state),
        session_id: session_id.clone(),
        sink: Arc::clone(&sink),
    };
    let result = runner.run(&request, &mut output, &cancel);
    let status = process_status(result, &cancel);
    if let Ok(mut current) = state.status.lock() {
        *current = status;
    }
    state.signal.notify_all();
    sink.on_event(CoreEvent::ProcessChanged {
        session_id,
        process_id: state.id.clone(),
        status,
        port: port_of(&state),
    });
}

fn process_status(
    result: Result<LinuxRunReport, LinuxError>,
    cancel: &CancellationToken,
) -> ProcessStatus {
    match result {
        Ok(report) if cancel.is_cancelled() || report.outcome == RunOutcome::Cancelled => {
            ProcessStatus::Stopped
        }
        Ok(report) => match report.outcome {
            RunOutcome::Exited(code) | RunOutcome::Killed(code) => ProcessStatus::Exited(code),
            RunOutcome::TimedOut | RunOutcome::Cancelled => ProcessStatus::Stopped,
        },
        Err(_) if cancel.is_cancelled() => ProcessStatus::Stopped,
        Err(_) => ProcessStatus::Exited(1),
    }
}

struct BackgroundOutput {
    state: Arc<ProcessState>,
    session_id: SessionId,
    sink: Arc<dyn crate::EventSink>,
}

impl OutputSink for BackgroundOutput {
    fn stdout(&mut self, bytes: &[u8]) {
        self.write(bytes);
    }

    fn stderr(&mut self, bytes: &[u8]) {
        self.write(bytes);
    }
}

impl BackgroundOutput {
    fn write(&self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        let pending = if let Ok(mut logs) = self.state.logs.lock() {
            logs.append(bytes);
            logs.pending.clone()
        } else {
            String::new()
        };
        let ports = detect_port(&text)
            .into_iter()
            .chain(text.lines().filter_map(detect_port))
            .chain(std::iter::once(pending).filter_map(|line| detect_port(&line)));
        for port in ports {
            set_port(&self.state, port, &self.session_id, &self.sink);
        }
        self.state.signal.notify_all();
    }
}

pub(super) fn set_port(
    state: &ProcessState,
    port: u16,
    session_id: &SessionId,
    sink: &Arc<dyn crate::EventSink>,
) {
    let Ok(mut current) = state.port.lock() else {
        return;
    };
    if current.is_some() {
        return;
    }
    *current = Some(port);
    sink.on_event(CoreEvent::ProcessChanged {
        session_id: session_id.clone(),
        process_id: state.id.clone(),
        status: ProcessStatus::Running,
        port: Some(port),
    });
}

pub(super) fn join_entry(entry: &ProcessEntry) -> Result<(), String> {
    let join = entry
        .join
        .lock()
        .map_err(|_| "process state is unavailable")?
        .take();
    if let Some(join) = join {
        join.join()
            .map_err(|_| "background process thread stopped")?;
    }
    Ok(())
}

pub(super) fn snapshot(state: &ProcessState) -> Result<ProcessSnapshot, String> {
    Ok(ProcessSnapshot {
        id: state.id.clone(),
        command: state.command.clone(),
        cwd: state.cwd.clone(),
        started_at_ms: state.started_at_ms,
        status: *state
            .status
            .lock()
            .map_err(|_| "process state is unavailable")?,
        port: port_of(state),
    })
}

pub(super) fn is_running(state: &ProcessState) -> bool {
    state
        .status
        .lock()
        .is_ok_and(|status| *status == ProcessStatus::Running)
}

pub(super) fn port_of(state: &ProcessState) -> Option<u16> {
    state.port.lock().ok().and_then(|port| *port)
}

pub(super) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}
