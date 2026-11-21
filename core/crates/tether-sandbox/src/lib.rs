//! WASM sandbox executor on Wasmtime.

mod limits;
mod module_cache;
mod pack;
mod pack_catalog;
mod pack_cleanup;
mod pack_install;
mod pack_io;
mod pack_replace;
#[cfg(test)]
mod pack_tests;
mod runner;
mod runner_result;
mod runner_support;
#[cfg(test)]
mod runner_tests;
mod runner_validation;
#[cfg(test)]
mod test_support;
mod wasi_context;

pub use limits::{CappedOutput, RunLimits};
pub use module_cache::{ModuleCache, SandboxError};
pub use pack::{
    PackEntry, PackError, PackFetcher, PackFile, PackManifest, PackMount, verify_manifest,
};
pub use pack_catalog::installed_programs;
pub use pack_install::{InstalledPack, RunSpecParts, install_pack, program_for};
pub use runner::{ExtraMount, RunOutcome, RunReport, RunSpec, SandboxRunner};
pub use wasi_context::{ContextError, OutputChannel, OutputSink, StreamWriter, WasiContextSpec};
