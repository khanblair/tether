//! Blocking WASI preview 1 command runner.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;
use wasmtime::{Linker, Store, UpdateDeadline};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::{FsPerms, WasiCtxBuilder};

use crate::runner_result::{classify_error, pump, report};
use crate::runner_support::{
    STOP_CANCELLED, STOP_NONE, STOP_TIMEOUT, create_run_tmp, spawn_epoch_thread,
};
use crate::runner_validation::validate_spec;
use crate::{CappedOutput, ModuleCache, OutputSink, RunLimits, SandboxError, WasiContextSpec};

/// A host folder mounted read-only at an absolute guest path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtraMount {
    /// Host folder inside the verified pack root.
    pub host: PathBuf,
    /// Absolute guest mount path.
    pub guest: String,
}

/// A compiled WASI preview 1 command and its per-run settings.
pub struct RunSpec {
    /// The compiled command module exporting `_start`.
    pub module: Arc<wasmtime::Module>,
    /// Command arguments, including the program name at index zero.
    pub argv: Vec<String>,
    /// Workspace and fixed guest environment.
    pub workspace: WasiContextSpec,
    /// Absolute guest working directory under `/workspace`.
    pub cwd: String,
    /// Canonical folder that may contain support mounts.
    pub pack_root: PathBuf,
    /// Read-only support folders visible to the guest.
    pub extra_mounts: Vec<ExtraMount>,
    /// Additional pack environment variables.
    pub extra_env: BTreeMap<String, String>,
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
    _cache: Arc<ModuleCache>,
}

impl SandboxRunner {
    /// Creates a runner backed by a module cache.
    #[must_use]
    pub fn new(cache: Arc<ModuleCache>) -> Self {
        Self { _cache: cache }
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
        validate_spec(spec)?;
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
            cwd: spec.cwd.clone(),
            tmp: tmp.to_path_buf(),
            extra_mounts: spec.extra_mounts.clone(),
            extra_env: spec.extra_env.clone(),
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
    cwd: String,
    extra_mounts: Vec<ExtraMount>,
    extra_env: BTreeMap<String, String>,
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
            cwd: &input.cwd,
            extra_mounts: &input.extra_mounts,
            extra_env: &input.extra_env,
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
    cwd: &'a str,
    extra_mounts: &'a [ExtraMount],
    extra_env: &'a BTreeMap<String, String>,
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
        cwd,
        extra_mounts,
        extra_env,
        limits,
        stdout,
        stderr,
        reason,
    } = args;
    let wasi = build_wasi(WasiInputs {
        argv,
        workspace,
        tmp,
        cwd,
        stdout,
        stderr,
        extra_mounts,
        extra_env,
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
    cwd: &'a str,
    stdout: wasmtime_wasi::p2::pipe::MemoryOutputPipe,
    stderr: wasmtime_wasi::p2::pipe::MemoryOutputPipe,
    extra_mounts: &'a [ExtraMount],
    extra_env: &'a BTreeMap<String, String>,
}

fn build_wasi(input: WasiInputs<'_>) -> Result<WasiP1Ctx, SandboxError> {
    let mut builder = WasiCtxBuilder::new();
    let mut env = input.workspace.env.clone();
    env.extend(input.extra_env.clone());
    builder.args(input.argv).envs(
        &env.iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>(),
    );
    builder.initial_cwd(input.cwd);
    builder.stdout(input.stdout).stderr(input.stderr);
    builder
        .preopened_dir(&input.workspace.workspace, "/workspace", FsPerms::ReadWrite)
        .map_err(|_| SandboxError::Runtime("workspace mount failed".into()))?;
    builder
        .preopened_dir(input.tmp, "/tmp", FsPerms::ReadWrite)
        .map_err(|_| SandboxError::Runtime("temporary mount failed".into()))?;
    for mount in input.extra_mounts {
        builder
            .preopened_dir(&mount.host, &mount.guest, FsPerms::ReadOnly)
            .map_err(|_| SandboxError::Runtime("support mount failed".into()))?;
    }
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

#[cfg(test)]
mod tests {
    use crate::runner_support::sanitize_message;

    #[test]
    fn sanitized_messages_hide_absolute_paths() {
        assert_eq!(
            sanitize_message("open /private/tmp/secret failed"),
            "open <host-path> failed"
        );
    }
}
