//! Blocking WASI preview 1 command runner.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;
use wasmtime::{Linker, Store, UpdateDeadline};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::{FsPerms, I32Exit, WasiCtxBuilder};

use crate::runner_support::{
    STOP_CANCELLED, STOP_NONE, STOP_TIMEOUT, create_run_tmp, sanitize_message, spawn_epoch_thread,
};
use crate::{CappedOutput, ModuleCache, OutputSink, RunLimits, SandboxError, WasiContextSpec};

/// A compiled WASI preview 1 command and its per-run settings.
pub struct RunSpec {
    /// The compiled command module exporting `_start`.
    pub module: Arc<wasmtime::Module>,
    /// Command arguments, including the program name at index zero.
    pub argv: Vec<String>,
    /// Workspace and fixed guest environment.
    pub workspace: WasiContextSpec,
    /// Memory, time, and output limits.
    pub limits: RunLimits,
}

/// The result of a sandbox command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    /// The command exited, including through WASI `proc_exit`.
    Exited(i32),
    /// The epoch deadline was reached.
    TimedOut,
    /// The caller cancelled the command.
    Cancelled,
    /// The guest exceeded its linear memory limit.
    OutOfMemory,
    /// The guest trapped for another reason.
    Trapped(String),
}

/// Output and timing information for one sandbox command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    /// The command's final outcome.
    pub outcome: RunOutcome,
    /// Wall time spent in the runner.
    pub elapsed: Duration,
    /// Bytes forwarded after applying the shared output cap.
    pub output_bytes: usize,
    /// Whether any guest output was discarded.
    pub output_truncated: bool,
}

/// Runs compiled WASI commands on blocking threads.
pub struct SandboxRunner {
    cache: Arc<ModuleCache>,
}

impl SandboxRunner {
    /// Creates a runner backed by a module cache.
    #[must_use]
    pub fn new(cache: Arc<ModuleCache>) -> Self {
        Self { cache }
    }

    /// Runs one command and forwards output while it executes.
    ///
    /// # Errors
    /// Returns an engine, setup, linker, or invocation error. Guest traps are
    /// returned in the report instead.
    pub fn run(
        &self,
        spec: &RunSpec,
        sink: &mut dyn OutputSink,
        cancel: &CancellationToken,
    ) -> Result<RunReport, SandboxError> {
        let started = Instant::now();
        if cancel.is_cancelled() {
            return Ok(report(RunOutcome::Cancelled, started, 0, false));
        }
        let tmp = create_run_tmp()
            .map_err(|_| SandboxError::Runtime("temporary directory setup failed".into()))?;
        let result = Self::execute(spec, &tmp, sink, cancel, started);
        let remove_result = fs::remove_dir_all(&tmp);
        if result.is_ok() && remove_result.is_err() {
            return Err(SandboxError::Runtime(
                "temporary directory cleanup failed".into(),
            ));
        }
        result
    }

    fn execute(
        spec: &RunSpec,
        tmp: &Path,
        sink: &mut dyn OutputSink,
        cancel: &CancellationToken,
        started: Instant,
    ) -> Result<RunReport, SandboxError> {
        let stdout = wasmtime_wasi::p2::pipe::MemoryOutputPipe::new(usize::MAX);
        let stderr = wasmtime_wasi::p2::pipe::MemoryOutputPipe::new(usize::MAX);
        let stdout_view = stdout.clone();
        let stderr_view = stderr.clone();
        let (result_tx, result_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let reason = Arc::new(AtomicU8::new(STOP_NONE));
        let epoch = spec.module.engine().clone();
        let ticker = spawn_epoch_thread(
            &epoch,
            spec.limits,
            cancel.clone(),
            Arc::clone(&stop),
            Arc::clone(&reason),
        );
        let execution = spawn_execution(ExecutionInput {
            module: Arc::clone(&spec.module),
            argv: spec.argv.clone(),
            workspace: spec.workspace.clone(),
            tmp: tmp.to_path_buf(),
            limits: spec.limits,
            stdout,
            stderr,
            reason: Arc::clone(&reason),
            result_tx,
        });
        let mut capped = CappedOutput::new(sink, spec.limits.output_bytes);
        let (invocation_result, mut stdout_len, mut stderr_len) =
            collect_output(&stdout_view, &stderr_view, &result_rx, &mut capped);
        pump(
            &stdout_view,
            &stderr_view,
            &mut stdout_len,
            &mut stderr_len,
            &mut capped,
        );
        stop.store(true, Ordering::SeqCst);
        drop(ticker.join());
        drop(execution.join());
        let invocation = invocation_result?;
        Ok(report(
            invocation,
            started,
            capped.sent(),
            capped.truncated(),
        ))
    }
}

struct ExecutionInput {
    module: Arc<wasmtime::Module>,
    argv: Vec<String>,
    workspace: WasiContextSpec,
    tmp: std::path::PathBuf,
    limits: RunLimits,
    stdout: wasmtime_wasi::p2::pipe::MemoryOutputPipe,
    stderr: wasmtime_wasi::p2::pipe::MemoryOutputPipe,
    reason: Arc<AtomicU8>,
    result_tx: mpsc::Sender<Result<RunOutcome, SandboxError>>,
}

fn spawn_execution(input: ExecutionInput) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let result = invoke(InvokeArgs {
            module: &input.module,
            argv: &input.argv,
            workspace: &input.workspace,
            tmp: &input.tmp,
            limits: input.limits,
            stdout: input.stdout,
            stderr: input.stderr,
            reason: &input.reason,
        });
        drop(input.result_tx.send(result));
    })
}

fn collect_output(
    stdout: &wasmtime_wasi::p2::pipe::MemoryOutputPipe,
    stderr: &wasmtime_wasi::p2::pipe::MemoryOutputPipe,
    result_rx: &mpsc::Receiver<Result<RunOutcome, SandboxError>>,
    output: &mut CappedOutput<'_>,
) -> (Result<RunOutcome, SandboxError>, usize, usize) {
    let mut stdout_len = 0;
    let mut stderr_len = 0;
    loop {
        pump(stdout, stderr, &mut stdout_len, &mut stderr_len, output);
        match result_rx.try_recv() {
            Ok(result) => return (result, stdout_len, stderr_len),
            Err(mpsc::TryRecvError::Empty) => thread::sleep(Duration::from_millis(1)),
            Err(mpsc::TryRecvError::Disconnected) => {
                return (
                    Err(SandboxError::Runtime(
                        "runner thread stopped unexpectedly".into(),
                    )),
                    stdout_len,
                    stderr_len,
                );
            }
        }
    }
}

struct InvokeArgs<'a> {
    module: &'a wasmtime::Module,
    argv: &'a [String],
    workspace: &'a WasiContextSpec,
    tmp: &'a Path,
    limits: RunLimits,
    stdout: wasmtime_wasi::p2::pipe::MemoryOutputPipe,
    stderr: wasmtime_wasi::p2::pipe::MemoryOutputPipe,
    reason: &'a Arc<AtomicU8>,
}

fn invoke(args: InvokeArgs<'_>) -> Result<RunOutcome, SandboxError> {
    let InvokeArgs {
        module,
        argv,
        workspace,
        tmp,
        limits,
        stdout,
        stderr,
        reason,
    } = args;
    let wasi = build_wasi(WasiInputs {
        argv,
        workspace,
        tmp,
        stdout,
        stderr,
    })?;
    let linker = make_linker(module)?;
    let mut store = make_store(module, wasi, limits, reason);
    let instance = match linker.instantiate(&mut store, module) {
        Ok(instance) => instance,
        Err(error) => return Ok(classify_error(&error, reason)),
    };
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|_| SandboxError::Runtime("WASI command entry point missing".into()))?;
    match start.call(&mut store, ()) {
        Ok(()) => Ok(RunOutcome::Exited(0)),
        Err(error) => Ok(classify_error(&error, reason)),
    }
}

struct WasiInputs<'a> {
    argv: &'a [String],
    workspace: &'a WasiContextSpec,
    tmp: &'a Path,
    stdout: wasmtime_wasi::p2::pipe::MemoryOutputPipe,
    stderr: wasmtime_wasi::p2::pipe::MemoryOutputPipe,
}

fn build_wasi(input: WasiInputs<'_>) -> Result<WasiP1Ctx, SandboxError> {
    let mut builder = WasiCtxBuilder::new();
    builder.args(input.argv).envs(
        &input
            .workspace
            .env
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>(),
    );
    builder.initial_cwd("/workspace");
    builder.stdout(input.stdout).stderr(input.stderr);
    builder
        .preopened_dir(&input.workspace.workspace, "/workspace", FsPerms::ReadWrite)
        .map_err(|_| SandboxError::Runtime("workspace mount failed".into()))?;
    builder
        .preopened_dir(input.tmp, "/tmp", FsPerms::ReadWrite)
        .map_err(|_| SandboxError::Runtime("temporary mount failed".into()))?;
    Ok(builder.build_p1())
}

fn make_linker(module: &wasmtime::Module) -> Result<Linker<WasiState>, SandboxError> {
    let mut linker: Linker<WasiState> = Linker::new(module.engine());
    p1::add_to_linker_sync(&mut linker, |state| &mut state.wasi)
        .map_err(|_| SandboxError::Runtime("WASI linker setup failed".into()))?;
    Ok(linker)
}

fn make_store(
    module: &wasmtime::Module,
    wasi: WasiP1Ctx,
    limits: RunLimits,
    reason: &Arc<AtomicU8>,
) -> Store<WasiState> {
    let mut store = Store::new(
        module.engine(),
        WasiState {
            wasi,
            limits: limits.store_limits(),
        },
    );
    store.limiter(|state| &mut state.limits);
    store.set_epoch_deadline(1);
    let callback_reason = Arc::clone(reason);
    store.epoch_deadline_callback(move |_| match callback_reason.load(Ordering::SeqCst) {
        STOP_CANCELLED | STOP_TIMEOUT => Ok(UpdateDeadline::Interrupt),
        _ => Ok(UpdateDeadline::Continue(1)),
    });
    store
}

struct WasiState {
    wasi: WasiP1Ctx,
    limits: wasmtime::StoreLimits,
}

fn classify_error(error: &wasmtime::Error, reason: &AtomicU8) -> RunOutcome {
    match reason.load(Ordering::SeqCst) {
        STOP_CANCELLED => RunOutcome::Cancelled,
        STOP_TIMEOUT => RunOutcome::TimedOut,
        _ => {
            if let Some(exit) = error.downcast_ref::<I32Exit>() {
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

fn pump(
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

fn report(
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

impl std::fmt::Debug for SandboxRunner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SandboxRunner")
            .field("cache", &self.cache)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitized_messages_hide_absolute_paths() {
        assert_eq!(
            sanitize_message("open /private/tmp/secret failed"),
            "open <host-path> failed"
        );
    }
}
