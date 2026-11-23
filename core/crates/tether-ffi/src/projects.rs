//! Project creation and listing for the platform.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_store::{Store, StoreError};
use tether_types::{ModelRef, PermissionMode, ProjectId};
use tether_workspace::archive::{ArchiveError, CreateOptions, create_zip, extract_zip};
use tokio_util::sync::CancellationToken;

use crate::archive::{archive_detail, extract_limits, is_archive_input_error};
use crate::{PermissionModeView, ProjectView, ProviderView};

/// A project operation failed.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ProjectRuntimeError {
    /// The project name is invalid.
    #[error("invalid project name")]
    InvalidName,
    /// A default provider and model id were not both set or both absent.
    #[error("invalid project defaults")]
    InvalidDefaults,
    /// Bypass needs an in-memory app confirmation.
    #[error("bypass needs confirmation")]
    BypassNeedsConfirmation,
    /// The archive input or output is invalid.
    #[error("invalid project archive: {detail}")]
    Archive {
        /// Short archive failure detail.
        detail: String,
    },
    /// The archive operation was cancelled.
    #[error("project archive operation cancelled")]
    Cancelled,
    /// The requested project does not exist.
    #[error("project not found")]
    NotFound,
    /// The database or workspace directory is unavailable.
    #[error("project is unavailable")]
    Unavailable,
}

/// Opens projects in the app database and creates their workspace directories.
#[derive(uniffi::Object)]
pub struct ProjectRuntime {
    pub(crate) database_path: String,
    pub(crate) workspaces_root: String,
    active_import: Mutex<Option<Arc<CancellationToken>>>,
}

#[uniffi::export]
impl ProjectRuntime {
    /// Opens the project database.
    ///
    /// # Errors
    /// Returns an error when the database cannot open.
    #[uniffi::constructor]
    pub fn new(
        database_path: &str,
        workspaces_root: &str,
    ) -> Result<Arc<Self>, ProjectRuntimeError> {
        Store::open(Path::new(database_path)).map_err(|_| ProjectRuntimeError::Unavailable)?;
        Ok(Arc::new(Self {
            database_path: database_path.into(),
            workspaces_root: workspaces_root.into(),
            active_import: Mutex::new(None),
        }))
    }

    /// Saves a project and creates its workspace directory.
    ///
    /// # Errors
    /// Returns an error for an invalid name or unavailable storage.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "UniFFI project input is owned"
    )]
    pub fn create_project(&self, name: String) -> Result<ProjectView, ProjectRuntimeError> {
        let mut store = self.store()?;
        let project = store
            .create_project(&name, now_ms()?)
            .map_err(|error| match error {
                StoreError::InvalidProjectName => ProjectRuntimeError::InvalidName,
                _ => ProjectRuntimeError::Unavailable,
            })?;
        if std::fs::create_dir_all(Path::new(&self.workspaces_root).join(&project.path)).is_err() {
            store
                .delete_project(&project.id)
                .map_err(|_| ProjectRuntimeError::Unavailable)?;
            return Err(ProjectRuntimeError::Unavailable);
        }
        Ok(project.into())
    }

    /// Imports a ZIP into a new project workspace and returns its project id.
    ///
    /// # Errors
    /// Returns an archive, name, storage, or cancellation error. Failed imports remove both the
    /// workspace folder and its database row.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "UniFFI archive input is owned"
    )]
    pub fn import_project_zip(
        &self,
        zip_path: String,
        name: Option<String>,
    ) -> Result<String, ProjectRuntimeError> {
        let cancel = Arc::new(CancellationToken::new());
        *self
            .active_import
            .lock()
            .map_err(|_| ProjectRuntimeError::Unavailable)? = Some(Arc::clone(&cancel));
        let result = self.import_with_cancel(&zip_path, name, &|| cancel.is_cancelled());
        self.clear_import(&cancel)?;
        result
    }

    /// Cancels the project import currently in flight, if any.
    pub fn cancel_import(&self) {
        if let Ok(active) = self.active_import.lock()
            && let Some(token) = active.as_ref()
        {
            token.cancel();
        }
    }

    /// Exports one project workspace to a ZIP file.
    ///
    /// # Errors
    /// Returns an unknown-project, destination, archive, or storage error.
    pub fn export_project_zip(
        &self,
        project_id: String,
        dest_path: String,
        include_ignored: bool,
    ) -> Result<(), ProjectRuntimeError> {
        let store = self.store()?;
        let relative = store
            .project_path(&ProjectId::new(project_id))
            .map_err(|_| ProjectRuntimeError::Unavailable)?
            .ok_or(ProjectRuntimeError::NotFound)?;
        let root = Path::new(&self.workspaces_root).join(relative);
        let options = CreateOptions {
            include_ignored,
            ..CreateOptions::default()
        };
        create_zip(root, dest_path, &options, &|| false)
            .map(|_| ())
            .map_err(|error| map_project_archive_error(&error))
    }

    /// Lists projects by newest activity.
    ///
    /// # Errors
    /// Returns an error when the database cannot be read.
    pub fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectRuntimeError> {
        self.store()?
            .list_projects()
            .map(|projects| projects.into_iter().map(Into::into).collect())
            .map_err(|_| ProjectRuntimeError::Unavailable)
    }

    /// Loads a saved project by id.
    ///
    /// # Errors
    /// Returns an error when the project is absent or storage is unavailable.
    pub fn load_project(&self, id: String) -> Result<ProjectView, ProjectRuntimeError> {
        self.store()?
            .load_project(&ProjectId::new(id))
            .map_err(|_| ProjectRuntimeError::Unavailable)?
            .map(Into::into)
            .ok_or(ProjectRuntimeError::NotFound)
    }

    /// Saves a project's model and permission defaults.
    ///
    /// # Errors
    /// Returns an error for an incomplete model pair, unknown project, or unavailable storage.
    pub fn set_project_defaults(
        &self,
        project_id: String,
        provider: Option<ProviderView>,
        model_id: Option<String>,
        mode: Option<PermissionModeView>,
    ) -> Result<(), ProjectRuntimeError> {
        let model = match (provider, model_id) {
            (Some(provider), Some(model_id)) => Some(ModelRef {
                provider: provider.into(),
                model_id,
            }),
            (None, None) => None,
            _ => return Err(ProjectRuntimeError::InvalidDefaults),
        };
        self.store()?
            .set_project_defaults(
                &ProjectId::new(project_id),
                model,
                mode.map(permission_mode),
            )
            .map_err(|error| match error {
                StoreError::MissingProject => ProjectRuntimeError::NotFound,
                StoreError::BypassNeedsConfirmation => ProjectRuntimeError::BypassNeedsConfirmation,
                _ => ProjectRuntimeError::Unavailable,
            })
    }
}

fn permission_mode(mode: PermissionModeView) -> PermissionMode {
    match mode {
        PermissionModeView::Plan => PermissionMode::Plan,
        PermissionModeView::Ask => PermissionMode::Ask,
        PermissionModeView::AcceptEdits => PermissionMode::AcceptEdits,
        PermissionModeView::Auto => PermissionMode::Auto,
        PermissionModeView::Bypass => PermissionMode::Bypass,
    }
}

impl ProjectRuntime {
    pub(crate) fn store(&self) -> Result<Store, ProjectRuntimeError> {
        Store::open(Path::new(&self.database_path)).map_err(|_| ProjectRuntimeError::Unavailable)
    }

    /// Imports a ZIP into a new project, stopping when `cancelled` says so. A failed or
    /// cancelled import removes both the workspace folder and its database row.
    pub(crate) fn import_with_cancel(
        &self,
        zip_path: &str,
        name: Option<String>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<String, ProjectRuntimeError> {
        let project_name = name.unwrap_or_else(|| archive_project_name(zip_path));
        let mut store = self.store()?;
        let project = store
            .create_project(&project_name, now_ms()?)
            .map_err(|error| map_store_error(&error))?;
        let destination = Path::new(&self.workspaces_root).join(&project.path);
        if fs::create_dir_all(&destination).is_err() {
            return import_cleanup(
                &mut store,
                &project.id,
                &destination,
                ProjectRuntimeError::Unavailable,
            );
        }
        let result = extract_zip(zip_path, &destination, &extract_limits(), cancelled, true);
        match result {
            Ok(_) => Ok(project.id.to_string()),
            Err(error) => import_cleanup(
                &mut store,
                &project.id,
                &destination,
                map_project_archive_error(&error),
            ),
        }
    }

    fn clear_import(&self, token: &Arc<CancellationToken>) -> Result<(), ProjectRuntimeError> {
        let mut active = self
            .active_import
            .lock()
            .map_err(|_| ProjectRuntimeError::Unavailable)?;
        if active
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, token))
        {
            *active = None;
        }
        Ok(())
    }
}

fn import_cleanup(
    store: &mut Store,
    project_id: &ProjectId,
    destination: &Path,
    error: ProjectRuntimeError,
) -> Result<String, ProjectRuntimeError> {
    drop(fs::remove_dir_all(destination));
    store
        .delete_project(project_id)
        .map_err(|_| ProjectRuntimeError::Unavailable)?;
    Err(error)
}

fn map_store_error(error: &StoreError) -> ProjectRuntimeError {
    match error {
        StoreError::InvalidProjectName => ProjectRuntimeError::InvalidName,
        _ => ProjectRuntimeError::Unavailable,
    }
}

fn map_project_archive_error(error: &ArchiveError) -> ProjectRuntimeError {
    if matches!(error, ArchiveError::Cancelled) {
        return ProjectRuntimeError::Cancelled;
    }
    if is_archive_input_error(error) || matches!(error, ArchiveError::InvalidDestination) {
        return ProjectRuntimeError::Archive {
            detail: archive_detail(error).to_owned(),
        };
    }
    ProjectRuntimeError::Unavailable
}

fn archive_project_name(zip_path: &str) -> String {
    PathBuf::from(zip_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("project")
        .to_owned()
}

fn now_ms() -> Result<i64, ProjectRuntimeError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .ok_or(ProjectRuntimeError::Unavailable)
}

#[cfg(test)]
mod tests;
