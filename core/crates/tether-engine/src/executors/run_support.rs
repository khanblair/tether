//! Shared parsing, output, and report helpers for the run handler.

use serde_json::Value;
use tether_sandbox::{OutputSink, RunOutcome};
use tether_types::Executor;
use tokio::sync::mpsc;

use super::{OUTPUT_HEAD, OUTPUT_TAIL, PreparedCall, RunContext, RunReport, RunStatus};
use crate::tool_loop::PrepareContext;

pub(super) fn parse_executor(value: &Value) -> Result<Executor, String> {
    match value.as_str() {
        Some("sandbox") => Ok(Executor::Sandbox),
        Some("linux") => Ok(Executor::Linux),
        Some("ci") => Ok(Executor::Ci),
        _ => Err("executor must be sandbox, linux, or ci".into()),
    }
}

pub(super) fn parse_cwd(
    object: &serde_json::Map<String, Value>,
    context: &PrepareContext<'_>,
) -> Result<String, String> {
    let Some(value) = object.get("cwd") else {
        return Ok(String::new());
    };
    let cwd = value.as_str().ok_or("cwd must be a string")?;
    context
        .guard
        .resolve(cwd)
        .map_err(|error| error.to_string())?;
    Ok(cwd.to_owned())
}

pub(super) fn parse_timeout(
    object: &serde_json::Map<String, Value>,
) -> Result<Option<u64>, String> {
    let Some(value) = object.get("timeout_seconds") else {
        return Ok(None);
    };
    let timeout = value.as_u64().ok_or("timeout_seconds must be an integer")?;
    if !(1..=600).contains(&timeout) {
        return Err("timeout_seconds must be between 1 and 600".into());
    }
    Ok(Some(timeout))
}

pub(super) fn append_user_args(mut prefix: Vec<String>, prepared: &[String]) -> Vec<String> {
    let skip = prefix.len().min(prepared.len());
    prefix.extend(prepared.iter().skip(skip).cloned());
    prefix
}

pub(super) fn guest_cwd(cwd: &str) -> String {
    if cwd.is_empty() {
        "/workspace".into()
    } else {
        format!("/workspace/{cwd}")
    }
}

pub(super) fn executor_name(executor: Executor) -> &'static str {
    match executor {
        Executor::Sandbox => "sandbox",
        Executor::Linux => "linux",
        Executor::Ci => "ci",
    }
}

pub(super) fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

pub(super) fn format_result(executor: Executor, call: &PreparedCall, report: &RunReport) -> String {
    let status = match report.status {
        RunStatus::Exited(code) => format!("exit code {code}"),
        RunStatus::TimedOut => call
            .payload
            .get("_timeout")
            .and_then(Value::as_u64)
            .map_or_else(
                || "timed out".to_owned(),
                |seconds| format!("timed out after {seconds}s"),
            ),
        RunStatus::Cancelled => "cancelled".into(),
        RunStatus::OutOfMemory => "out of memory".into(),
    };
    let output = trim_output(&report.output);
    let truncation = if report.output_truncated || output.1 > 0 {
        format!("\noutput truncated: {} characters omitted", output.1)
    } else {
        String::new()
    };
    let reason = call
        .payload
        .get("_reason")
        .and_then(Value::as_str)
        .unwrap_or("executor selected");
    format!(
        "executor: {}\nreason: {reason}\n{status}\nduration: {} ms{truncation}\n{}",
        executor_name(executor),
        report.duration.as_millis(),
        output.0
    )
}

pub(super) fn trim_output(output: &str) -> (String, usize) {
    let chars: Vec<char> = output.chars().collect();
    if chars.len() <= OUTPUT_HEAD + OUTPUT_TAIL {
        return (output.to_owned(), 0);
    }
    let omitted = chars.len() - OUTPUT_HEAD - OUTPUT_TAIL;
    let head: String = chars[..OUTPUT_HEAD].iter().collect();
    let tail: String = chars[chars.len() - OUTPUT_TAIL..].iter().collect();
    (
        format!("{head}\n[... {omitted} characters omitted ...]\n{tail}"),
        omitted,
    )
}

/// Collects executor output and forwards each chunk to the async loop.
pub(super) struct OutputForwarder {
    tx: mpsc::UnboundedSender<String>,
    output: Vec<u8>,
}

impl OutputSink for OutputForwarder {
    fn stdout(&mut self, bytes: &[u8]) {
        self.output.extend_from_slice(bytes);
        drop(self.tx.send(String::from_utf8_lossy(bytes).into_owned()));
    }

    fn stderr(&mut self, bytes: &[u8]) {
        self.output.extend_from_slice(bytes);
        drop(self.tx.send(String::from_utf8_lossy(bytes).into_owned()));
    }
}

pub(super) async fn run_blocking<T, E, F>(
    run: F,
    context: &RunContext<'_>,
) -> Result<RunReport, String>
where
    F: FnOnce(&mut OutputForwarder) -> Result<T, E> + Send + 'static,
    E: std::fmt::Debug + Send + 'static,
    T: IntoRunReport + Send + 'static,
{
    let (tx, mut rx) = mpsc::unbounded_channel();
    let join = tokio::task::spawn_blocking(move || {
        let mut sink = OutputForwarder {
            tx,
            output: Vec::new(),
        };
        let result = run(&mut sink);
        (result, sink.output)
    });
    while let Some(chunk) = rx.recv().await {
        if let Some(sink) = context.sink {
            sink.on_event(tether_types::CoreEvent::ToolOutput {
                session_id: context.session_id.clone(),
                tool_call_id: context.tool_call_id.clone(),
                chunk,
            });
        }
    }
    let (result, output) = join
        .await
        .map_err(|_| "executor thread stopped".to_owned())?;
    let raw = result.map_err(|error| format!("{error:?}"))?;
    Ok(report_from_runner(raw, &output))
}

fn report_from_runner<T: IntoRunReport>(report: T, output: &[u8]) -> RunReport {
    let mut converted = report.into_run_report();
    converted.output = String::from_utf8_lossy(output).into_owned();
    converted
}

/// Converts a native runner report to the common run report.
pub(super) trait IntoRunReport {
    fn into_run_report(self) -> RunReport;
}

impl IntoRunReport for tether_sandbox::RunReport {
    fn into_run_report(self) -> RunReport {
        RunReport {
            status: match self.outcome {
                RunOutcome::Exited(code) => RunStatus::Exited(code),
                RunOutcome::TimedOut => RunStatus::TimedOut,
                RunOutcome::Cancelled => RunStatus::Cancelled,
                RunOutcome::OutOfMemory => RunStatus::OutOfMemory,
                RunOutcome::Trapped(_) => RunStatus::Exited(1),
            },
            output: String::new(),
            duration: self.elapsed,
            output_truncated: self.output_truncated,
        }
    }
}

impl IntoRunReport for tether_linux::LinuxRunReport {
    fn into_run_report(self) -> RunReport {
        RunReport {
            status: match self.outcome {
                tether_linux::RunOutcome::Exited(code) | tether_linux::RunOutcome::Killed(code) => {
                    RunStatus::Exited(code)
                }
                tether_linux::RunOutcome::TimedOut => RunStatus::TimedOut,
                tether_linux::RunOutcome::Cancelled => RunStatus::Cancelled,
            },
            output: String::new(),
            duration: self.elapsed,
            output_truncated: self.output_truncated,
        }
    }
}
