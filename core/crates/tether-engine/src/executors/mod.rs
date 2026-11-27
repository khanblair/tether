//! Command executors and the built-in executor router.

pub mod background;
mod process_tools;
mod router;
mod run;

pub use background::{
    BackgroundProcesses, BackgroundStart, LinuxRunnerLike, ProcessSnapshot, detect_port,
};
pub use process_tools::ProcessTools;
pub use router::{Availability, Route, RouteError, RouteInput, Router, ShellFeature, tokenize};
pub use run::RunTools;

use std::time::Duration;

use tether_providers::BoxFuture;
use tether_types::ProjectId;
use tokio_util::sync::CancellationToken;

/// A user or model command sent to a CI executor.
#[derive(Debug, Clone)]
pub struct CiRunRequest {
    /// Shell command to run.
    pub command: String,
    /// Project whose repository owns the run.
    pub project_id: ProjectId,
    /// Cancellation for the remote run and log polling.
    pub cancel: CancellationToken,
}

/// The final state of an executor run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// The command returned an exit code.
    Exited(i32),
    /// The executor stopped at its time limit.
    TimedOut,
    /// The caller cancelled the run.
    Cancelled,
    /// The guest exceeded its memory limit.
    OutOfMemory,
}

/// A CI-compatible run report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    /// The final executor state.
    pub status: RunStatus,
    /// Combined output collected by the executor.
    pub output: String,
    /// Wall time spent running the command.
    pub duration: Duration,
    /// Whether the executor discarded output at its cap.
    pub output_truncated: bool,
}

/// Errors returned by a CI executor.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExecutorError {
    /// The requested executor has not been configured.
    #[error("CI executor is not configured")]
    NotConfigured,
    /// The executor rejected the command or could not start.
    #[error("executor error: {0}")]
    Failed(String),
}

/// Runs commands through the CI backend owned by another task.
pub trait CiExecutor: Send + Sync {
    /// Returns whether this implementation can accept a run.
    #[must_use]
    fn is_configured(&self) -> bool {
        true
    }

    /// Starts one remote command and streams its output through `sink`.
    fn run<'a>(
        &'a self,
        request: CiRunRequest,
        sink: Option<&'a dyn crate::EventSink>,
    ) -> BoxFuture<'a, Result<RunReport, ExecutorError>>;
}

/// A safe placeholder until the CI executor is installed.
#[derive(Debug, Default)]
pub struct NotConfigured;

impl CiExecutor for NotConfigured {
    fn is_configured(&self) -> bool {
        false
    }

    fn run<'a>(
        &'a self,
        _request: CiRunRequest,
        _sink: Option<&'a dyn crate::EventSink>,
    ) -> BoxFuture<'a, Result<RunReport, ExecutorError>> {
        Box::pin(std::future::ready(Err(ExecutorError::NotConfigured)))
    }
}
