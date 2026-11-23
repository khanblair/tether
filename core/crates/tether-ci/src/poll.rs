use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;

use crate::{CiClient, CiError, JobRef, RunRef};

/// States in the CI run state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiRunState {
    /// The command is being prepared.
    Preparing,
    /// The branch is being pushed.
    Pushing,
    /// GitHub accepted the dispatch.
    Dispatched,
    /// GitHub has queued the run.
    Queued,
    /// A job is running.
    InProgress,
    /// All jobs passed.
    Succeeded,
    /// A job failed or the run concluded otherwise.
    Failed,
    /// The run was cancelled.
    Cancelled,
    /// The polling deadline elapsed.
    TimedOut,
}

/// A job status sent with progress updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobProgress {
    /// Job name.
    pub name: String,
    /// GitHub job status.
    pub status: String,
    /// GitHub job conclusion.
    pub conclusion: Option<String>,
    /// Current or last running step.
    pub current_step: Option<String>,
}

/// One polling progress event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiProgress {
    /// State after reading the run.
    pub state: CiRunState,
    /// Current jobs.
    pub jobs: Vec<JobProgress>,
}

/// Polling limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PollConfig {
    /// Maximum time spent polling, including queue time.
    pub timeout: Duration,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30 * 60),
        }
    }
}

/// The completed run and its final jobs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiOutcome {
    /// Final state.
    pub state: CiRunState,
    /// Final run reference.
    pub run: RunRef,
    /// Final jobs.
    pub jobs: Vec<JobProgress>,
}

/// A clock-independent sleep seam for polling tests.
pub trait Sleeper: Send + Sync {
    /// Waits for one backoff interval.
    fn sleep<'a>(&'a self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}

/// Maps GitHub run status and conclusion to the CI state machine.
#[must_use]
pub fn state_from_run(status: &str, conclusion: Option<&str>) -> CiRunState {
    match status {
        "queued" | "requested" | "waiting" | "pending" => CiRunState::Queued,
        "in_progress" => CiRunState::InProgress,
        "completed" => match conclusion {
            Some("success") => CiRunState::Succeeded,
            Some("cancelled") => CiRunState::Cancelled,
            Some("timed_out") => CiRunState::TimedOut,
            _ => CiRunState::Failed,
        },
        _ => CiRunState::Failed,
    }
}

/// Polls a workflow with bounded exponential backoff.
///
/// # Errors
/// Returns the first GitHub error, a timeout outcome, or cancellation.
#[expect(
    clippy::too_many_arguments,
    reason = "The polling contract exposes its client, clock seam, cancellation, and progress callback."
)]
pub async fn poll_run<F>(
    client: &CiClient,
    run: RunRef,
    config: PollConfig,
    sleeper: &dyn Sleeper,
    cancel: CancellationToken,
    mut on_progress: F,
) -> Result<CiOutcome, CiError>
where
    F: FnMut(CiProgress),
{
    let started = Instant::now();
    let mut current = run;
    let mut last_state = None;
    let mut delay = Duration::from_secs(10);
    loop {
        if cancel.is_cancelled() {
            drop(client.cancel_run(current.id).await);
            return Ok(CiOutcome {
                state: CiRunState::Cancelled,
                run: current,
                jobs: Vec::new(),
            });
        }
        if started.elapsed() >= config.timeout {
            drop(client.cancel_run(current.id).await);
            return Ok(CiOutcome {
                state: CiRunState::TimedOut,
                run: current,
                jobs: Vec::new(),
            });
        }
        current = client.get_run(current.id).await?;
        let state = state_from_run(&current.status, current.conclusion.as_deref());
        let jobs = client.run_jobs(current.id).await?;
        let progress = jobs.iter().map(JobProgress::from).collect::<Vec<_>>();
        if last_state != Some(state) {
            on_progress(CiProgress {
                state,
                jobs: progress.clone(),
            });
            last_state = Some(state);
            delay = Duration::from_secs(10);
        }
        if is_terminal(state) {
            return Ok(CiOutcome {
                state,
                run: current,
                jobs: progress,
            });
        }
        sleeper.sleep(delay).await;
        delay = next_delay(delay);
    }
}

impl From<&JobRef> for JobProgress {
    fn from(job: &JobRef) -> Self {
        let current_step = job
            .steps
            .iter()
            .find(|step| step.status == "in_progress")
            .or_else(|| job.steps.last())
            .map(|step| step.name.clone());
        Self {
            name: job.name.clone(),
            status: job.status.clone(),
            conclusion: job.conclusion.clone(),
            current_step,
        }
    }
}

fn is_terminal(state: CiRunState) -> bool {
    matches!(
        state,
        CiRunState::Succeeded | CiRunState::Failed | CiRunState::Cancelled | CiRunState::TimedOut
    )
}

fn next_delay(delay: Duration) -> Duration {
    let next = delay.mul_f64(1.5);
    next.min(Duration::from_secs(60))
}
