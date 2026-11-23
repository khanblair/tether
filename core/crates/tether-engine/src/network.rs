//! Network pause state shared by session actors and the platform callback.

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

/// Shared network state for all sessions in one process.
#[derive(Clone)]
pub struct NetworkControl {
    enabled: Arc<AtomicBool>,
    online: Arc<AtomicBool>,
    wake: Arc<Notify>,
    clock: Arc<dyn NetworkClock>,
}

/// Supplies time and sleeping to the network pause state machine.
pub trait NetworkClock: Send + Sync {
    /// Returns the current monotonic time.
    fn now(&self) -> Instant;
    /// Sleeps for a duration without blocking the async runtime.
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

struct TokioNetworkClock;

impl NetworkClock for TokioNetworkClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(tokio::time::sleep(duration))
    }
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
            clock: Arc::new(TokioNetworkClock),
        }
    }

    /// Creates network control using an injected clock and sleeper.
    #[must_use]
    pub fn with_clock(clock: Arc<dyn NetworkClock>) -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            online: Arc::new(AtomicBool::new(true)),
            wake: Arc::new(Notify::new()),
            clock,
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

    /// Marks the connection unavailable after a provider network failure.
    pub fn mark_offline(&self) {
        self.online.store(false, Ordering::Release);
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
        // The backoff loop never finishes normally, so its only outcome is the error.
        match self.wait_backoff(cancel).await {
            Ok(never) => match never {},
            Err(result) => result,
        }
    }

    /// Backs off until the network is back. The `Err` carries the final result.
    async fn wait_backoff(&self, cancel: &CancellationToken) -> Result<Infallible, WaitResult> {
        let started = self.clock.now();
        let mut delay = Duration::from_secs(2);
        loop {
            let elapsed = self.clock.now().saturating_duration_since(started);
            let remaining = remaining_wait(elapsed).ok_or(WaitResult::TimedOut)?;
            self.wait_once(cancel, delay.min(remaining)).await?;
            delay = next_backoff(delay);
        }
    }

    /// Waits for a wake-up or the sleep to end. The `Err` means the wait is over.
    async fn wait_once(
        &self,
        cancel: &CancellationToken,
        sleep: Duration,
    ) -> Result<(), WaitResult> {
        tokio::select! {
            () = cancel.cancelled() => return Err(WaitResult::Cancelled),
            () = self.wake.notified() => {}
            () = self.clock.sleep(sleep) => {}
        }
        if self.is_online() {
            Err(WaitResult::Online)
        } else {
            Ok(())
        }
    }
}

/// Time left in the 30-minute wait, or `None` once the limit is reached.
fn remaining_wait(elapsed: Duration) -> Option<Duration> {
    let limit = Duration::from_secs(30 * 60);
    if elapsed >= limit {
        None
    } else {
        Some(limit.saturating_sub(elapsed))
    }
}

fn next_backoff(delay: Duration) -> Duration {
    (delay * 2).min(Duration::from_secs(60))
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

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct FakeClock {
        now: Mutex<Instant>,
        sleeps: Mutex<Vec<Duration>>,
    }

    impl FakeClock {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                now: Mutex::new(Instant::now()),
                sleeps: Mutex::new(Vec::new()),
            })
        }
    }

    impl NetworkClock for FakeClock {
        fn now(&self) -> Instant {
            *self.now.lock().unwrap()
        }

        fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
            self.sleeps.lock().unwrap().push(duration);
            *self.now.lock().unwrap() += duration;
            Box::pin(std::future::ready(()))
        }
    }

    async fn wait_for_online(control: NetworkControl) -> WaitResult {
        control.wait_until_online(&CancellationToken::new()).await
    }

    #[tokio::test]
    async fn network_changed_wakes_a_paused_wait() {
        let control = NetworkControl::new();
        control.enable(false);
        let wake = control.clone();
        let task = tokio::spawn(wait_for_online(wake));
        tokio::task::yield_now().await;
        control.set_online(true);
        assert_eq!(task.await.unwrap(), WaitResult::Online);
    }

    #[tokio::test]
    async fn cancellation_ends_a_paused_wait_without_sleeping() {
        let clock = FakeClock::new();
        let control = NetworkControl::with_clock(Arc::clone(&clock) as Arc<dyn NetworkClock>);
        control.enable(false);
        let cancel = CancellationToken::new();
        cancel.cancel();

        assert_eq!(
            control.wait_until_online(&cancel).await,
            WaitResult::Cancelled
        );
    }

    #[tokio::test]
    async fn an_offline_wait_gives_up_after_thirty_minutes() {
        let clock = FakeClock::new();
        let control = NetworkControl::with_clock(Arc::clone(&clock) as Arc<dyn NetworkClock>);
        control.enable(false);

        assert_eq!(
            control.wait_until_online(&CancellationToken::new()).await,
            WaitResult::TimedOut
        );
        assert_eq!(
            clock.sleeps.lock().unwrap().last(),
            Some(&Duration::from_secs(58))
        );
    }

    #[test]
    fn remaining_wait_has_a_thirty_minute_limit() {
        assert_eq!(
            remaining_wait(Duration::from_secs(1_799)),
            Some(Duration::from_secs(1))
        );
        assert_eq!(remaining_wait(Duration::from_secs(1_800)), None);
    }

    #[test]
    fn backoff_sequence_is_two_four_eight_sixteen_thirty_two_then_sixty() {
        let mut delay = Duration::from_secs(2);
        let mut values = Vec::new();
        for _ in 0..7 {
            values.push(delay);
            delay = next_backoff(delay);
        }

        assert_eq!(
            values,
            vec![
                Duration::from_secs(2),
                Duration::from_secs(4),
                Duration::from_secs(8),
                Duration::from_secs(16),
                Duration::from_secs(32),
                Duration::from_secs(60),
                Duration::from_secs(60),
            ]
        );
    }
}
