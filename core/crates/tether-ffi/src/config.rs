//! UniFFI access to global, project, and trust-aware configuration.

use std::fs;
use std::path::{Path, PathBuf};

use tether_engine::config::{self, ConfigIssue, ConfigOrigin, EffectiveConfig};
use tether_types::ProjectId;

use crate::{ExecutorView, ModelRefView, ProjectRuntime, ProjectRuntimeError};

/// A configuration issue shown by the settings screen.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConfigIssueView {
    /// Configuration key or file label.
    pub key: String,
    /// Plain explanation.
    pub message: String,
    /// One-based line, when available.
    pub line: Option<u32>,
    /// One-based column, when available.
    pub column: Option<u32>,
}

impl From<ConfigIssue> for ConfigIssueView {
    fn from(issue: ConfigIssue) -> Self {
        Self {
            key: issue.key,
            message: issue.message,
            line: issue.line.and_then(|value| u32::try_from(value).ok()),
            column: issue.column.and_then(|value| u32::try_from(value).ok()),
        }
    }
}

/// Effective project settings and the source of each setting.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EffectiveConfigView {
    /// Effective executor.
    pub executor_default: ExecutorView,
    /// Origin of the executor setting.
    pub executor_origin: String,
    /// Effective allowed programs.
    pub allowed_commands: Vec<String>,
    /// Origin of allowed programs.
    pub allowed_commands_origin: String,
    /// Quality values as `key=value` strings.
    pub quality: Vec<String>,
    /// Origin of quality values.
    pub quality_origin: String,
    /// Origin of each quality value as `key=origin` strings.
    pub quality_origins: Vec<String>,
    /// Effective model pair, when complete.
    pub model: Option<ModelRefView>,
    /// Origin of the model pair.
    pub model_origin: String,
}

#[uniffi::export]
impl ProjectRuntime {
    /// Trusts a project so its non-quality configuration can apply.
    ///
    /// # Errors
    /// Returns `NotFound` for an unknown project or `Unavailable` for storage errors.
    pub fn trust_project(&self, project_id: String) -> Result<(), ProjectRuntimeError> {
        self.store()?
            .trust_project(&ProjectId::new(project_id), now_ms())
            .map_err(map_store_error)
    }

    /// Revokes a project's configuration trust.
    ///
    /// # Errors
    /// Returns `Unavailable` when storage cannot update the flag.
    pub fn revoke_project_trust(&self, project_id: String) -> Result<(), ProjectRuntimeError> {
        self.store()?
            .revoke_project_trust(&ProjectId::new(project_id))
            .map_err(|_| ProjectRuntimeError::Unavailable)
    }

    /// Returns whether a project is trusted.
    ///
    /// # Errors
    /// Returns `Unavailable` when storage cannot read the flag.
    pub fn is_project_trusted(&self, project_id: String) -> Result<bool, ProjectRuntimeError> {
        self.store()?
            .is_project_trusted(&ProjectId::new(project_id))
            .map_err(|_| ProjectRuntimeError::Unavailable)
    }

    /// Reads project configuration issues without failing the project.
    ///
    /// # Errors
    /// Returns `NotFound` for an unknown project.
    pub fn config_issues(
        &self,
        project_id: String,
    ) -> Result<Vec<ConfigIssueView>, ProjectRuntimeError> {
        let (_, mut issues) = self.project_config(&project_id)?;
        let (_, global_issues) =
            config::load_global(&global_config_path(Path::new(&self.database_path)));
        issues.extend(global_issues);
        Ok(issues.into_iter().map(Into::into).collect())
    }

    /// Returns the effective configuration for a project.
    ///
    /// # Errors
    /// Returns `NotFound` for an unknown project.
    pub fn effective_config(
        &self,
        project_id: String,
    ) -> Result<EffectiveConfigView, ProjectRuntimeError> {
        let (project, _) = self.project_config(&project_id)?;
        let (global, _) = config::load_global(&global_config_path(Path::new(&self.database_path)));
        Ok(view(config::effective_config(
            &global,
            &project,
            &Default::default(),
        )))
    }

    /// Saves global TOML text. Malformed text is reported by `global_config_issues` in a later phase.
    ///
    /// # Errors
    /// Returns `Unavailable` when the configuration file cannot be written.
    pub fn set_global_config(&self, text: String) -> Result<(), ProjectRuntimeError> {
        let path = global_config_path(Path::new(&self.database_path));
        let folder = path.parent().ok_or(ProjectRuntimeError::Unavailable)?;
        fs::create_dir_all(folder).map_err(|_| ProjectRuntimeError::Unavailable)?;
        fs::write(path, text).map_err(|_| ProjectRuntimeError::Unavailable)
    }

    /// Reads global TOML text, if it exists.
    ///
    /// # Errors
    /// Returns `Unavailable` when the file exists but cannot be read.
    pub fn global_config(&self) -> Result<Option<String>, ProjectRuntimeError> {
        match fs::read_to_string(global_config_path(Path::new(&self.database_path))) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(ProjectRuntimeError::Unavailable),
        }
    }
}

impl ProjectRuntime {
    fn project_config(
        &self,
        project_id: &str,
    ) -> Result<(config::ProjectConfig, Vec<ConfigIssue>), ProjectRuntimeError> {
        let store = self.store()?;
        let id = ProjectId::new(project_id);
        let project = store
            .load_project(&id)
            .map_err(|_| ProjectRuntimeError::Unavailable)?
            .ok_or(ProjectRuntimeError::NotFound)?;
        let root = Path::new(&self.workspaces_root).join(&project.path);
        let trusted = store
            .is_project_trusted(&id)
            .map_err(|_| ProjectRuntimeError::Unavailable)?;
        Ok(config::load_project(&root, trusted))
    }
}

fn view(config: EffectiveConfig) -> EffectiveConfigView {
    let model = config
        .model
        .provider
        .zip(config.model.id)
        .map(|(provider, model_id)| ModelRefView {
            provider: provider.into(),
            model_id,
        });
    EffectiveConfigView {
        executor_default: config.executor_default.into(),
        executor_origin: origin(config.executor_origin),
        allowed_commands: config.allowed_commands,
        allowed_commands_origin: origin(config.allowed_commands_origin),
        quality: config
            .quality
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect(),
        quality_origin: origin(config.quality_origin),
        quality_origins: config
            .quality_origins
            .into_iter()
            .map(|(key, value)| format!("{key}={}", origin(value)))
            .collect(),
        model,
        model_origin: origin(config.model_origin),
    }
}

fn origin(origin: ConfigOrigin) -> String {
    match origin {
        ConfigOrigin::BuiltIn => "built_in",
        ConfigOrigin::Global => "global",
        ConfigOrigin::Project => "project",
        ConfigOrigin::Session => "session",
    }
    .into()
}

fn global_config_path(database_path: &Path) -> PathBuf {
    let folder = database_path.parent().unwrap_or_else(|| Path::new("."));
    let files = if folder.file_name().and_then(|name| name.to_str()) == Some("db") {
        folder.parent().unwrap_or(folder)
    } else {
        folder
    };
    files.join("config").join("global.toml")
}

fn map_store_error(error: tether_store::StoreError) -> ProjectRuntimeError {
    match error {
        tether_store::StoreError::MissingProject => ProjectRuntimeError::NotFound,
        _ => ProjectRuntimeError::Unavailable,
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}
