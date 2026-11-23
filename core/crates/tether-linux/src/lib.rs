//! Linux mode process execution and Alpine root filesystem management.

mod error;
mod extract;
mod manifest;
mod packages;
mod proot;
mod rootfs;
mod rootfs_io;
mod runner;
mod shell;

pub use error::LinuxError;
pub use manifest::{RootfsFetcher, RootfsImage, RootfsManifest, verify_rootfs_manifest};
pub use packages::{SetupRequest, install_base_packages, setup};
pub use portable_pty::PtySize;
pub use proot::{NetworkPolicy, ProotConfig, SessionKind, classify_network_failure};
pub use rootfs::{NoopProgress, RootfsManager, RootfsProgress, RootfsStatus};
pub use runner::{
    LinuxRunReport, LinuxRunner, RunCommand, RunOutcome, RunRequest, RunRequestCommand, RunSpec,
};
pub use shell::LinuxShell;
pub use tether_sandbox::{CappedOutput, OutputSink};
