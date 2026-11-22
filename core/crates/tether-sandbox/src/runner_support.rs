//! Runner timing and host-path helpers.

use std::fs;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio_util::sync::CancellationToken;

use crate::RunLimits;

pub(crate) const STOP_NONE: u8 = 0;
pub(crate) const STOP_TIMEOUT: u8 = 1;
pub(crate) const STOP_CANCELLED: u8 = 2;
const EPOCH_TICK: Duration = Duration::from_millis(10);

#[expect(
    clippy::excessive_nesting,
    reason = "the ticker checks cancellation at each short interval"
)]
pub(crate) fn spawn_epoch_thread(
    engine: &wasmtime::Engine,
    limits: RunLimits,
    cancel: CancellationToken,
    stop: Arc<AtomicBool>,
    reason: Arc<AtomicU8>,
) -> thread::JoinHandle<()> {
    let engine = engine.clone();
    thread::spawn(move || {
        let ticks = limits.epoch_ticks(EPOCH_TICK);
        if ticks == 0 {
            reason.store(STOP_TIMEOUT, Ordering::SeqCst);
            engine.increment_epoch();
            return;
        }
        for tick in 0..ticks {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            if cancel.is_cancelled() {
                reason.store(STOP_CANCELLED, Ordering::SeqCst);
                engine.increment_epoch();
                return;
            }
            thread::sleep(EPOCH_TICK);
            if stop.load(Ordering::SeqCst) {
                return;
            }
            if cancel.is_cancelled() {
                reason.store(STOP_CANCELLED, Ordering::SeqCst);
            } else if tick + 1 == ticks {
                reason.store(STOP_TIMEOUT, Ordering::SeqCst);
            }
            engine.increment_epoch();
            if reason.load(Ordering::SeqCst) != STOP_NONE {
                return;
            }
        }
    })
}

pub(crate) fn create_run_tmp() -> io::Result<std::path::PathBuf> {
    static NEXT: AtomicU8 = AtomicU8::new(0);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for attempt in 0..32u8 {
        let id = NEXT.fetch_add(1, Ordering::Relaxed).wrapping_add(attempt);
        let path = std::env::temp_dir().join(format!("tether-sandbox-{stamp:x}-{id:x}"));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "temporary directory collision",
    ))
}

pub(crate) fn sanitize_message(message: &str) -> String {
    let mut safe = String::with_capacity(message.len());
    for part in message.split_whitespace() {
        if part.starts_with('/') {
            safe.push_str("<host-path>");
        } else {
            safe.push_str(part);
        }
        safe.push(' ');
    }
    safe.trim_end().to_owned()
}
