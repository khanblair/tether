//! WASM sandbox executor on Wasmtime.

mod limits;
mod module_cache;
mod pack;
mod pack_install;
mod pack_io;
#[cfg(test)]
mod pack_tests;
mod runner;
mod runner_support;
#[cfg(test)]
mod runner_tests;
#[cfg(test)]
mod test_support;
mod wasi_context;

pub use limits::{CappedOutput, RunLimits};
pub use module_cache::{ModuleCache, SandboxError};
pub use pack::{PackEntry, PackError, PackFetcher, PackFile, PackManifest, verify_manifest};
pub use pack_install::{InstalledPack, install_pack, program_for};
pub use runner::{RunOutcome, RunReport, RunSpec, SandboxRunner};
pub use wasi_context::{ContextError, OutputChannel, OutputSink, StreamWriter, WasiContextSpec};
