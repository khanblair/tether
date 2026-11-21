//! Result classification and output pumping for sandbox runs.

use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;

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
    stdout: &wasmtime_wasi::p2::pipe::MemoryOutputPipe,
    stderr: &wasmtime_wasi::p2::pipe::MemoryOutputPipe,
    stdout_len: &mut usize,
    stderr_len: &mut usize,
    output: &mut CappedOutput<'_>,
) {
    let stdout_bytes = stdout.contents();
    if stdout_bytes.len() > *stdout_len {
        output.stdout(&stdout_bytes[*stdout_len..]);
        *stdout_len = stdout_bytes.len();
    }
    let stderr_bytes = stderr.contents();
    if stderr_bytes.len() > *stderr_len {
        output.stderr(&stderr_bytes[*stderr_len..]);
        *stderr_len = stderr_bytes.len();
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
