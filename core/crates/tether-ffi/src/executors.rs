//! UniFFI configuration for local command executors.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_linux::{RootfsManager, RootfsStatus};
use tether_sandbox::installed_programs;
use tether_types::ProjectId;

use crate::{SessionRuntime, SessionRuntimeError};

/// Platform paths and verifying keys for the local executors.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ExecutorPathsView {
    /// Folder containing installed runtime packs.
    pub packs_root: String,
    /// Private Linux mode folder.
    pub linux_root: String,
    /// Absolute PRoot executable path.
    pub proot_path: Option<String>,
    /// Absolute PRoot loader path.
    pub proot_loader_path: Option<String>,
    /// Hex Ed25519 key for runtime packs.
    pub pack_verifying_key_hex: Option<String>,
    /// Hex Ed25519 key for the Linux root filesystem.
    pub rootfs_verifying_key_hex: Option<String>,
}

/// Current executor readiness shown by the platform.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ExecutorAvailabilityView {
    /// Verified sandbox program names.
    pub sandbox_programs: Vec<String>,
    /// Whether Linux mode has PRoot and a ready root filesystem.
    pub linux_ready: bool,
    /// Whether CI has been configured.
    pub ci_ready: bool,
}

/// Errors from executor configuration and project network settings.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ExecutorRuntimeError {
    /// A path or key setting was invalid.
    #[error("invalid executor configuration")]
    InvalidConfiguration,
    /// The database or local configuration could not be read or written.
    #[error("executor configuration unavailable")]
    Unavailable,
    /// The requested project does not exist.
    #[error("project not found")]
    ProjectNotFound,
}

#[uniffi::export]
impl SessionRuntime {
    /// Stores platform executor paths for later session opens.
    ///
    /// # Errors
    /// Returns an error when a supplied path contains unsafe control characters.
    pub fn configure_executors(
        &self,
        paths: ExecutorPathsView,
    ) -> Result<(), ExecutorRuntimeError> {
        validate_paths(&paths)?;
        let config = self.files_root.join("executors");
        fs::create_dir_all(&config).map_err(|_| ExecutorRuntimeError::Unavailable)?;
        let mut lines = vec![
            format!("packs_root={}", paths.packs_root),
            format!("linux_root={}", paths.linux_root),
        ];
        if let Some(path) = paths.proot_path {
            lines.push(format!("proot_path={path}"));
        }
        if let Some(path) = paths.proot_loader_path {
            lines.push(format!("loader_path={path}"));
        }
        if let Some(key) = paths.pack_verifying_key_hex {
            lines.push(format!("pack_key={key}"));
        }
        if let Some(key) = paths.rootfs_verifying_key_hex {
            lines.push(format!("rootfs_key={key}"));
        }
        fs::write(config.join("paths.conf"), lines.join("\n"))
            .map_err(|_| ExecutorRuntimeError::Unavailable)
    }

    /// Reads verified pack names and local Linux readiness.
    ///
    /// # Errors
    /// Returns an error when local executor configuration cannot be read.
    pub fn executor_availability(&self) -> Result<ExecutorAvailabilityView, ExecutorRuntimeError> {
        let paths = read_paths(&self.files_root)?;
        let sandbox_programs = installed_programs(&paths.packs_root).into_iter().collect();
        let linux_ready = paths
            .proot_path
            .as_ref()
            .is_some_and(|path| Path::new(path).is_file())
            && matches!(
                RootfsManager::new(paths.linux_root).status(),
                Ok(RootfsStatus::Ready { .. })
            );
        Ok(ExecutorAvailabilityView {
            sandbox_programs,
            linux_ready,
            ci_ready: false,
        })
    }

    /// Enables or disables network access for one project.
    ///
    /// # Errors
    /// Returns an error when the project or database is unavailable.
    pub fn set_project_network(
        &self,
        project_id: String,
        allowed: bool,
    ) -> Result<(), ExecutorRuntimeError> {
        self.store()
            .map_err(|_| ExecutorRuntimeError::Unavailable)?
            .set_project_network(&ProjectId::new(project_id), allowed, now_ms()?)
            .map_err(|error| match error {
                tether_store::StoreError::MissingProject => ExecutorRuntimeError::ProjectNotFound,
                _ => ExecutorRuntimeError::Unavailable,
            })
    }

    /// Reads one project's network access setting. It is off by default.
    ///
    /// # Errors
    /// Returns an error when the project or database is unavailable.
    pub fn project_network(&self, project_id: String) -> Result<bool, ExecutorRuntimeError> {
        self.store()
            .map_err(|_| ExecutorRuntimeError::Unavailable)?
            .project_network(&ProjectId::new(project_id))
            .map_err(|error| match error {
                tether_store::StoreError::MissingProject => ExecutorRuntimeError::ProjectNotFound,
                _ => ExecutorRuntimeError::Unavailable,
            })
    }
}

#[derive(Debug)]
struct ExecutorPaths {
    packs_root: PathBuf,
    linux_root: PathBuf,
    proot_path: Option<PathBuf>,
}

fn validate_paths(paths: &ExecutorPathsView) -> Result<(), ExecutorRuntimeError> {
    let values = [
        paths.packs_root.as_str(),
        paths.linux_root.as_str(),
        paths.proot_path.as_deref().unwrap_or(""),
        paths.proot_loader_path.as_deref().unwrap_or(""),
    ];
    if values.iter().any(|value| {
        value
            .bytes()
            .any(|byte| byte == 0 || byte == b'\n' || byte == b'\r')
    }) {
        return Err(ExecutorRuntimeError::InvalidConfiguration);
    }
    Ok(())
}

fn read_paths(files_root: &Path) -> Result<ExecutorPaths, ExecutorRuntimeError> {
    let config = files_root.join("executors/paths.conf");
    let text = fs::read_to_string(config).map_err(|_| ExecutorRuntimeError::Unavailable)?;
    let mut packs_root = None;
    let mut linux_root = None;
    let mut proot_path = None;
    for line in text.lines() {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        match name {
            "packs_root" => packs_root = Some(PathBuf::from(value)),
            "linux_root" => linux_root = Some(PathBuf::from(value)),
            "proot_path" => proot_path = Some(PathBuf::from(value)),
            "loader_path" | "pack_key" | "rootfs_key" => {}
            _ => {}
        }
    }
    Ok(ExecutorPaths {
        packs_root: packs_root.ok_or(ExecutorRuntimeError::Unavailable)?,
        linux_root: linux_root.ok_or(ExecutorRuntimeError::Unavailable)?,
        proot_path,
    })
}

fn now_ms() -> Result<i64, ExecutorRuntimeError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_millis()).ok())
        .ok_or(ExecutorRuntimeError::Unavailable)
}
