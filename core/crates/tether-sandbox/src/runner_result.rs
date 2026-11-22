//! Result classification and output pumping for sandbox runs.

use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;

use crate::output_pipe::BoundedPipe;
use crate::runner_support::{STOP_CANCELLED, STOP_TIMEOUT, sanitize_message};
use crate::{CappedOutput, OutputSink, RunOutcome, RunReport};

pub(crate) fn classify_error(error: &wasmtime::Error, reason: &AtomicU8) -> RunOutcome {
    match reason.load(Ordering::SeqCst) {
        STOP_CANCELLED => RunOutcome::Cancelled,
        STOP_TIMEOUT => RunOutcome::TimedOut,
        _ => {
            if let Some(exit) = error.downcast_ref::<wasmtime_wasi::I32Exit>() {
                return RunOutcome::Exited(exit.0);
            }
            let message = error.to_string();
            if message.to_ascii_lowercase().contains("memory") {
                RunOutcome::OutOfMemory
            } else {
                RunOutcome::Trapped(sanitize_message(&message))
            }
        }
    }
}

pub(crate) fn pump(
    stdout: &BoundedPipe,
    stderr: &BoundedPipe,
    stdout_len: &mut usize,
    stderr_len: &mut usize,
    output: &mut CappedOutput<'_>,
) {
    let fresh = stdout.since(*stdout_len);
    if !fresh.is_empty() {
        output.stdout(&fresh);
        *stdout_len += fresh.len();
    }
    let fresh = stderr.since(*stderr_len);
    if !fresh.is_empty() {
        output.stderr(&fresh);
        *stderr_len += fresh.len();
    }
    if stdout.discarded() || stderr.discarded() {
        output.mark_truncated();
    }
}

pub(crate) fn report(
    outcome: RunOutcome,
    started: Instant,
    output_bytes: usize,
    output_truncated: bool,
) -> RunReport {
    RunReport {
        outcome,
        elapsed: started.elapsed(),
        output_bytes,
        output_truncated,
    }
}
