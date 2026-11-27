//! Shared parsing, output, and report helpers for the run handler.

use serde_json::Value;
use tether_sandbox::{OutputSink, RunOutcome as SandboxRunOutcome};
use tether_types::Executor;
use tokio::sync::mpsc;

use super::{OUTPUT_HEAD, OUTPUT_TAIL, PreparedCall, RunContext, RunReport};
use crate::executors::RunStatus;
use crate::path_utils::lexical_path;
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
    let path = lexical_path(cwd).map_err(|_| "cwd must stay inside the workspace")?;
    if path.as_os_str().is_empty() {
        return Ok(String::new());
    }
    let normalized = path.to_string_lossy().replace('\\', "/");
    context
        .guard
        .resolve(&normalized)
        .map_err(|_| "cwd must stay inside the workspace")?;
    Ok(normalized)
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

/// Joins a pack's launch prefix (the program name plus the pack's own arguments) with the
/// arguments the user typed after the program name.
pub(super) fn append_user_args(mut prefix: Vec<String>, prepared: &[String]) -> Vec<String> {
    prefix.extend(prepared.iter().skip(1).cloned());
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

const EXIT_CODE_PREFIX: &str = "exit code ";
const TIMED_OUT: &str = "timed out";
const TIMED_OUT_AFTER_PREFIX: &str = "timed out after ";
const CANCELLED: &str = "cancelled";
const OUT_OF_MEMORY: &str = "out of memory";
const CANCELLED_BEFORE_EXECUTION: &str = "tool cancelled before execution";
const DENIED_BY_USER: &str = "denied by the user";

/// The outcome read from a saved `run` result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    /// The command returned an exit code.
    Exited(i32),
    /// The executor stopped at its time limit.
    TimedOut,
    /// The caller cancelled the command.
    Cancelled,
    /// The command exceeded its memory limit.
    OutOfMemory,
    /// The user denied the call before execution.
    Denied,
    /// Linux started a background process.
    Background,
    /// The saved text did not match a known result shape.
    Unknown,
}

/// The parsed executor, outcome, and duration of a saved `run` result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRun {
    /// Executor named by the result, or Linux for a background result.
    pub executor: Option<Executor>,
    /// Parsed terminal outcome.
    pub outcome: RunOutcome,
    /// Duration in milliseconds for a foreground result.
    pub duration_ms: Option<u64>,
}

/// Parses the exact result text written by the `run` tool and its cancellation paths.
#[must_use]
pub fn parse_run_result(text: &str, is_error: bool) -> ParsedRun {
    if is_background_result(text) {
        return ParsedRun {
            executor: Some(Executor::Linux),
            outcome: RunOutcome::Background,
            duration_ms: None,
        };
    }
    if is_error && text == CANCELLED_BEFORE_EXECUTION {
        return parsed_without_executor(RunOutcome::Cancelled);
    }
    if is_error && is_denied_result(text) {
        return parsed_without_executor(RunOutcome::Denied);
    }
    parse_foreground_result(text).unwrap_or_else(|| parsed_without_executor(RunOutcome::Unknown))
}

fn status_text(status: RunStatus, call: &PreparedCall) -> String {
    match status {
        RunStatus::Exited(code) => format!("{EXIT_CODE_PREFIX}{code}"),
        RunStatus::TimedOut => call
            .payload
            .get("_timeout")
            .and_then(Value::as_u64)
            .map_or_else(
                || TIMED_OUT.to_owned(),
                |seconds| format!("{TIMED_OUT_AFTER_PREFIX}{seconds}s"),
            ),
        RunStatus::Cancelled => CANCELLED.to_owned(),
        RunStatus::OutOfMemory => OUT_OF_MEMORY.to_owned(),
    }
}

fn parse_foreground_result(text: &str) -> Option<ParsedRun> {
    let mut lines = text.split('\n');
    let executor = parse_executor_name(lines.next()?.strip_prefix("executor: ")?)?;
    lines.next()?.strip_prefix("reason: ")?;
    let outcome = parse_status(lines.next()?)?;
    let duration_ms = parse_duration(lines.next()?)?;
    Some(ParsedRun {
        executor: Some(executor),
        outcome,
        duration_ms: Some(duration_ms),
    })
}

fn parse_status(line: &str) -> Option<RunOutcome> {
    if let Some(code) = line.strip_prefix(EXIT_CODE_PREFIX) {
        return code.parse().ok().map(RunOutcome::Exited);
    }
    if line == TIMED_OUT {
        return Some(RunOutcome::TimedOut);
    }
    if line
        .strip_prefix(TIMED_OUT_AFTER_PREFIX)
        .and_then(|seconds| seconds.strip_suffix('s'))
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .is_some()
    {
        return Some(RunOutcome::TimedOut);
    }
    match line {
        CANCELLED => Some(RunOutcome::Cancelled),
        OUT_OF_MEMORY => Some(RunOutcome::OutOfMemory),
        _ => None,
    }
}

fn parse_duration(line: &str) -> Option<u64> {
    line.strip_prefix("duration: ")?
        .strip_suffix(" ms")?
        .parse()
        .ok()
}

fn parse_executor_name(name: &str) -> Option<Executor> {
    match name {
        "sandbox" => Some(Executor::Sandbox),
        "linux" => Some(Executor::Linux),
        "ci" => Some(Executor::Ci),
        _ => None,
    }
}

fn parsed_without_executor(outcome: RunOutcome) -> ParsedRun {
    ParsedRun {
        executor: None,
        outcome,
        duration_ms: None,
    }
}

fn is_background_result(text: &str) -> bool {
    text.starts_with("process: ") && text.contains("\noutput:\n")
}

fn is_denied_result(text: &str) -> bool {
    text.strip_prefix(DENIED_BY_USER)
        .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with(':'))
}

pub(super) fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

pub(super) fn format_result(executor: Executor, call: &PreparedCall, report: &RunReport) -> String {
    let status = status_text(report.status, call);
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
                SandboxRunOutcome::Exited(code) => RunStatus::Exited(code),
                SandboxRunOutcome::TimedOut => RunStatus::TimedOut,
                SandboxRunOutcome::Cancelled => RunStatus::Cancelled,
                SandboxRunOutcome::OutOfMemory => RunStatus::OutOfMemory,
                SandboxRunOutcome::Trapped(_) => RunStatus::Exited(1),
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
