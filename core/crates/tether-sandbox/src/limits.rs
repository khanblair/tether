//! Per-run limits and bounded streaming output.

use std::time::Duration;

use wasmtime::{StoreLimits, StoreLimitsBuilder};

use crate::OutputSink;

/// Defaults from product-scope F10, with a bounded output stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunLimits {
    /// Maximum guest linear memory, in bytes.
    pub memory_bytes: usize,
    /// Maximum wall time before an epoch interrupt.
    pub timeout: Duration,
    /// Maximum combined stdout and stderr bytes forwarded to the UI.
    pub output_bytes: usize,
}

impl Default for RunLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 512 * 1024 * 1024,
            timeout: Duration::from_secs(60),
            output_bytes: 1024 * 1024,
        }
    }
}

impl RunLimits {
    /// Builds Wasmtime's memory limiter for attachment to a per-run store.
    #[must_use]
    pub fn store_limits(self) -> StoreLimits {
        StoreLimitsBuilder::new()
            .memory_size(self.memory_bytes)
            .trap_on_grow_failure(true)
            .build()
    }

    /// Number of epoch ticks needed to cover the timeout, rounded up.
    #[must_use]
    pub fn epoch_ticks(self, tick: Duration) -> u64 {
        if tick.is_zero() {
            return 0;
        }
        let total = self.timeout.as_nanos();
        let step = tick.as_nanos();
        u64::try_from(total.div_ceil(step)).unwrap_or(u64::MAX)
    }
}

/// Forwards at most the combined output limit, preserving chunk order.
pub struct CappedOutput<'a> {
    inner: &'a mut dyn OutputSink,
    limit: usize,
    sent: usize,
    truncated: bool,
}

impl<'a> CappedOutput<'a> {
    /// Wraps the output sink with one shared cap for both channels.
    pub fn new(inner: &'a mut dyn OutputSink, limit: usize) -> Self {
        Self {
            inner,
            limit,
            sent: 0,
            truncated: false,
        }
    }

    /// Whether any bytes were discarded.
    #[must_use]
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// Number of bytes forwarded.
    #[must_use]
    pub fn sent(&self) -> usize {
        self.sent
    }

    fn forward(&mut self, bytes: &[u8], stderr: bool) {
        let count = bytes.len().min(self.limit.saturating_sub(self.sent));
        if count > 0 {
            self.send(&bytes[..count], stderr);
            self.sent += count;
        }
        if count < bytes.len() {
            self.truncated = true;
        }
    }

    fn send(&mut self, bytes: &[u8], stderr: bool) {
        if stderr {
            self.inner.stderr(bytes);
        } else {
            self.inner.stdout(bytes);
        }
    }
}

impl OutputSink for CappedOutput<'_> {
    fn stdout(&mut self, bytes: &[u8]) {
        self.forward(bytes, false);
    }
    fn stderr(&mut self, bytes: &[u8]) {
        self.forward(bytes, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Collected(Vec<u8>);
    impl OutputSink for Collected {
        fn stdout(&mut self, bytes: &[u8]) {
            self.0.extend_from_slice(bytes);
        }
        fn stderr(&mut self, bytes: &[u8]) {
            self.0.extend_from_slice(bytes);
        }
    }

    #[test]
    fn stdout_and_stderr_share_the_cap() {
        let mut collected = Collected(Vec::new());
        let mut capped = CappedOutput::new(&mut collected, 5);
        capped.stdout(b"one");
        capped.stderr(b"two");
        assert_eq!(capped.sent(), 5);
        assert!(capped.truncated());
        assert_eq!(collected.0, b"onetw");
    }

    #[test]
    fn epoch_ticks_round_up() {
        let limits = RunLimits {
            timeout: Duration::from_millis(250),
            ..RunLimits::default()
        };
        assert_eq!(limits.epoch_ticks(Duration::from_millis(100)), 3);
    }
}
