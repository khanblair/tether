use std::io::Read;
use std::process::{Child, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use tether_sandbox::{CappedOutput, OutputSink};
use tokio_util::sync::CancellationToken;

use crate::LinuxError;
use crate::proot::{ProotConfig, SessionKind};

/// A command expressed as a shell string or an argument vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunCommand {
    /// Runs `sh -c` inside the guest.
    Shell(String),
    /// Runs the supplied guest argv directly.
    Argv(Vec<String>),
}

/// A request for one non-interactive Linux command.
#[derive(Debug, Clone)]
pub struct RunRequest {
    /// The guest command.
    pub command: RunCommand,
    /// Absolute guest working directory.
    pub cwd: String,
    /// Additional safe guest environment variables.
    pub extra_env: Vec<(String, String)>,
    /// Optional wall time limit. The default is 120 seconds.
    pub timeout: Option<Duration>,
    /// Combined stdout and stderr cap.
    pub output_cap_bytes: usize,
}

/// Alias retained for callers that name the request a run specification.
pub type RunSpec = RunRequest;

/// Compatibility alias for callers that use the request-qualified command name.
pub type RunRequestCommand = RunCommand;

impl Default for RunRequest {
    fn default() -> Self {
        Self {
            command: RunCommand::Argv(Vec::new()),
            cwd: "/workspace".into(),
            extra_env: Vec::new(),
            timeout: None,
            output_cap_bytes: 1024 * 1024,
        }
    }
}

/// The process outcome after output streaming finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    /// The guest process returned an exit code.
    Exited(i32),
    /// The configured time limit killed the process.
    TimedOut,
    /// Cancellation killed the process.
    Cancelled,
    /// The process was terminated by a signal.
    Killed(i32),
}

/// The result of a Linux command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxRunReport {
    /// The process outcome.
    pub outcome: RunOutcome,
    /// Wall time from spawn to wait.
    pub elapsed: Duration,
    /// Bytes forwarded through the output cap.
    pub output_bytes: usize,
    /// Whether bytes were discarded by the output cap.
    pub output_truncated: bool,
}

/// Runs PRoot commands with bounded, arrival-ordered output.
#[derive(Debug, Clone)]
pub struct LinuxRunner {
    config: ProotConfig,
}

impl LinuxRunner {
    /// Creates a runner for one PRoot configuration.
    #[must_use]
    pub fn new(config: ProotConfig) -> Self {
        Self { config }
    }

    /// Runs one command until exit, timeout, or cancellation.
    ///
    /// # Errors
    /// Returns a not-ready, validation, spawn, stream, or wait error.
    pub fn run(
        &self,
        request: &RunRequest,
        sink: &mut dyn OutputSink,
        cancel: &CancellationToken,
    ) -> Result<LinuxRunReport, LinuxError> {
        self.ensure_ready()?;
        let argv = request_argv(request)?;
        let mut command = self.config.command(
            &argv,
            &request.extra_env,
            &request.cwd,
            SessionKind::Command,
        )?;
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|_| LinuxError::Process("could not start PRoot".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LinuxError::Process("missing standard output".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| LinuxError::Process("missing standard error".into()))?;
        let (sender, receiver) = mpsc::channel();
        spawn_reader(stdout, false, sender.clone());
        spawn_reader(stderr, true, sender);
        stream_process(&mut child, &receiver, request, sink, cancel)
    }

    fn ensure_ready(&self) -> Result<(), LinuxError> {
        if !self.config.proot_path.is_file() {
            return Err(LinuxError::NotReady("PRoot is unavailable".into()));
        }
        let rootfs = self.config.linux_root.join("rootfs/alpine");
        let usable = std::fs::symlink_metadata(rootfs)
            .is_ok_and(|metadata| !metadata.file_type().is_symlink() && metadata.is_dir());
        if !usable {
            return Err(LinuxError::NotReady(
                "the Alpine root filesystem is unavailable".into(),
            ));
        }
        Ok(())
    }
}

fn request_argv(request: &RunRequest) -> Result<Vec<String>, LinuxError> {
    let argv = match &request.command {
        RunCommand::Shell(command) => vec!["sh".into(), "-c".into(), command.clone()],
        RunCommand::Argv(argv) => argv.clone(),
    };
    if argv.is_empty() {
        Err(LinuxError::Path("guest command is empty".into()))
    } else {
        Ok(argv)
    }
}

fn spawn_reader<R: Read + Send + 'static>(
    mut reader: R,
    stderr: bool,
    sender: Sender<StreamChunk>,
) {
    thread::spawn(move || {
        let mut buffer = [0_u8; 16 * 1024];
        while read_and_send(&mut reader, &mut buffer, stderr, &sender) {}
        drop(sender.send(StreamChunk {
            stderr,
            bytes: Vec::new(),
        }));
    });
}

#[derive(Debug)]
struct StreamChunk {
    stderr: bool,
    bytes: Vec<u8>,
}

fn stream_process(
    child: &mut Child,
    receiver: &Receiver<StreamChunk>,
    request: &RunRequest,
    sink: &mut dyn OutputSink,
    cancel: &CancellationToken,
) -> Result<LinuxRunReport, LinuxError> {
    let started = Instant::now();
    let timeout = request.timeout.unwrap_or(Duration::from_secs(120));
    let deadline = started + timeout;
    let mut capped = CappedOutput::new(sink, request.output_cap_bytes);
    let mut closed = 0_u8;
    let mut reason = None;
    while reason.is_none() {
        if let Some(stop) = stop_reason(child, cancel, deadline) {
            reason = Some(stop);
        } else {
            closed = receive_chunk(receiver, &mut capped, closed);
            reason = poll_if_closed(child, closed)?;
        }
    }
    let outcome = match reason {
        Some(RunOutcome::Exited(code)) => RunOutcome::Exited(code),
        Some(outcome) => {
            drop(child.wait());
            outcome
        }
        None => RunOutcome::Exited(
            child
                .wait()
                .map_err(|_| LinuxError::Process("could not wait for PRoot".into()))?
                .code()
                .unwrap_or(1),
        ),
    };
    Ok(LinuxRunReport {
        outcome,
        elapsed: started.elapsed(),
        output_bytes: capped.sent(),
        output_truncated: capped.truncated(),
    })
}

fn stop_reason(
    child: &mut Child,
    cancel: &CancellationToken,
    deadline: Instant,
) -> Option<RunOutcome> {
    if cancel.is_cancelled() {
        kill_child(child);
        Some(RunOutcome::Cancelled)
    } else if Instant::now() >= deadline {
        kill_child(child);
        Some(RunOutcome::TimedOut)
    } else {
        None
    }
}

fn receive_chunk(
    receiver: &Receiver<StreamChunk>,
    capped: &mut CappedOutput<'_>,
    closed: u8,
) -> u8 {
    match receiver.recv_timeout(Duration::from_millis(10)) {
        Ok(chunk) if chunk.bytes.is_empty() => closed.saturating_add(1),
        Ok(chunk) if chunk.stderr => {
            capped.stderr(&chunk.bytes);
            closed
        }
        Ok(chunk) => {
            capped.stdout(&chunk.bytes);
            closed
        }
        Err(RecvTimeoutError::Disconnected | RecvTimeoutError::Timeout) => closed,
    }
}

fn send_chunk(sender: &Sender<StreamChunk>, stderr: bool, bytes: &[u8]) -> bool {
    sender
        .send(StreamChunk {
            stderr,
            bytes: bytes.to_vec(),
        })
        .is_ok()
}

fn read_and_send<R: Read>(
    reader: &mut R,
    buffer: &mut [u8],
    stderr: bool,
    sender: &Sender<StreamChunk>,
) -> bool {
    match reader.read(buffer) {
        Ok(0) | Err(_) => false,
        Ok(count) => send_chunk(sender, stderr, &buffer[..count]),
    }
}

fn poll_outcome(child: &mut Child) -> Result<Option<RunOutcome>, LinuxError> {
    child
        .try_wait()
        .map(|status| status.map(status_outcome))
        .map_err(|_| LinuxError::Process("could not poll PRoot".into()))
}

fn poll_if_closed(child: &mut Child, closed: u8) -> Result<Option<RunOutcome>, LinuxError> {
    if closed >= 2 {
        poll_outcome(child)
    } else {
        Ok(None)
    }
}

fn kill_child(child: &mut Child) {
    drop(child.kill());
}

fn status_outcome(status: std::process::ExitStatus) -> RunOutcome {
    if let Some(code) = status.code() {
        return RunOutcome::Exited(code);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        RunOutcome::Killed(status.signal().unwrap_or(0))
    }
    #[cfg(not(unix))]
    RunOutcome::Killed(0)
}
