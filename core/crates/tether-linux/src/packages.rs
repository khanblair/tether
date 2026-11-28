use std::collections::VecDeque;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::manifest::{RootfsFetcher, RootfsImage, RootfsManifest, validate_manifest};
use crate::proot::{NetworkPolicy, ProotConfig};
use crate::rootfs::{RootfsManager, RootfsProgress, RootfsStatus};
use crate::runner::{LinuxRunner, RunCommand, RunRequest};
use crate::{LinuxError, OutputSink};

/// Maximum time allowed for installing Alpine's base packages.
const PACKAGE_INSTALL_TIMEOUT: Duration = Duration::from_secs(900);

/// Inputs for signed Linux environment setup.
pub struct SetupRequest<'a> {
    /// Exact signed manifest bytes.
    pub manifest_bytes: &'a [u8],
    /// Detached manifest signature.
    pub signature: &'a [u8],
    /// Manifest verification key.
    pub key: &'a [u8; 32],
    /// Image selected for the device architecture.
    pub image: &'a RootfsImage,
    /// PRoot process configuration.
    pub config: &'a ProotConfig,
    /// Root filesystem archive source.
    pub fetcher: &'a dyn RootfsFetcher,
    /// Setup progress receiver.
    pub progress: &'a mut dyn RootfsProgress,
    /// Cancellation token.
    pub cancel: &'a CancellationToken,
}

/// Installs the manifest's base packages with networking enabled for this call only.
///
/// # Errors
/// Returns a validation, process, cancellation, or package installation error.
pub fn install_base_packages(
    manager: &RootfsManager,
    config: &ProotConfig,
    manifest: &RootfsManifest,
    _progress: &mut dyn RootfsProgress,
    cancel: &CancellationToken,
) -> Result<(), LinuxError> {
    validate_manifest(manifest)?;
    let mut setup_config = config.clone();
    setup_config.network = NetworkPolicy::On;
    let runner = LinuxRunner::new(setup_config);
    let mut command = vec!["apk".into(), "add".into(), "--no-cache".into()];
    command.extend(manifest.base_packages.iter().cloned());
    let request = RunRequest {
        command: RunCommand::Argv(command),
        cwd: "/".into(),
        extra_env: Vec::new(),
        timeout: Some(PACKAGE_INSTALL_TIMEOUT),
        output_cap_bytes: 1024 * 1024,
    };
    let mut tail = TailOutput::new(4096);
    let report = runner.run(&request, &mut tail, cancel)?;
    if !matches!(report.outcome, crate::RunOutcome::Exited(0)) {
        return Err(LinuxError::Packages(tail.text()));
    }
    manager.mark_ready()
}

/// Verifies a signed manifest, installs its image, then installs base packages.
///
/// # Errors
/// Returns any manifest, download, extraction, process, or cancellation failure.
pub fn setup(
    manager: &RootfsManager,
    request: &mut SetupRequest<'_>,
) -> Result<RootfsManifest, LinuxError> {
    let manifest =
        crate::verify_rootfs_manifest(request.manifest_bytes, request.signature, request.key)?;
    if !manifest
        .images
        .iter()
        .any(|candidate| candidate.sha256 == request.image.sha256)
    {
        return Err(LinuxError::Manifest(
            "selected image is not in manifest".into(),
        ));
    }
    match manager.status()? {
        RootfsStatus::Ready { .. } => return Ok(manifest),
        RootfsStatus::NeedsRepair { .. } => {
            manager.repair(request.fetcher, request.progress, request.cancel)?;
        }
        RootfsStatus::NotInstalled => manager.install(
            request.image,
            request.fetcher,
            request.progress,
            request.cancel,
        )?,
        RootfsStatus::Installed { .. } => {}
    }
    manager.set_version(&manifest.alpine_version)?;
    install_base_packages(
        manager,
        request.config,
        &manifest,
        request.progress,
        request.cancel,
    )?;
    Ok(manifest)
}

struct TailOutput {
    bytes: VecDeque<u8>,
    capacity: usize,
}

impl TailOutput {
    fn new(capacity: usize) -> Self {
        Self {
            bytes: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn append(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.append_byte(*byte);
        }
    }

    fn append_byte(&mut self, byte: u8) {
        if self.bytes.len() == self.capacity {
            let _ = self.bytes.pop_front();
        }
        self.bytes.push_back(byte);
    }

    fn text(&self) -> String {
        let raw: Vec<u8> = self.bytes.iter().copied().collect();
        String::from_utf8_lossy(&raw).replace('\n', " ")
    }
}

impl OutputSink for TailOutput {
    fn stdout(&mut self, bytes: &[u8]) {
        self.append(bytes);
    }
    fn stderr(&mut self, bytes: &[u8]) {
        self.append(bytes);
    }
}
