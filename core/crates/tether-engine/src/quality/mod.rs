//! Quality gate execution for one native file-edit batch.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use tether_quality::{
    Batch, FileChange, Finding, ProjectManifest, QualityReport, Registry, Severity,
};
use tether_workspace::PathGuard;

use crate::config;

/// A file's content before the first successful edit in an exchange.
#[derive(Debug, Clone)]
pub(crate) struct PendingChange {
    /// Workspace-relative file path.
    pub(crate) path: PathBuf,
    /// Prior UTF-8 text, or `None` when the file did not exist.
    pub(crate) old_text: Option<String>,
}

/// The quality gate's result and the number of skipped changed files.
#[derive(Debug)]
pub(crate) struct GateReport {
    /// Findings and severity counts.
    pub(crate) report: QualityReport,
    /// Changed files skipped because they were not suitable for parsing.
    pub(crate) skipped: u32,
}

/// A recoverable quality gate failure.
#[derive(Debug, thiserror::Error)]
pub(crate) enum GateError {
    /// A changed file could not be read through the workspace guard.
    #[error("quality file read failed for {path}")]
    FileRead { path: String },
    /// The quality worker could not be joined.
    #[error("quality worker stopped")]
    Worker,
}

/// Owns the built-in quality rule registry.
pub(crate) struct QualityGate {
    registry: Registry,
    guard: PathGuard,
    global_config_path: PathBuf,
}

impl QualityGate {
    /// Creates a gate for one guarded project workspace.
    pub(crate) fn new(guard: PathGuard) -> Self {
        Self {
            global_config_path: default_global_config_path(guard.root()),
            registry: Registry::default(),
            guard,
        }
    }

    /// Runs the gate in a blocking worker.
    pub(crate) async fn run(
        self: std::sync::Arc<Self>,
        changes: Vec<PendingChange>,
    ) -> Result<GateReport, GateError> {
        tokio::task::spawn_blocking(move || self.run_blocking(changes))
            .await
            .map_err(|_| GateError::Worker)?
    }

    fn run_blocking(&self, changes: Vec<PendingChange>) -> Result<GateReport, GateError> {
        let (files, skipped) = self.read_changes(changes)?;
        let manifest = self.read_manifest();
        let thresholds = self.read_thresholds();
        let report =
            QualityReport::new(self.registry.run(&Batch { files }, &thresholds, &manifest));
        Ok(GateReport { report, skipped })
    }

    fn read_changes(
        &self,
        changes: Vec<PendingChange>,
    ) -> Result<(Vec<FileChange>, u32), GateError> {
        let mut files = Vec::new();
        let mut skipped = 0_u32;
        for change in changes {
            match self.read_change(change)? {
                Some(file) => files.push(file),
                None => skipped = skipped.saturating_add(1),
            }
        }
        Ok((files, skipped))
    }

    /// Reads one changed file. Returns `None` when the file is skipped.
    fn read_change(&self, change: PendingChange) -> Result<Option<FileChange>, GateError> {
        let path = self
            .guard
            .resolve(&change.path)
            .map_err(|_| file_read_error(&change.path))?;
        let Some(bytes) = read_changed_bytes(&path).map_err(|_| file_read_error(&change.path))?
        else {
            return Ok(Some(FileChange {
                path: change.path,
                old_text: change.old_text,
                new_text: None,
            }));
        };
        if should_skip(&bytes) {
            return Ok(None);
        }
        let Ok(new_text) = String::from_utf8(bytes) else {
            return Ok(None);
        };
        Ok(Some(FileChange {
            path: change.path,
            old_text: change.old_text,
            new_text: Some(new_text),
        }))
    }

    fn read_manifest(&self) -> ProjectManifest {
        let read = |relative: &str| {
            let path = self.guard.resolve(relative).ok()?;
            let bytes = std::fs::read(path).ok()?;
            (bytes.len() <= 256 * 1024)
                .then(|| String::from_utf8(bytes).ok())
                .flatten()
        };
        tether_quality::manifest_from_files(&read)
    }

    fn read_thresholds(&self) -> tether_quality::Thresholds {
        let (global, _) = config::load_global(&self.global_config_path);
        let (project, _) = config::load_project(self.guard.root(), false);
        let (thresholds, _) = tether_quality::Thresholds::merge(
            &config_table(&global.quality),
            &config_table(&project.quality),
        );
        thresholds
    }
}

fn read_changed_bytes(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => std::fs::read(path).map(Some),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn should_skip(bytes: &[u8]) -> bool {
    bytes.len() > 400 * 1024 || bytes.contains(&0)
}

fn config_table(values: &std::collections::BTreeMap<String, config::ConfigValue>) -> toml::Table {
    values
        .iter()
        .map(|(key, value)| (key.to_owned(), toml_value(value)))
        .collect()
}

fn toml_value(value: &config::ConfigValue) -> toml::Value {
    match value {
        config::ConfigValue::Number(number) => integer_or_float(*number),
        config::ConfigValue::String(text) => toml::Value::String(text.clone()),
        config::ConfigValue::Array(values) => {
            toml::Value::Array(values.iter().map(toml_value).collect())
        }
        config::ConfigValue::Table(values) => toml::Value::Table(
            values
                .iter()
                .map(|(key, value)| (key.clone(), toml_value(value)))
                .collect(),
        ),
    }
}

fn file_read_error(path: &Path) -> GateError {
    GateError::FileRead {
        path: display_path(path),
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

fn default_global_config_path(root: &Path) -> PathBuf {
    root.parent().and_then(Path::parent).map_or_else(
        || PathBuf::from("config/global.toml"),
        |path| path.join("config/global.toml"),
    )
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(crate) fn pending_change(path: PathBuf, old_text: Option<String>) -> PendingChange {
    PendingChange { path, old_text }
}

pub(crate) fn old_text(guard: &PathGuard, path: &Path) -> Option<String> {
    let resolved = guard.resolve(path).ok()?;
    let bytes = read_changed_bytes(&resolved).ok()??;
    if should_skip(&bytes) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

pub(crate) fn findings_to_records(findings: &[Finding]) -> Vec<tether_types::QualityFindingRecord> {
    findings
        .iter()
        .take(50)
        .map(|finding| tether_types::QualityFindingRecord {
            rule_id: finding.rule_id.clone(),
            severity: match finding.severity {
                Severity::Info => tether_types::QualitySeverity::Info,
                Severity::Warn => tether_types::QualitySeverity::Warn,
                Severity::Block => tether_types::QualitySeverity::Block,
            },
            path: display_path(&finding.path),
            line: finding.line,
            message: finding.message.clone(),
            advice: finding.advice.clone(),
        })
        .collect()
}
