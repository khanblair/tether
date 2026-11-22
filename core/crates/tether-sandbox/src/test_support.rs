//! Shared WASI test fixtures.

use std::sync::Arc;

use wasmtime::Module;

use crate::{ModuleCache, SandboxError};

/// Compiles a small WASI command fixture from WAT.
pub(crate) fn fixture(cache: &ModuleCache, body: &str) -> Result<Arc<Module>, SandboxError> {
    let wat = format!(
        r#"(module
            (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))
            (memory (export "memory") 1)
            {body}
        )"#
    );
    let bytes = wat::parse_str(wat).map_err(|error| SandboxError::Module(error.to_string()))?;
    cache.load(&bytes)
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
