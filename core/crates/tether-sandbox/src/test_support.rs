//! Shared WASI test fixtures.

use std::fmt::Write;
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard, OnceLock};

use wasmtime::Module;

use crate::{ModuleCache, SandboxError};

static RUN_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn run_lock() -> MutexGuard<'static, ()> {
    RUN_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Compiles a small WASI command fixture from WAT.
pub(crate) fn fixture(cache: &ModuleCache, body: &str) -> Result<Arc<Module>, SandboxError> {
    compile(cache, &fixture_wat(body))
}

/// Builds raw WASM bytes for a basic WASI command fixture.
pub(crate) fn fixture_bytes(body: &str) -> Result<Vec<u8>, SandboxError> {
    wat::parse_str(fixture_wat(body)).map_err(|error| SandboxError::Module(error.to_string()))
}

fn fixture_wat(body: &str) -> String {
    format!(
        r#"(module
            (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))
            (memory (export "memory") 1)
            {body}
        )"#
    )
}

/// Compiles a complete WAT module for a WASI behavior test.
pub(crate) fn compile(cache: &ModuleCache, wat: &str) -> Result<Arc<Module>, SandboxError> {
    let bytes = wat::parse_str(wat).map_err(|error| SandboxError::Module(error.to_string()))?;
    cache.load(&bytes)
}

/// Escapes a host path for insertion into a WAT string literal.
pub(crate) fn wat_string(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .fold(String::new(), |mut escaped, byte| {
            write!(escaped, r"\{byte:02x}").expect("writing to a String cannot fail");
            escaped
        })
}

/// A fixture that writes one fixed stdout message.
pub(crate) const HELLO: &str = r#"
    (data (i32.const 8) "hello\n")
    (func (export "_start")
        (i32.store (i32.const 0) (i32.const 8))
        (i32.store (i32.const 4) (i32.const 6))
        (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 20))
        drop
    )
"#;

/// A fixture that exits through WASI with code three.
pub(crate) const EXIT_THREE: &str = r#"
    (func (export "_start") (call $proc_exit (i32.const 3)))
"#;

/// A fixture that never returns.
pub(crate) const ENDLESS: &str = r#"
    (func (export "_start") (loop br 0))
"#;

/// A fixture that emits three separated chunks around a long computation.
pub(crate) const STREAMING: &str = r#"
    (data (i32.const 100) "one\n")
    (data (i32.const 110) "two\n")
    (data (i32.const 120) "three\n")
    (func $emit (param $ptr i32) (param $len i32)
        (i32.store (i32.const 0) (local.get $ptr))
        (i32.store (i32.const 4) (local.get $len))
        (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 20)))
    )
    (func (export "_start")
        (local $count i32)
        (call $emit (i32.const 100) (i32.const 4))
        (local.set $count (i32.const 50000000))
        (loop
            (local.set $count (i32.sub (local.get $count) (i32.const 1)))
            (br_if 0 (local.get $count))
        )
        (call $emit (i32.const 110) (i32.const 4))
        (local.set $count (i32.const 50000000))
        (loop
            (local.set $count (i32.sub (local.get $count) (i32.const 1)))
            (br_if 0 (local.get $count))
        )
        (call $emit (i32.const 120) (i32.const 6))
    )
"#;

/// Builds a fixture that writes and reads files through `/workspace`.
pub(crate) fn workspace_io_module(cache: &ModuleCache) -> Result<Arc<Module>, SandboxError> {
    compile(
        cache,
        r#"(module
            (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "fd_read" (func $fd_read (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "path_open" (func $path_open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
            (memory (export "memory") 2)
            (data (i32.const 100) "out.txt")
            (data (i32.const 120) "in.txt")
            (data (i32.const 140) "x")
            (func (export "_start")
                (drop (call $path_open (i32.const 3) (i32.const 0) (i32.const 100) (i32.const 7) (i32.const 1)
                    (i64.const 9280) (i64.const 64) (i32.const 0) (i32.const 200)))
                (i32.store (i32.const 300) (i32.const 140))
                (i32.store (i32.const 304) (i32.const 1))
                (drop (call $fd_write (i32.load (i32.const 200)) (i32.const 300) (i32.const 1) (i32.const 320)))
                (drop (call $path_open (i32.const 3) (i32.const 0) (i32.const 120) (i32.const 6) (i32.const 0)
                    (i64.const 2) (i64.const 0) (i32.const 0) (i32.const 220)))
                (i32.store (i32.const 340) (i32.const 500))
                (i32.store (i32.const 344) (i32.const 32))
                (drop (call $fd_read (i32.load (i32.const 220)) (i32.const 340) (i32.const 1) (i32.const 360)))
                (i32.store (i32.const 344) (i32.load (i32.const 360)))
                (drop (call $fd_write (i32.const 1) (i32.const 340) (i32.const 1) (i32.const 380)))
            )
        )"#,
    )
}

/// Builds a fixture that prints every environment entry.
pub(crate) fn environment_module(cache: &ModuleCache) -> Result<Arc<Module>, SandboxError> {
    compile(
        cache,
        r#"(module
            (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "environ_sizes_get" (func $environ_sizes_get (param i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "environ_get" (func $environ_get (param i32 i32) (result i32)))
            (memory (export "memory") 4)
            (data (i32.const 3000) "\0a")
            (func $write_line (param $ptr i32)
                (local $len i32)
                (local.set $len (i32.const 0))
                (block $done
                    (loop $scan
                        (br_if $done (i32.eqz (i32.load8_u (i32.add (local.get $ptr) (local.get $len)))))
                        (local.set $len (i32.add (local.get $len) (i32.const 1)))
                        (br $scan)
                    )
                )
                (i32.store (i32.const 0) (local.get $ptr))
                (i32.store (i32.const 4) (local.get $len))
                (i32.store (i32.const 8) (i32.const 3000))
                (i32.store (i32.const 12) (i32.const 1))
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 20)))
                (drop (call $fd_write (i32.const 1) (i32.const 8) (i32.const 1) (i32.const 20)))
            )
            (func (export "_start")
                (local $index i32)
                (drop (call $environ_sizes_get (i32.const 1000) (i32.const 1004)))
                (drop (call $environ_get (i32.const 1100) (i32.const 2000)))
                (local.set $index (i32.const 0))
                (block $done
                    (loop $entries
                        (br_if $done (i32.ge_u (local.get $index) (i32.load (i32.const 1000))))
                        (call $write_line (i32.load (i32.add (i32.const 1100)
                            (i32.mul (local.get $index) (i32.const 4)))))
                        (local.set $index (i32.add (local.get $index) (i32.const 1)))
                        (br $entries)
                    )
                )
            )
        )"#,
    )
}

/// Builds a fixture that creates a marker in the temporary preopen.
pub(crate) fn tmp_module(cache: &ModuleCache) -> Result<Arc<Module>, SandboxError> {
    compile(
        cache,
        r#"(module
            (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "path_open" (func $path_open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))
            (memory (export "memory") 1)
            (data (i32.const 100) "marker")
            (data (i32.const 120) "tmp")
            (func (export "_start")
                (i32.store (i32.const 200)
                    (call $path_open (i32.const 4) (i32.const 0) (i32.const 100) (i32.const 6) (i32.const 5)
                        (i64.const 9280) (i64.const 64) (i32.const 0) (i32.const 220)))
                (if (i32.ne (i32.load (i32.const 200)) (i32.const 0))
                    (then (call $proc_exit (i32.const 7))))
                (i32.store (i32.const 300) (i32.const 120))
                (i32.store (i32.const 304) (i32.const 3))
                (drop (call $fd_write (i32.load (i32.const 220)) (i32.const 300) (i32.const 1) (i32.const 320)))
            )
        )"#,
    )
}

/// Builds a fixture that grows linear memory until the store limiter traps it.
pub(crate) fn memory_module(cache: &ModuleCache) -> Result<Arc<Module>, SandboxError> {
    compile(
        cache,
        r#"(module
            (memory (export "memory") 1)
            (func (export "_start")
                (loop
                    (drop (memory.grow (i32.const 16)))
                    (br 0)
                )
            )
        )"#,
    )
}

const CONTAINMENT_IMPORTS: &str = r#"
            (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "fd_read" (func $fd_read (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "fd_close" (func $fd_close (param i32) (result i32)))
            (import "wasi_snapshot_preview1" "fd_prestat_get" (func $fd_prestat_get (param i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "fd_prestat_dir_name" (func $fd_prestat_dir_name (param i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "path_open" (func $path_open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
            (memory (export "memory") 3)
"#;

const CONTAINMENT_DATA: &str = r#"
            (data (i32.const 1000) "P3=")
            (data (i32.const 1004) "P4=")
            (data (i32.const 1008) "P5=")
            (data (i32.const 1020) "none")
            (data (i32.const 1024) "\0a")
            (data (i32.const 1100) "etc=")
            (data (i32.const 1105) "root=")
            (data (i32.const 1110) "parent=")
            (data (i32.const 1117) "absolute=")
            (data (i32.const 1126) "link=")
            (data (i32.const 1200) "/etc/passwd")
            (data (i32.const 1212) "/")
            (data (i32.const 1214) "../x")
            (data (i32.const 1400) "link")
            (data (i32.const 1500) "ok\0a")
            (data (i32.const 1504) "fail\0a")
"#;

const CONTAINMENT_HELPERS: &str = r"
            (func $report (param $label i32) (param $label_len i32) (param $path i32) (param $path_len i32)
                (local $error i32)
                (local.set $error (call $path_open (i32.const 3) (i32.const 0) (local.get $path)
                    (local.get $path_len) (i32.const 0) (i64.const 2) (i64.const 0) (i32.const 0) (i32.const 500)))
                (i32.store (i32.const 0) (local.get $label))
                (i32.store (i32.const 4) (local.get $label_len))
                (if (i32.eqz (local.get $error))
                    (then
                        (i32.store (i32.const 8) (i32.const 1500))
                        (i32.store (i32.const 12) (i32.const 3))
                        (drop (call $fd_close (i32.load (i32.const 500))))
                    )
                    (else
                        (i32.store (i32.const 8) (i32.const 1504))
                        (i32.store (i32.const 12) (i32.const 5))
                    )
                )
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 600)))
                (drop (call $fd_write (i32.const 1) (i32.const 8) (i32.const 1) (i32.const 600)))
            )
            (func $preopen (param $fd i32) (param $label i32) (param $label_len i32)
                (local $error i32)
                (local.set $error (call $fd_prestat_get (local.get $fd) (i32.const 700)))
                (i32.store (i32.const 0) (local.get $label))
                (i32.store (i32.const 4) (local.get $label_len))
                (if (i32.eqz (local.get $error))
                    (then
                        (drop (call $fd_prestat_dir_name (local.get $fd) (i32.const 1300) (i32.const 32)))
                        (i32.store (i32.const 8) (i32.const 1300))
                        (i32.store (i32.const 12) (i32.load (i32.const 704)))
                    )
                    (else
                        (i32.store (i32.const 8) (i32.const 1020))
                        (i32.store (i32.const 12) (i32.const 4))
                    )
                )
                (i32.store (i32.const 16) (i32.const 1024))
                (i32.store (i32.const 20) (i32.const 1))
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 720)))
                (drop (call $fd_write (i32.const 1) (i32.const 8) (i32.const 1) (i32.const 720)))
                (drop (call $fd_write (i32.const 1) (i32.const 16) (i32.const 1) (i32.const 720)))
            )
            (func $read_link
                (local $error i32)
                (local.set $error (call $path_open (i32.const 3) (i32.const 0) (i32.const 1400)
                    (i32.const 4) (i32.const 0) (i64.const 2) (i64.const 0) (i32.const 0) (i32.const 800)))
                (if (i32.eqz (local.get $error))
                    (then
                        (i32.store (i32.const 900) (i32.const 2000))
                        (i32.store (i32.const 904) (i32.const 128))
                        (drop (call $fd_read (i32.load (i32.const 800)) (i32.const 900) (i32.const 1) (i32.const 920)))
                        (drop (call $fd_write (i32.const 1) (i32.const 900) (i32.const 1) (i32.const 940)))
                        (drop (call $fd_close (i32.load (i32.const 800))))
                    )
                )
            )
";

/// Builds a fixture that reports preopens and attempts unsafe paths.
pub(crate) fn containment_module(
    cache: &ModuleCache,
    absolute_path: &str,
) -> Result<Arc<Module>, SandboxError> {
    compile(cache, &containment_wat(absolute_path))
}

fn containment_wat(absolute_path: &str) -> String {
    let absolute_len = absolute_path.len();
    let absolute_path = wat_string(absolute_path);
    format!(
        r#"(module
            {CONTAINMENT_IMPORTS}
            {CONTAINMENT_DATA}
            (data (i32.const 1219) "{absolute_path}")
            {CONTAINMENT_HELPERS}
            (func (export "_start")
                (call $preopen (i32.const 3) (i32.const 1000) (i32.const 3))
                (call $preopen (i32.const 4) (i32.const 1004) (i32.const 3))
                (call $preopen (i32.const 5) (i32.const 1008) (i32.const 3))
                (call $report (i32.const 1100) (i32.const 4) (i32.const 1200) (i32.const 11))
                (call $report (i32.const 1105) (i32.const 5) (i32.const 1212) (i32.const 1))
                (call $report (i32.const 1110) (i32.const 7) (i32.const 1214) (i32.const 4))
                (call $report (i32.const 1117) (i32.const 9) (i32.const 1219) (i32.const {absolute_len}))
                (call $report (i32.const 1126) (i32.const 5) (i32.const 1400) (i32.const 4))
                (call $read_link)
            )
        )"#
    )
}
