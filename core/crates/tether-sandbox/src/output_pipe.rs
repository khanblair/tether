//! A guest output pipe that keeps at most a fixed number of bytes in memory.

use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};

use tokio::io::AsyncWrite;
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};

#[derive(Debug, Default)]
struct Retained {
    bytes: Vec<u8>,
    discarded: bool,
}

/// Collects what a guest writes to stdout or stderr.
///
/// The pipe keeps the first `limit` bytes and silently drops the rest, so a guest that prints
/// without end cannot grow host memory. A write always reports success: the guest keeps
/// running, and the run report says the output was cut.
#[derive(Debug, Clone)]
pub(crate) struct BoundedPipe {
    limit: usize,
    retained: Arc<Mutex<Retained>>,
}

impl BoundedPipe {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            limit,
            retained: Arc::new(Mutex::new(Retained::default())),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Retained> {
        self.retained
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Bytes kept since `offset`, or none when nothing new arrived.
    pub(crate) fn since(&self, offset: usize) -> Vec<u8> {
        let retained = self.lock();
        retained.bytes.get(offset..).unwrap_or_default().to_vec()
    }

    /// Whether the pipe dropped any bytes.
    pub(crate) fn discarded(&self) -> bool {
        self.lock().discarded
    }
}

impl IsTerminal for BoundedPipe {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl StdoutStream for BoundedPipe {
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }
}

impl AsyncWrite for BoundedPipe {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut retained = self.lock();
        let room = self.limit.saturating_sub(retained.bytes.len());
        let keep = buf.len().min(room);
        retained.bytes.extend_from_slice(&buf[..keep]);
        if keep < buf.len() {
            retained.discarded = true;
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use std::task::Waker;

    use super::*;

    fn write(pipe: &mut BoundedPipe, bytes: &[u8]) -> usize {
        let mut context = Context::from_waker(Waker::noop());
        match Pin::new(pipe).poll_write(&mut context, bytes) {
            Poll::Ready(Ok(count)) => count,
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn bytes_are_kept_up_to_the_limit_and_the_rest_is_dropped() {
        let mut pipe = BoundedPipe::new(8);
        assert_eq!(write(&mut pipe, b"abcde"), 5);
        assert!(!pipe.discarded());
        assert_eq!(
            write(&mut pipe, b"fghij"),
            5,
            "a write always reports success"
        );
        assert!(pipe.discarded());
        assert_eq!(pipe.since(0), b"abcdefgh");
        assert_eq!(write(&mut pipe, b"zzz"), 3);
        assert_eq!(pipe.since(0), b"abcdefgh");
    }

    #[test]
    fn since_returns_only_new_bytes() {
        let mut pipe = BoundedPipe::new(100);
        assert_eq!(pipe.since(0), Vec::<u8>::new());
        write(&mut pipe, b"one");
        assert_eq!(pipe.since(0), b"one");
        write(&mut pipe, b"two");
        assert_eq!(pipe.since(3), b"two");
        assert_eq!(pipe.since(6), Vec::<u8>::new());
        assert_eq!(pipe.since(99), Vec::<u8>::new());
    }

    #[test]
    fn memory_stays_bounded_for_a_huge_stream() {
        let mut pipe = BoundedPipe::new(1024);
        let chunk = vec![b'x'; 64 * 1024];
        for _ in 0..256 {
            write(&mut pipe, &chunk);
        }
        assert_eq!(pipe.since(0).len(), 1024);
        assert!(pipe.discarded());
    }
}
