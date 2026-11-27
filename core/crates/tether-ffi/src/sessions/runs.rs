use serde_json::Value;
use tether_engine::executors::{ParsedRun, RunOutcome, parse_run_result};
use tether_store::{Store, run_history::RunCallRow};
use tether_types::ProjectId;

use super::{SessionRuntime, SessionRuntimeError};
use crate::{ExecutorView, RunHistoryView, RunOutcomeView};

#[uniffi::export]
impl SessionRuntime {
    /// Lists saved `run` calls for a live project, newest first.
    ///
    /// # Errors
    /// Returns `NotFound` for an unknown project or `Unavailable` when the store cannot be read.
    pub fn list_runs(
        &self,
        project_id: String,
        limit: u32,
    ) -> Result<Vec<RunHistoryView>, SessionRuntimeError> {
        let store = self.store()?;
        let project = ProjectId::new(project_id);
        ensure_project(&store, &project)?;
        store
            .run_calls(&project, limit)
            .map_err(|_| SessionRuntimeError::Unavailable)
            .map(|rows| rows.into_iter().filter_map(run_view).collect())
    }
}

fn ensure_project(store: &Store, project: &ProjectId) -> Result<(), SessionRuntimeError> {
    store
        .load_project(project)
        .map_err(|_| SessionRuntimeError::Unavailable)?
        .map(|_| ())
        .ok_or(SessionRuntimeError::NotFound)
}

fn run_view(row: RunCallRow) -> Option<RunHistoryView> {
    let args: Value = serde_json::from_str(&row.args_json).ok()?;
    let command = args.get("command").and_then(Value::as_str)?;
    let parsed = row
        .result_text
        .as_deref()
        .map(|text| parse_run_result(text, row.result_is_error.unwrap_or(false)));
    let parsed_executor = parsed.as_ref().and_then(|run| run.executor.map(Into::into));
    let executor = parsed_executor.or_else(|| argument_executor(&args));
    let (outcome, exit_code, background, duration_ms) = parsed
        .as_ref()
        .map_or((RunOutcomeView::Unknown, None, false, None), parsed_outcome);
    Some(RunHistoryView {
        call_id: row.call_id.into(),
        session_id: row.session_id.into(),
        session_title: row.session_title,
        command: cut_command(command),
        executor,
        outcome,
        exit_code,
        duration_ms,
        started_at_ms: row.called_at_ms,
        background,
    })
}

fn parsed_outcome(run: &ParsedRun) -> (RunOutcomeView, Option<i32>, bool, Option<u64>) {
    let duration_ms = run.duration_ms;
    match &run.outcome {
        RunOutcome::Exited(code) => (
            if *code == 0 {
                RunOutcomeView::Succeeded
            } else {
                RunOutcomeView::Failed
            },
            Some(*code),
            false,
            duration_ms,
        ),
        RunOutcome::TimedOut => (RunOutcomeView::TimedOut, None, false, duration_ms),
        RunOutcome::Cancelled => (RunOutcomeView::Cancelled, None, false, duration_ms),
        RunOutcome::OutOfMemory => (RunOutcomeView::OutOfMemory, None, false, duration_ms),
        RunOutcome::Denied => (RunOutcomeView::Denied, None, false, duration_ms),
        RunOutcome::Background => (RunOutcomeView::Background, None, true, None),
        RunOutcome::Unknown => (RunOutcomeView::Unknown, None, false, duration_ms),
    }
}

fn argument_executor(args: &Value) -> Option<ExecutorView> {
    match args.get("executor").and_then(Value::as_str) {
        Some("sandbox") => Some(ExecutorView::Sandbox),
        Some("linux") => Some(ExecutorView::Linux),
        Some("ci") => Some(ExecutorView::Ci),
        _ => None,
    }
}

fn cut_command(command: &str) -> String {
    command.chars().take(500).collect()
}

#[cfg(test)]
#[path = "runs_tests.rs"]
mod tests;
