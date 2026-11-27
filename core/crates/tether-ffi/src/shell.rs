//! UniFFI access to one interactive Linux shell.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;

use tether_linux::{LinuxError, LinuxShell, ProotConfig, PtySize};

use crate::ExecutorRuntimeError;

const MAX_OUTPUT_CHUNK: usize = 8 * 1024;

/// Receives raw terminal output and the shell's exit status.
///
/// Calls happen on a Rust reader thread. Implementations must be thread-safe and must return
/// promptly so a guest that writes continuously cannot hold the reader thread indefinitely.
#[uniffi::export(with_foreign)]
pub trait LinuxShellSink: Send + Sync {
    /// Receives one output chunk of at most 8 KiB.
    fn on_output(&self, bytes: Vec<u8>);
    /// Receives the shell exit code exactly once.
    fn on_exit(&self, code: i32);
}

/// Tracks the one shell allowed by one executor runtime.
pub(crate) struct ShellSlot {
    next_id: AtomicU64,
    active_id: Mutex<Option<u64>>,
}

impl ShellSlot {
    /// Creates an empty shell slot.
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            next_id: AtomicU64::new(1),
            active_id: Mutex::new(None),
        })
    }

    pub(crate) fn acquire(self: &Arc<Self>) -> Result<Arc<ShellLease>, ExecutorRuntimeError> {
        let mut active = self
            .active_id
            .lock()
            .map_err(|_| ExecutorRuntimeError::Unavailable)?;
        if active.is_some() {
            return Err(ExecutorRuntimeError::LinuxShellAlreadyOpen);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        *active = Some(id);
        Ok(Arc::new(ShellLease {
            slot: Arc::clone(self),
            id,
        }))
    }

    fn release(&self, id: u64) {
        if let Ok(mut active) = self.active_id.lock()
            && *active == Some(id)
        {
            *active = None;
        }
    }
}

pub(crate) struct ShellLease {
    slot: Arc<ShellSlot>,
    id: u64,
}

impl Drop for ShellLease {
    fn drop(&mut self) {
        self.slot.release(self.id);
    }
}

/// Owns an interactive shell in the Alpine guest.
#[derive(uniffi::Object)]
pub struct LinuxShellSession {
    shell: Arc<Mutex<Option<LinuxShell>>>,
    closed: AtomicBool,
}

impl LinuxShellSession {
    pub(crate) fn open(
        config: &ProotConfig,
        rows: u16,
        cols: u16,
        sink: Arc<dyn LinuxShellSink>,
        lease: Arc<ShellLease>,
    ) -> Result<Arc<Self>, ExecutorRuntimeError> {
        let mut shell = LinuxShell::spawn(
            config,
            PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
        .map_err(map_linux_error)?;
        let output = shell.output();
        let shell = Arc::new(Mutex::new(Some(shell)));
        let session = Arc::new(Self {
            shell: Arc::clone(&shell),
            closed: AtomicBool::new(false),
        });
        spawn_reader(shell, output, sink, lease);
        Ok(session)
    }

    fn close_shell(&self) -> Result<(), ExecutorRuntimeError> {
        if self.closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let mut shell = self
            .shell
            .lock()
            .map_err(|_| ExecutorRuntimeError::Unavailable)?;
        if let Some(shell) = shell.as_mut() {
            shell.kill().map_err(map_linux_error)?;
        }
        Ok(())
    }
}

#[uniffi::export]
impl LinuxShellSession {
    /// Writes raw terminal input to the Alpine shell.
    ///
    /// # Errors
    /// Returns an executor error when the shell is unavailable or the pty rejects the write.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "UniFFI transfers terminal input as an owned byte vector"
    )]
    pub fn write(&self, bytes: Vec<u8>) -> Result<(), ExecutorRuntimeError> {
        let mut shell = self
            .shell
            .lock()
            .map_err(|_| ExecutorRuntimeError::Unavailable)?;
        shell
            .as_mut()
            .ok_or(ExecutorRuntimeError::Unavailable)?
            .write(&bytes)
            .map_err(map_linux_error)
    }

    /// Resizes the Alpine shell pty.
    ///
    /// # Errors
    /// Returns an executor error when the shell is unavailable or the pty rejects the resize.
    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), ExecutorRuntimeError> {
        let shell = self
            .shell
            .lock()
            .map_err(|_| ExecutorRuntimeError::Unavailable)?;
        shell
            .as_ref()
            .ok_or(ExecutorRuntimeError::Unavailable)?
            .resize(rows, cols)
            .map_err(map_linux_error)
    }

    /// Kills the Alpine shell. Calling this more than once is safe.
    ///
    /// # Errors
    /// Returns an executor error when the first kill cannot be delivered.
    pub fn close(&self) -> Result<(), ExecutorRuntimeError> {
        self.close_shell()
    }
}

impl Drop for LinuxShellSession {
    fn drop(&mut self) {
        drop(self.close_shell());
    }
}

fn spawn_reader(
    shell: Arc<Mutex<Option<LinuxShell>>>,
    output: Receiver<Vec<u8>>,
    sink: Arc<dyn LinuxShellSink>,
    lease: Arc<ShellLease>,
) {
    thread::spawn(move || {
        forward_output(output, &sink);
        let code = wait_for_shell(&shell);
        sink.on_exit(code);
        drop(lease);
    });
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the reader thread owns the output receiver"
)]
fn forward_output(output: Receiver<Vec<u8>>, sink: &Arc<dyn LinuxShellSink>) {
    while let Ok(bytes) = output.recv() {
        for chunk in bytes.chunks(MAX_OUTPUT_CHUNK) {
            sink.on_output(chunk.to_vec());
        }
    }
}

fn wait_for_shell(shell: &Mutex<Option<LinuxShell>>) -> i32 {
    let Ok(mut shell_guard) = shell.lock() else {
        return 1;
    };
    let Some(mut shell) = shell_guard.take() else {
        return 1;
    };
    shell.wait().unwrap_or(1)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "matching the owned Linux error avoids exposing error details"
)]
fn map_linux_error(error: LinuxError) -> ExecutorRuntimeError {
    match error {
        LinuxError::NotReady(_) => ExecutorRuntimeError::LinuxNotReady,
        _ => ExecutorRuntimeError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::MAX_OUTPUT_CHUNK;

    #[test]
    fn output_cap_is_8_kib() {
        assert_eq!(MAX_OUTPUT_CHUNK, 8 * 1024);
    }
}
