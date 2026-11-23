//! Runner behavior tests.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

use crate::test_support::{
    ENDLESS, EXIT_THREE, HELLO, STREAMING, containment_module, environment_module, fixture,
    memory_module, run_lock, tmp_module, workspace_io_module,
};
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
    chunks: Vec<Vec<u8>>,
    calls: usize,
}

impl OutputSink for Sink {
    fn stdout(&mut self, bytes: &[u8]) {
        self.stdout.extend_from_slice(bytes);
        self.chunks.push(bytes.to_vec());
        self.calls += 1;
    }

    fn stderr(&mut self, _bytes: &[u8]) {}
}

fn run_fixture(body: &str, limits: RunLimits) -> (crate::RunReport, Sink) {
    let _guard = run_lock();
    let cache = Arc::new(ModuleCache::new().unwrap());
    let module = fixture(&cache, body).unwrap();
    let workspace = tempdir().unwrap();
    let context = WasiContextSpec::new(workspace.path()).unwrap();
    let spec = RunSpec {
        module,
        argv: vec!["fixture".into()],
        workspace: context,
        cwd: "/workspace".into(),
        pack_root: workspace.path().to_path_buf(),
        extra_mounts: Vec::new(),
        extra_env: BTreeMap::new(),
        limits,
    };
    let mut sink = Sink::default();
    let report = SandboxRunner::new(cache)
        .run(&spec, &mut sink, &CancellationToken::new())
        .unwrap();
    (report, sink)
}

fn run_module(
    module: Arc<wasmtime::Module>,
    workspace: &Path,
    limits: RunLimits,
    cancel: &CancellationToken,
) -> (crate::RunReport, Sink) {
    let _guard = run_lock();
    run_module_inner(module, workspace, limits, cancel)
}

fn run_module_inner(
    module: Arc<wasmtime::Module>,
    workspace: &Path,
    limits: RunLimits,
    cancel: &CancellationToken,
) -> (crate::RunReport, Sink) {
    let cache = Arc::new(ModuleCache::new().unwrap());
    let context = WasiContextSpec::new(workspace).unwrap();
    let spec = RunSpec {
        module,
        argv: vec!["fixture".into()],
        workspace: context,
        cwd: "/workspace".into(),
        pack_root: workspace.to_path_buf(),
        extra_mounts: Vec::new(),
        extra_env: BTreeMap::new(),
        limits,
    };
    let mut sink = Sink::default();
    let report = SandboxRunner::new(cache)
        .run(&spec, &mut sink, cancel)
        .unwrap();
    (report, sink)
}

fn sandbox_tmp_names() -> std::collections::BTreeSet<String> {
    fs::read_dir(std::env::temp_dir())
        .unwrap()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with("tether-sandbox-").then_some(name)
        })
        .collect()
}

fn clear_sandbox_tmp() {
    for entry in fs::read_dir(std::env::temp_dir()).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("tether-sandbox-") {
            drop(fs::remove_dir_all(entry.path()));
        }
    }
}

#[test]
fn hello_writes_exact_output() {
    let (report, sink) = run_fixture(HELLO, RunLimits::default());
    assert_eq!(report.outcome, RunOutcome::Exited(0));
    assert_eq!(sink.stdout, b"hello\n");
    assert_eq!(report.output_bytes, 6);
}

#[test]
fn exit_code_three_is_preserved() {
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
fn streaming_forwards_chunks_before_the_run_ends() {
    let cache = Arc::new(ModuleCache::new().unwrap());
    let module = fixture(&cache, STREAMING).unwrap();
    let workspace = tempdir().unwrap();
    let (report, sink) = run_module(
        module,
        workspace.path(),
        RunLimits::default(),
        &CancellationToken::new(),
    );

    assert_eq!(report.outcome, RunOutcome::Exited(0));
    assert!(sink.calls >= 3);
    assert_eq!(sink.stdout, b"one\ntwo\nthree\n");
    assert_eq!(
        sink.chunks,
        [b"one\n".to_vec(), b"two\n".to_vec(), b"three\n".to_vec()]
    );
}

#[test]
fn workspace_can_be_read_and_written() {
    let cache = Arc::new(ModuleCache::new().unwrap());
    let module = workspace_io_module(&cache).unwrap();
    let workspace = tempdir().unwrap();
    fs::write(workspace.path().join("in.txt"), b"host input").unwrap();

    let (report, sink) = run_module(
        module,
        workspace.path(),
        RunLimits::default(),
        &CancellationToken::new(),
    );

    assert_eq!(report.outcome, RunOutcome::Exited(0));
    assert_eq!(fs::read(workspace.path().join("out.txt")).unwrap(), b"x");
    assert_eq!(sink.stdout, b"host input");
}

#[test]
fn environment_is_exactly_the_fixed_guest_environment() {
    let _host_secret = std::env::vars().find(|(name, _)| {
        name.contains("KEY") || name.contains("TOKEN") || name.contains("SECRET")
    });
    let cache = Arc::new(ModuleCache::new().unwrap());
    let module = environment_module(&cache).unwrap();
    let workspace = tempdir().unwrap();
    let (report, sink) = run_module(
        module,
        workspace.path(),
        RunLimits::default(),
        &CancellationToken::new(),
    );
    let mut lines = String::from_utf8(sink.stdout)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    lines.sort();

    assert_eq!(report.outcome, RunOutcome::Exited(0));
    assert_eq!(
        lines,
        [
            "HOME=/workspace".to_owned(),
            "LANG=C.UTF-8".to_owned(),
            "PATH=/usr/bin:/bin".to_owned(),
            "TMPDIR=/tmp".to_owned(),
        ]
    );
}

#[cfg(unix)]
#[test]
fn guest_filesystem_is_contained_to_two_preopens() {
    use std::os::unix::fs::symlink;

    let outer = tempdir().unwrap();
    let workspace = outer.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let secret = outer.path().join("secret.txt");
    fs::write(&secret, b"outside secret").unwrap();
    symlink(&secret, workspace.join("link")).unwrap();
    let cache = Arc::new(ModuleCache::new().unwrap());
    let module = containment_module(&cache, secret.to_str().unwrap()).unwrap();

    let (report, sink) = run_module(
        module,
        &workspace,
        RunLimits::default(),
        &CancellationToken::new(),
    );
    let output = String::from_utf8(sink.stdout).unwrap();

    assert_eq!(report.outcome, RunOutcome::Exited(0));
    assert!(output.contains("P3=/workspace\n"));
    assert!(output.contains("P4=/tmp\n"));
    assert!(output.contains("P5=none\n"));
    assert!(output.contains("etc=fail\n"));
    assert!(output.contains("root=fail\n"));
    assert!(output.contains("parent=fail\n"));
    assert!(output.contains("absolute=fail\n"));
    assert!(output.contains("link=fail\n"));
    assert!(!output.contains("outside secret"));
}

#[test]
fn temporary_preopen_starts_empty_and_is_removed() {
    let _guard = run_lock();
    clear_sandbox_tmp();
    let before = sandbox_tmp_names();
    let cache = Arc::new(ModuleCache::new().unwrap());
    let module = tmp_module(&cache).unwrap();
    let workspace = tempdir().unwrap();
    let (report, _) = run_module_inner(
        module,
        workspace.path(),
        RunLimits::default(),
        &CancellationToken::new(),
    );

    assert_eq!(report.outcome, RunOutcome::Exited(0));
    assert_eq!(sandbox_tmp_names(), before);
}

#[test]
fn temporary_preopen_is_removed_after_timeout() {
    let _guard = run_lock();
    clear_sandbox_tmp();
    let before = sandbox_tmp_names();
    let started = Instant::now();
    let cache = Arc::new(ModuleCache::new().unwrap());
    let module = fixture(&cache, ENDLESS).unwrap();
    let workspace = tempdir().unwrap();
    let (report, _) = run_module_inner(
        module,
        workspace.path(),
        RunLimits {
            timeout: Duration::from_millis(200),
            ..RunLimits::default()
        },
        &CancellationToken::new(),
    );

    assert_eq!(report.outcome, RunOutcome::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(sandbox_tmp_names(), before);
}

#[test]
fn cancellation_interrupts_an_endless_fixture() {
    let _guard = run_lock();
    let started = Instant::now();
    let cache = Arc::new(ModuleCache::new().unwrap());
    let module = fixture(&cache, ENDLESS).unwrap();
    let workspace = tempdir().unwrap();
    let context = WasiContextSpec::new(workspace.path()).unwrap();
    let spec = RunSpec {
        module,
        argv: vec!["fixture".into()],
        workspace: context,
        cwd: "/workspace".into(),
        pack_root: workspace.path().to_path_buf(),
        extra_mounts: Vec::new(),
        extra_env: BTreeMap::new(),
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
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn memory_limit_reports_out_of_memory() {
    let cache = Arc::new(ModuleCache::new().unwrap());
    let module = memory_module(&cache).unwrap();
    let workspace = tempdir().unwrap();
    let (report, _) = run_module(
        module,
        workspace.path(),
        RunLimits {
            memory_bytes: 16 * 1024 * 1024,
            ..RunLimits::default()
        },
        &CancellationToken::new(),
    );

    assert_eq!(report.outcome, RunOutcome::OutOfMemory);
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
    assert_eq!(report.output_bytes, 5);
    assert!(report.output_truncated);
}

#[test]
fn cached_module_compiles_once_for_repeated_bytes() {
    let cache = ModuleCache::new().unwrap();
    let bytes = wat::parse_str(r#"(module (memory 1) (func (export "_start")))"#).unwrap();

    cache.load(&bytes).unwrap();
    cache.load(&bytes).unwrap();

    assert_eq!(cache.len().unwrap(), 1);
}
