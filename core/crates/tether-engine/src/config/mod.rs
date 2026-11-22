//! Project and global TOML configuration with trust-aware merging.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tether_store::Store;
use tether_types::{Executor, PermissionRule, ProjectId, Provider};
mod parser;
mod serde_text;
use parser::parse_value;
use tether_workspace::PathGuard;

/// A scalar quality setting kept for the quality crate to interpret later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    /// A numeric threshold.
    Number(f64),
    /// A named severity or setting.
    String(String),
}

impl fmt::Display for ConfigValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(value) => write!(formatter, "{value}"),
            Self::String(value) => formatter.write_str(value),
        }
    }
}

/// Values read from one configuration file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Executor settings.
    pub executor: ExecutorConfig,
    /// Permission settings.
    pub permissions: PermissionsConfig,
    /// Quality threshold and severity values.
    pub quality: BTreeMap<String, ConfigValue>,
    /// Default model pair.
    pub model: ModelConfig,
    #[serde(skip)]
    pub(crate) executor_set: bool,
    #[serde(skip)]
    pub(crate) permissions_set: bool,
    #[serde(skip)]
    pub(crate) model_set: bool,
}

/// Global configuration has the same shape as project configuration.
pub type GlobalConfig = ProjectConfig;

/// Executor settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Default executor.
    #[serde(with = "serde_text::executor_text")]
    pub default: Executor,
}

/// Permission settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionsConfig {
    /// Program names allowed for `run`.
    pub allowed_commands: Vec<String>,
}

/// Project model settings.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Provider name.
    #[serde(with = "serde_text::provider_option_text")]
    pub provider: Option<Provider>,
    /// Provider model id.
    pub id: Option<String>,
}

/// Values that can be overridden for one open session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionConfig {
    /// Session-only executor override.
    pub executor_default: Option<Executor>,
}

/// Where one effective setting came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigOrigin {
    /// Built-in default.
    BuiltIn,
    /// Global file.
    Global,
    /// Project file.
    Project,
    /// Session override.
    Session,
}

/// The merged settings used by a session.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveConfig {
    /// Effective default executor.
    pub executor_default: Executor,
    /// Source of the executor default.
    pub executor_origin: ConfigOrigin,
    /// Effective allowed program names.
    pub allowed_commands: Vec<String>,
    /// Source of allowed program names.
    pub allowed_commands_origin: ConfigOrigin,
    /// Effective quality settings.
    pub quality: BTreeMap<String, ConfigValue>,
    /// Source of quality settings.
    pub quality_origin: ConfigOrigin,
    /// Source of each quality setting.
    pub quality_origins: BTreeMap<String, ConfigOrigin>,
    /// Effective model pair.
    pub model: ModelConfig,
    /// Source of the model pair.
    pub model_origin: ConfigOrigin,
}

/// A configuration issue safe to show in the settings screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigIssue {
    /// Configuration key or file label.
    pub key: String,
    /// Plain explanation.
    pub message: String,
    /// One-based TOML line when parsing failed.
    pub line: Option<usize>,
    /// One-based TOML column when parsing failed.
    pub column: Option<usize>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            executor: ExecutorConfig {
                default: Executor::Sandbox,
            },
            permissions: PermissionsConfig {
                allowed_commands: Vec::new(),
            },
            quality: BTreeMap::new(),
            model: ModelConfig::default(),
            executor_set: false,
            permissions_set: false,
            model_set: false,
        }
    }
}

impl Default for EffectiveConfig {
    fn default() -> Self {
        Self {
            executor_default: Executor::Sandbox,
            executor_origin: ConfigOrigin::BuiltIn,
            allowed_commands: Vec::new(),
            allowed_commands_origin: ConfigOrigin::BuiltIn,
            quality: BTreeMap::new(),
            quality_origin: ConfigOrigin::BuiltIn,
            quality_origins: BTreeMap::new(),
            model: ModelConfig::default(),
            model_origin: ConfigOrigin::BuiltIn,
        }
    }
}

/// Parses TOML without allowing malformed or unknown data to fail a session.
#[must_use]
pub fn parse(text: &str) -> (ProjectConfig, Vec<ConfigIssue>) {
    let value = match toml::from_str::<toml::Table>(text) {
        Ok(table) => toml::Value::Table(table),
        Err(error) => {
            let (line, column) = error.span().map_or((None, None), |span| {
                let prefix = &text[..span.start.min(text.len())];
                let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
                let column = prefix
                    .rsplit('\n')
                    .next()
                    .map_or(1, |part| part.chars().count() + 1);
                (Some(line), Some(column))
            });
            return (
                ProjectConfig::default(),
                vec![ConfigIssue {
                    key: "file".into(),
                    message: format!("malformed TOML: {error}"),
                    line,
                    column,
                }],
            );
        }
    };
    parse_value(&value)
}

/// Reads `.tether/config.toml` through a workspace guard.
#[must_use]
pub fn load_project(root: &Path, trusted: bool) -> (ProjectConfig, Vec<ConfigIssue>) {
    let Ok(guard) = PathGuard::new(root) else {
        return (
            ProjectConfig::default(),
            vec![issue("file", "project root is unavailable")],
        );
    };
    let Ok(path) = guard.resolve(".tether/config.toml") else {
        return (ProjectConfig::default(), Vec::new());
    };
    let Ok(text) = fs::read_to_string(path) else {
        return (ProjectConfig::default(), Vec::new());
    };
    let (mut config, mut issues) = parse(&text);
    if !trusted {
        if config.permissions_set {
            config.permissions.allowed_commands.clear();
            config.permissions_set = false;
            issues.push(issue(
                "permissions.allowed_commands",
                "ignored until the project is trusted",
            ));
        }
        if config.model_set {
            config.model = ModelConfig::default();
            config.model_set = false;
            issues.push(issue("model", "ignored until the project is trusted"));
        }
        if config.executor_set && config.executor.default != Executor::Sandbox {
            config.executor.default = Executor::Sandbox;
            config.executor_set = false;
            issues.push(issue(
                "executor.default",
                "ignored until the project is trusted",
            ));
        }
    }
    (config, issues)
}

/// Reads the global TOML file, falling back to defaults on any file error.
#[must_use]
pub fn load_global(path: &Path) -> (GlobalConfig, Vec<ConfigIssue>) {
    match fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(_) => (GlobalConfig::default(), Vec::new()),
    }
}

/// Merges built-ins, global, project, and session settings in that order.
#[must_use]
pub fn effective_config(
    global: &GlobalConfig,
    project: &ProjectConfig,
    session: &SessionConfig,
) -> EffectiveConfig {
    let mut result = EffectiveConfig::default();
    apply_layer(&mut result, global, ConfigOrigin::Global);
    apply_layer(&mut result, project, ConfigOrigin::Project);
    if let Some(executor) = session.executor_default {
        result.executor_default = executor;
        result.executor_origin = ConfigOrigin::Session;
    }
    result
}

/// Copies every setting one file set over the result, tagging each with `origin`.
fn apply_layer(result: &mut EffectiveConfig, layer: &ProjectConfig, origin: ConfigOrigin) {
    if layer.executor_set {
        result.executor_default = layer.executor.default;
        result.executor_origin = origin;
    }
    if layer.permissions_set {
        result
            .allowed_commands
            .clone_from(&layer.permissions.allowed_commands);
        result.allowed_commands_origin = origin;
    }
    if !layer.quality.is_empty() {
        for (key, value) in &layer.quality {
            result.quality.insert(key.clone(), value.clone());
            result.quality_origins.insert(key.clone(), origin);
        }
        result.quality_origin = origin;
    }
    if layer.model_set {
        result.model = layer.model.clone();
        result.model_origin = origin;
    }
}

/// Converts trusted allowed programs to in-memory sandbox and Linux rules.
#[must_use]
pub fn allowed_command_rules(config: &EffectiveConfig) -> Vec<PermissionRule> {
    config
        .allowed_commands
        .iter()
        .flat_map(|program| {
            [
                tether_types::Executor::Sandbox,
                tether_types::Executor::Linux,
            ]
            .into_iter()
            .map(move |executor| PermissionRule {
                id: format!("config:{program}:{}", executor.as_str()),
                tool: "run".into(),
                pattern: program.clone(),
                executor: Some(executor),
            })
        })
        .collect()
}

/// Loads trusted project command rules for one permission check.
///
/// # Errors
/// Returns a store error when the trust flag cannot be read.
pub fn project_allowed_command_rules(
    store: &Store,
    project_id: &ProjectId,
    root: &Path,
) -> Result<Vec<PermissionRule>, tether_store::StoreError> {
    if !store.is_project_trusted(project_id)? {
        return Ok(Vec::new());
    }
    let (project, _) = load_project(root, true);
    Ok(allowed_command_rules(&EffectiveConfig {
        allowed_commands: project.permissions.allowed_commands,
        ..EffectiveConfig::default()
    }))
}

fn issue(key: &str, message: &str) -> ConfigIssue {
    ConfigIssue {
        key: key.into(),
        message: message.into(),
        line: None,
        column: None,
    }
}

#[cfg(test)]
mod tests;
