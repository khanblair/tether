//! Network pause state shared by session actors and the platform callback.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

/// Shared network state for all sessions in one process.
#[derive(Debug, Clone)]
pub struct NetworkControl {
    enabled: Arc<AtomicBool>,
    online: Arc<AtomicBool>,
    wake: Arc<Notify>,
}

impl Default for NetworkControl {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkControl {
    /// Creates a disabled control. Disabled controls preserve existing behavior.
    #[must_use]
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            online: Arc::new(AtomicBool::new(true)),
            wake: Arc::new(Notify::new()),
        }
    }

    /// Enables platform-aware network pausing with its initial state.
    pub fn enable(&self, online: bool) {
        self.online.store(online, Ordering::Release);
        self.enabled.store(true, Ordering::Release);
        self.wake.notify_waiters();
    }

    /// Records a platform connectivity change and wakes paused turns.
    pub fn set_online(&self, online: bool) {
        self.online.store(online, Ordering::Release);
        self.wake.notify_waiters();
    }

    /// Returns whether the platform currently reports an online connection.
    #[must_use]
    pub fn is_online(&self) -> bool {
        self.online.load(Ordering::Acquire)
    }

    /// Returns whether this control should affect turns.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Waits with backoff until online, cancellation, or the 30-minute limit.
    pub async fn wait_until_online(&self, cancel: &CancellationToken) -> WaitResult {
        if !self.enabled() || self.is_online() {
            return WaitResult::Online;
        }
        let started = Instant::now();
        let mut delay = Duration::from_secs(2);
        loop {
            let elapsed = started.elapsed();
            if elapsed >= Duration::from_secs(30 * 60) {
                return WaitResult::TimedOut;
            }
            let remaining = Duration::from_secs(30 * 60).saturating_sub(elapsed);
            let sleep = delay.min(remaining);
            tokio::select! {
                () = cancel.cancelled() => return WaitResult::Cancelled,
                () = self.wake.notified() => if self.is_online() { return WaitResult::Online; },
                () = tokio::time::sleep(sleep) => if self.is_online() { return WaitResult::Online; },
            }
            delay = (delay * 2).min(Duration::from_secs(60));
        }
    }
}

/// Result of waiting for the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    /// Connectivity returned.
    Online,
    /// The active turn was cancelled.
    Cancelled,
    /// Connectivity did not return within 30 minutes.
    TimedOut,
}
