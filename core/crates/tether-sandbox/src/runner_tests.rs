//! Runner behavior tests.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

use crate::test_support::{ENDLESS, EXIT_THREE, HELLO, fixture};
use crate::{
    ModuleCache, OutputSink, RunLimits, RunOutcome, RunSpec, SandboxRunner, WasiContextSpec,
};

const MANY_OUTPUTS: &str = r#"
    (data (i32.const 8) "abcdefghij")
    (func (export "_start")
        (local $count i32)
        (local.set $count (i32.const 20))
        (loop
            (i32.store (i32.const 0) (i32.const 8))
            (i32.store (i32.const 4) (i32.const 10))
            (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 20))
            drop
            (local.set $count (i32.sub (local.get $count) (i32.const 1)))
            (br_if 0 (local.get $count))
        )
    )
"#;

#[derive(Default)]
struct Sink {
    stdout: Vec<u8>,
    calls: usize,
}

impl OutputSink for Sink {
    fn stdout(&mut self, bytes: &[u8]) {
        self.stdout.extend_from_slice(bytes);
        self.calls += 1;
    }

    fn stderr(&mut self, _bytes: &[u8]) {}
}

fn run_fixture(body: &str, limits: RunLimits) -> (crate::RunReport, Sink) {
    let cache = Arc::new(ModuleCache::new().unwrap());
    let module = fixture(&cache, body).unwrap();
    let workspace = tempdir().unwrap();
    let context = WasiContextSpec::new(workspace.path()).unwrap();
    let spec = RunSpec {
        module,
        argv: vec!["fixture".into()],
        workspace: context,
        limits,
    };
    let mut sink = Sink::default();
    let report = SandboxRunner::new(cache)
        .run(&spec, &mut sink, &CancellationToken::new())
        .unwrap();
    (report, sink)
}

#[test]
fn hello_exits_and_streams_exact_output() {
    let (report, sink) = run_fixture(HELLO, RunLimits::default());
    assert_eq!(report.outcome, RunOutcome::Exited(0));
    assert_eq!(sink.stdout, b"hello\n");
    assert_eq!(report.output_bytes, 6);
}

#[test]
fn proc_exit_preserves_exit_code() {
    let (report, _) = run_fixture(EXIT_THREE, RunLimits::default());
    assert_eq!(report.outcome, RunOutcome::Exited(3));
}

#[test]
fn endless_fixture_times_out_without_blocking() {
    let started = Instant::now();
    let (report, _) = run_fixture(
        ENDLESS,
        RunLimits {
            timeout: Duration::from_millis(200),
            ..RunLimits::default()
        },
    );
    assert_eq!(report.outcome, RunOutcome::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn cancellation_interrupts_an_endless_fixture() {
    let cache = Arc::new(ModuleCache::new().unwrap());
    let module = fixture(&cache, ENDLESS).unwrap();
    let workspace = tempdir().unwrap();
    let context = WasiContextSpec::new(workspace.path()).unwrap();
    let spec = RunSpec {
        module,
        argv: vec!["fixture".into()],
        workspace: context,
        limits: RunLimits::default(),
    };
    let cancel = CancellationToken::new();
    let later = cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        later.cancel();
    });
    let mut sink = Sink::default();
    let report = SandboxRunner::new(cache)
        .run(&spec, &mut sink, &cancel)
        .unwrap();
    assert_eq!(report.outcome, RunOutcome::Cancelled);
}

#[test]
fn output_cap_truncates_without_trapping_the_guest() {
    let (report, sink) = run_fixture(
        MANY_OUTPUTS,
        RunLimits {
            output_bytes: 5,
            ..RunLimits::default()
        },
    );
    assert_eq!(report.outcome, RunOutcome::Exited(0));
    assert_eq!(sink.stdout, b"abcde");
    assert!(report.output_truncated);
}
