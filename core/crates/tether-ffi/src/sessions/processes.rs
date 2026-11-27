//! Session-owned background process controls for the platform UI.

use std::sync::atomic::Ordering;

use super::{SessionRunner, SessionRuntimeError};
use crate::{ProcessStatusView, ProcessView};
use tether_engine::ProcessSnapshot;

#[uniffi::export]
impl SessionRunner {
    /// Reads the session's background process table without waiting for a turn.
    ///
    /// A plain chat session has no process manager and returns an empty list.
    ///
    pub fn list_processes(&self) -> Vec<ProcessView> {
        if !self.valid.load(Ordering::Acquire) {
            return Vec::new();
        }
        self.background
            .as_ref()
            .and_then(|processes| processes.list().ok())
            .map_or_else(Vec::new, map_processes)
    }

    /// Stops one session-owned background process without waiting for a turn.
    ///
    /// Stopping a process that has already stopped is successful. Plain chat
    /// sessions and sessions without Linux tools report `NotFound`.
    ///
    /// # Errors
    /// Returns `NotFound` for an unknown process or a session without a process
    /// manager, and `Unavailable` when the process table cannot be read.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "UniFFI exposes process ids as owned strings"
    )]
    pub fn stop_process(&self, process_id: String) -> Result<(), SessionRuntimeError> {
        if !self.valid.load(Ordering::Acquire) {
            return Err(SessionRuntimeError::Unavailable);
        }
        let Some(processes) = &self.background else {
            return Err(SessionRuntimeError::NotFound);
        };
        let process = processes
            .list()
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .into_iter()
            .find(|process| process.id == process_id)
            .ok_or(SessionRuntimeError::NotFound)?;
        if process.status != tether_types::ProcessStatus::Running {
            return Ok(());
        }
        match processes.stop(&process_id) {
            Ok(_) => Ok(()),
            Err(error) if error.contains("not running") => Ok(()),
            Err(_) => Err(SessionRuntimeError::Unavailable),
        }
    }
}

fn map_processes(processes: Vec<ProcessSnapshot>) -> Vec<ProcessView> {
    processes.into_iter().map(map_process).collect()
}

fn map_process(process: ProcessSnapshot) -> ProcessView {
    ProcessView {
        id: process.id,
        command: process.command,
        status: ProcessStatusView::from(process.status),
        port: process.port,
        started_at_ms: u64::try_from(process.started_at_ms).unwrap_or(0),
    }
}
