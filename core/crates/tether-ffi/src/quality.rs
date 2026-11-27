//! UniFFI access to quality gate settings and built-in rules.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tether_engine::config::{self, ConfigOrigin, ConfigValue};
use tether_quality::{Severity, Thresholds};
use tether_store::{Store, project_dir};
use tether_types::ProjectId;

use crate::config::ConfigIssueView;

/// One effective quality threshold and its source.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ThresholdView {
    /// Threshold key.
    pub key: String,
    /// Effective value as text.
    pub value: String,
    /// `built_in`, `global`, or `project`.
    pub origin: String,
}

/// Effective quality settings for one project.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct QualityConfigView {
    /// Effective numeric thresholds.
    pub thresholds: Vec<ThresholdView>,
    /// Disabled rule ids.
    pub disabled_rules: Vec<String>,
    /// Rule severity overrides as `rule_id=severity` strings.
    pub severity_overrides: Vec<String>,
    /// Recoverable configuration issues.
    pub issues: Vec<ConfigIssueView>,
}

/// One built-in rule shown by the settings screen.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct QualityRuleView {
    /// Stable rule id.
    pub id: String,
    /// Default severity.
    pub default_severity: String,
    /// Rule family.
    pub family: String,
}

/// Quality settings runtime.
#[derive(uniffi::Object)]
pub struct QualityRuntime {
    database_path: String,
    workspaces_root: String,
}

/// A quality runtime operation failed.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum QualityRuntimeError {
    /// The requested project does not exist.
    #[error("project not found")]
    NotFound,
    /// Storage or configuration paths are unavailable.
    #[error("quality settings unavailable")]
    Unavailable,
}

#[uniffi::export]
impl QualityRuntime {
    /// Opens the quality settings runtime.
    ///
    /// # Errors
    /// Returns `Unavailable` when the database cannot be opened.
    #[uniffi::constructor]
    pub fn new(
        database_path: &str,
        workspaces_root: &str,
    ) -> Result<Arc<Self>, QualityRuntimeError> {
        Store::open(Path::new(database_path)).map_err(|_| QualityRuntimeError::Unavailable)?;
        Ok(Arc::new(Self {
            database_path: database_path.to_owned(),
            workspaces_root: workspaces_root.to_owned(),
        }))
    }

    /// Reads the effective quality settings for a project.
    ///
    /// # Errors
    /// Returns `NotFound` for an unknown project.
    pub fn effective_quality(
        &self,
        project_id: &str,
    ) -> Result<QualityConfigView, QualityRuntimeError> {
        let store = Store::open(Path::new(&self.database_path))
            .map_err(|_| QualityRuntimeError::Unavailable)?;
        let project = store
            .load_project(&ProjectId::new(project_id))
            .map_err(|_| QualityRuntimeError::Unavailable)?
            .ok_or(QualityRuntimeError::NotFound)?;
        let root = project_dir(Path::new(&self.workspaces_root), &project);
        let (project_config, mut issues) = config::load_project(&root, true);
        let (global_config, global_issues) =
            config::load_global(&global_config_path(Path::new(&self.database_path)));
        issues.extend(global_issues);
        let effective = config::effective_config(
            &global_config,
            &project_config,
            &config::SessionConfig::default(),
        );
        let (thresholds, quality_issues) = Thresholds::merge(
            &config_table(&global_config.quality),
            &config_table(&project_config.quality),
        );
        let mut views = quality_issues
            .into_iter()
            .map(|issue| ConfigIssueView {
                key: issue.key,
                message: issue.message,
                line: None,
                column: None,
            })
            .collect::<Vec<_>>();
        views.extend(issues.into_iter().map(Into::into));
        Ok(QualityConfigView {
            thresholds: threshold_views(&thresholds, &effective.quality_origins),
            disabled_rules: thresholds.disabled_rules.into_iter().collect(),
            severity_overrides: thresholds
                .severity_overrides
                .into_iter()
                .map(|(id, severity)| format!("{id}={}", severity_name(severity)))
                .collect(),
            issues: views,
        })
    }

    /// Lists every built-in quality rule.
    #[must_use]
    pub fn list_quality_rules(&self) -> Vec<QualityRuleView> {
        built_in_rules()
            .into_iter()
            .map(|(id, severity, family)| QualityRuleView {
                id: id.to_owned(),
                default_severity: severity.to_owned(),
                family: family.to_owned(),
            })
            .collect()
    }
}

fn threshold_views(
    thresholds: &Thresholds,
    origins: &BTreeMap<String, ConfigOrigin>,
) -> Vec<ThresholdView> {
    [
        ("function_lines", thresholds.function_lines.to_string()),
        (
            "kotlin_composable_lines",
            thresholds.kotlin_composable_lines.to_string(),
        ),
        ("nesting", thresholds.nesting.to_string()),
        ("parameters", thresholds.parameters.to_string()),
        ("file_lines", thresholds.file_lines.to_string()),
        ("test_file_lines", thresholds.test_file_lines.to_string()),
        ("duplicate_lines", thresholds.duplicate_lines.to_string()),
        ("struct_fields", thresholds.struct_fields.to_string()),
        ("struct_functions", thresholds.struct_functions.to_string()),
        ("max_findings", thresholds.max_findings.to_string()),
    ]
    .into_iter()
    .map(|(key, value)| ThresholdView {
        origin: origins
            .get(key)
            .copied()
            .map_or_else(|| "built_in".to_owned(), origin_name),
        key: key.to_owned(),
        value,
    })
    .collect()
}

fn origin_name(origin: ConfigOrigin) -> String {
    match origin {
        ConfigOrigin::BuiltIn => "built_in",
        ConfigOrigin::Global => "global",
        ConfigOrigin::Project => "project",
        ConfigOrigin::Session => "session",
    }
    .to_owned()
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warn => "warn",
        Severity::Block => "block",
    }
}

fn config_table(values: &BTreeMap<String, ConfigValue>) -> toml::Table {
    values
        .iter()
        .map(|(key, value)| (key.clone(), toml_value(value)))
        .collect()
}

fn toml_value(value: &ConfigValue) -> toml::Value {
    match value {
        ConfigValue::Number(number) => integer_or_float(*number),
        ConfigValue::String(text) => toml::Value::String(text.clone()),
        ConfigValue::Array(values) => toml::Value::Array(values.iter().map(toml_value).collect()),
        ConfigValue::Table(values) => toml::Value::Table(
            values
                .iter()
                .map(|(key, value)| (key.clone(), toml_value(value)))
                .collect(),
        ),
    }
}

fn integer_or_float(number: f64) -> toml::Value {
    if number.fract() == 0.0
        && let Ok(integer) = number.to_string().parse::<i64>()
    {
        return toml::Value::Integer(integer);
    }
    toml::Value::Float(number)
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

fn built_in_rules() -> Vec<(&'static str, &'static str, &'static str)> {
    [
        "smell.function_length",
        "smell.nesting_depth",
        "smell.parameter_count",
        "smell.file_length",
        "smell.large_type",
        "smell.duplicate_block",
        "smell.dead_code",
        "smell.commented_out_code",
        "smell.magic_number",
        "smell.swallowed_error",
        "smell.debug_print",
        "smell.todo_without_issue",
        "slop.restating_comment",
        "slop.placeholder_body",
        "slop.unknown_import",
        "slop.broad_catch",
        "slop.unused_added_function",
        "slop.generic_name",
        "slop.banned_word_identifier",
        "slop.emoji_in_log",
        "slop.test_constant_assertion",
        "slop.test_without_assertion",
        "slop.test_removed_or_skipped",
        "slop.test_mocks_unit_under_test",
        "slop.test_snapshot_without_change",
    ]
    .into_iter()
    .map(|id| {
        (
            id,
            if id == "slop.test_removed_or_skipped" {
                "block"
            } else {
                "warn"
            },
            id.split('.').next().unwrap_or("slop"),
        )
    })
    .collect()
}
