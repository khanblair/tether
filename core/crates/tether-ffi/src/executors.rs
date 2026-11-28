//! UniFFI configuration for local command executors.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tether_engine::config;
use tether_engine::executors::{CiExecutor, NotConfigured, RunTools};
use tether_linux::{LinuxRunner, NetworkPolicy, ProotConfig, RootfsManager, RootfsStatus};
use tether_sandbox::installed_programs;
use tether_store::{Store, project_dir};
use tether_types::{ProjectId, SessionId};

use crate::{LinuxShellSession, LinuxShellSink, SessionRuntime};

#[path = "executors_download.rs"]
mod downloads;
pub use downloads::{ExecutorProgress, RuntimePackView};

#[path = "executors_paths.rs"]
mod paths;
use paths::{ExecutorPaths, files_root, read_paths, validate_paths};

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
    /// Whether Linux mode has PRoot, its loader, and a ready root filesystem.
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
    /// The download did not pass its signature or content check.
    #[error("the download did not pass its signature check")]
    VerificationFailed,
    /// The requested project does not exist.
    #[error("project not found")]
    ProjectNotFound,
    /// Linux mode is not installed or cannot be used yet.
    #[error("Linux mode is not ready")]
    LinuxNotReady,
    /// One interactive Linux shell is already open for this executor runtime.
    #[error("a Linux shell is already open")]
    LinuxShellAlreadyOpen,
}

#[uniffi::export]
impl SessionRuntime {
    /// Stores platform executor paths for later session opens.
    ///
    /// # Errors
    /// Returns an error when a supplied path is unsafe, relative, or incomplete.
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
            && paths.loader_path.is_file()
            && matches!(
                RootfsManager::new(paths.linux_root).status(),
                Ok(RootfsStatus::Ready { .. })
            );
        let ci_ready = ci_executor().is_configured();
        Ok(ExecutorAvailabilityView {
            sandbox_programs,
            linux_ready,
            ci_ready,
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

    /// Opens one interactive shell in the Alpine guest.
    ///
    /// When `project_id` is present, its workspace is mounted at `/workspace` and its saved
    /// network policy is used. Output and exit callbacks run on a Rust reader thread.
    ///
    /// # Errors
    /// Returns an error when Linux is not ready, the project is missing, or a shell is open.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "UniFFI transfers the optional project id as an owned value"
    )]
    pub fn open_linux_shell(
        &self,
        project_id: Option<String>,
        rows: u16,
        cols: u16,
        sink: Arc<dyn LinuxShellSink>,
    ) -> Result<Arc<LinuxShellSession>, ExecutorRuntimeError> {
        let paths = read_paths(&self.files_root)?;
        let (workspace, network) = self.linux_shell_project(project_id.as_deref())?;
        let config = linux_shell_config(&paths, workspace, network)?;
        let lease = self.linux_shell_slot.acquire()?;
        LinuxShellSession::open(&config, rows, cols, sink, lease)
    }
}

impl SessionRuntime {
    fn linux_shell_project(
        &self,
        project_id: Option<&str>,
    ) -> Result<(Option<PathBuf>, NetworkPolicy), ExecutorRuntimeError> {
        let Some(project_id) = project_id else {
            return Ok((None, NetworkPolicy::Off));
        };
        let store = self
            .store()
            .map_err(|_| ExecutorRuntimeError::Unavailable)?;
        let id = ProjectId::new(project_id);
        let project = store
            .load_project(&id)
            .map_err(|_| ExecutorRuntimeError::Unavailable)?
            .ok_or(ExecutorRuntimeError::ProjectNotFound)?;
        let workspace = project_dir(&self.files_root.join("workspaces"), &project);
        let network = if store
            .project_network(&id)
            .map_err(|_| ExecutorRuntimeError::Unavailable)?
        {
            NetworkPolicy::On
        } else {
            NetworkPolicy::Off
        };
        Ok((Some(workspace), network))
    }
}

/// Returns the CI seam used by session tool wiring.
pub(crate) fn ci_executor() -> Arc<dyn CiExecutor> {
    Arc::new(NotConfigured)
}

/// Builds the run handler for one project workspace.
pub(crate) fn session_run_tools(
    database_path: &Path,
    project_root: &Path,
    project_id: &ProjectId,
) -> Result<RunTools, String> {
    let files_root = files_root(database_path);
    let Ok(paths) = read_paths(&files_root) else {
        return RunTools::new(files_root.join("runtimes"));
    };
    let runners = linux_runners(&paths, project_root);
    let database = database_path.to_owned();
    let project = project_id.clone();
    let network = Arc::new(move |_session: &SessionId| {
        Store::open(&database)
            .and_then(|store| store.project_network(&project))
            .unwrap_or(false)
    });
    let project_default = project_default(database_path, project_root, project_id);
    let remote_project = project_id.clone();
    RunTools::new(paths.packs_root).map(|tools| {
        tools
            .with_network_runners(runners, ci_executor(), project_default, network)
            .with_project_id(Arc::new(move |_| remote_project.clone()))
    })
}

fn project_default(
    database_path: &Path,
    project_root: &Path,
    project_id: &ProjectId,
) -> Option<tether_types::Executor> {
    let store = Store::open(database_path).ok()?;
    let trusted = store.is_project_trusted(project_id).ok()?;
    let (project, _) = config::load_project(project_root, trusted);
    let global_path = files_root(database_path).join("config/global.toml");
    let (global, _) = config::load_global(&global_path);
    Some(
        config::effective_config(&global, &project, &config::SessionConfig::default())
            .executor_default,
    )
}

fn linux_runners(
    paths: &ExecutorPaths,
    project_root: &Path,
) -> (Option<Arc<LinuxRunner>>, Option<Arc<LinuxRunner>>) {
    let Some(proot_path) = paths.proot_path.as_ref() else {
        return (None, None);
    };
    let ready = proot_path.is_file()
        && paths.loader_path.is_file()
        && matches!(
            RootfsManager::new(&paths.linux_root).status(),
            Ok(RootfsStatus::Ready { .. })
        );
    if !ready {
        return (None, None);
    }
    let make = |network| {
        Arc::new(LinuxRunner::new(ProotConfig {
            proot_path: proot_path.clone(),
            loader_path: paths.loader_path.clone(),
            linux_root: paths.linux_root.clone(),
            workspace: Some(project_root.to_owned()),
            network,
        }))
    };
    (
        Some(make(NetworkPolicy::Off)),
        Some(make(NetworkPolicy::On)),
    )
}

fn linux_shell_config(
    paths: &ExecutorPaths,
    workspace: Option<PathBuf>,
    network: NetworkPolicy,
) -> Result<ProotConfig, ExecutorRuntimeError> {
    let Some(proot_path) = paths.proot_path.clone() else {
        return Err(ExecutorRuntimeError::LinuxNotReady);
    };
    let ready = proot_path.is_file()
        && paths.loader_path.is_file()
        && matches!(
            RootfsManager::new(&paths.linux_root).status(),
            Ok(RootfsStatus::Ready { .. })
        );
    if !ready {
        return Err(ExecutorRuntimeError::LinuxNotReady);
    }
    Ok(ProotConfig {
        proot_path,
        loader_path: paths.loader_path.clone(),
        linux_root: paths.linux_root.clone(),
        workspace,
        network,
    })
}

fn now_ms() -> Result<i64, ExecutorRuntimeError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_millis()).ok())
        .ok_or(ExecutorRuntimeError::Unavailable)
}

#[cfg(test)]
#[path = "executors_tests.rs"]
mod tests;
