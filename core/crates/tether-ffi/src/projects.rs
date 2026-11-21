//! Project creation and listing for the platform.

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tether_store::{Store, StoreError};
use tether_types::{ModelRef, PermissionMode, ProjectId};

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
    database_path: String,
    workspaces_root: String,
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
    fn store(&self) -> Result<Store, ProjectRuntimeError> {
        Store::open(Path::new(&self.database_path)).map_err(|_| ProjectRuntimeError::Unavailable)
    }
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
