//! Quality thresholds and configuration parsing.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use crate::{Severity, rules};

/// A recoverable quality configuration issue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigIssue {
    /// The setting that caused the issue.
    pub key: String,
    /// The reason the value was not used.
    pub reason: String,
    /// Alias for callers that use the engine configuration vocabulary.
    pub message: String,
}

/// Configurable thresholds and rule settings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Thresholds {
    /// Maximum function lines.
    pub function_lines: u32,
    /// Maximum Kotlin composable lines.
    pub kotlin_composable_lines: u32,
    /// Maximum nesting depth.
    pub nesting: u32,
    /// Maximum function parameters.
    pub parameters: u32,
    /// Maximum ordinary file lines.
    pub file_lines: u32,
    /// Maximum test-only file lines.
    pub test_file_lines: u32,
    /// Minimum duplicate line run.
    pub duplicate_lines: u32,
    /// Maximum fields in a type.
    pub struct_fields: u32,
    /// Maximum functions in a type.
    pub struct_functions: u32,
    /// Per-rule severity overrides.
    pub severity_overrides: BTreeMap<String, Severity>,
    /// Disabled rule identifiers.
    pub disabled_rules: BTreeSet<String>,
    /// Maximum findings returned by a registry run.
    pub max_findings: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            function_lines: 50,
            kotlin_composable_lines: 80,
            nesting: 3,
            parameters: 5,
            file_lines: 400,
            test_file_lines: 800,
            duplicate_lines: 10,
            struct_fields: 12,
            struct_functions: 20,
            severity_overrides: BTreeMap::new(),
            disabled_rules: BTreeSet::new(),
            max_findings: 200,
        }
    }
}

impl Thresholds {
    /// Parses a `[quality]` table, retaining defaults for invalid values.
    #[must_use]
    pub fn from_table(table: &Table) -> (Self, Vec<ConfigIssue>) {
        let mut result = Self::default();
        let mut issues = Vec::new();
        for (key, value) in table {
            match key.as_str() {
                "function_lines" => set_limit(&mut result.function_lines, key, value, &mut issues),
                "nesting" => set_limit(&mut result.nesting, key, value, &mut issues),
                "parameters" => set_limit(&mut result.parameters, key, value, &mut issues),
                "file_lines" => set_limit(&mut result.file_lines, key, value, &mut issues),
                "test_file_lines" => {
                    set_limit(&mut result.test_file_lines, key, value, &mut issues);
                }
                "duplicate_lines" => {
                    set_limit(&mut result.duplicate_lines, key, value, &mut issues);
                }
                "max_findings" => {
                    set_max_findings(&mut result.max_findings, key, value, &mut issues)
                }
                "disable" => parse_disabled(&mut result.disabled_rules, key, value, &mut issues),
                "severity" => {
                    parse_severity(&mut result.severity_overrides, key, value, &mut issues)
                }
                _ => issues.push(issue(key, "unknown quality setting")),
            }
        }
        (result, issues)
    }

    /// Merges a global quality table with a project table.
    #[must_use]
    pub fn merge(global: &Table, project: &Table) -> (Self, Vec<ConfigIssue>) {
        let mut merged = global.clone();
        for (key, value) in project {
            if key == "severity" {
                let mut severity = merged
                    .remove(key)
                    .and_then(|value| value.as_table().cloned())
                    .unwrap_or_default();
                if let Some(project_severity) = value.as_table() {
                    severity.extend(project_severity.clone());
                }
                merged.insert(key.clone(), Value::Table(severity));
            } else {
                merged.insert(key.clone(), value.clone());
            }
        }
        Self::from_table(&merged)
    }
}

fn set_limit(target: &mut u32, key: &str, value: &Value, issues: &mut Vec<ConfigIssue>) {
    match value.as_integer() {
        Some(value) if (1..=10_000).contains(&value) => *target = value as u32,
        Some(_) => issues.push(issue(key, "must be in the range 1..=10000")),
        None => issues.push(issue(key, "must be an integer")),
    }
}

fn set_max_findings(target: &mut usize, key: &str, value: &Value, issues: &mut Vec<ConfigIssue>) {
    match value.as_integer() {
        Some(value) if (1..=10_000).contains(&value) => *target = value as usize,
        Some(_) => issues.push(issue(key, "must be in the range 1..=10000")),
        None => issues.push(issue(key, "must be an integer")),
    }
}

fn parse_disabled(
    target: &mut BTreeSet<String>,
    key: &str,
    value: &Value,
    issues: &mut Vec<ConfigIssue>,
) {
    let Some(values) = value.as_array() else {
        issues.push(issue(key, "must be an array of rule ids"));
        return;
    };
    for value in values {
        let Some(rule_id) = value.as_str() else {
            issues.push(issue(key, "must contain only rule ids"));
            continue;
        };
        if !rules::is_known_rule_id(rule_id) {
            issues.push(issue(key, &format!("unknown rule id: {rule_id}")));
        } else {
            target.insert(rule_id.to_owned());
        }
    }
}

fn parse_severity(
    target: &mut BTreeMap<String, Severity>,
    key: &str,
    value: &Value,
    issues: &mut Vec<ConfigIssue>,
) {
    let Some(values) = value.as_table() else {
        issues.push(issue(key, "must be a table of rule severities"));
        return;
    };
    for (rule_id, value) in values {
        if !rules::is_known_rule_id(rule_id) {
            issues.push(issue(&format!("severity.{rule_id}"), "unknown rule id"));
            continue;
        }
        let Some(name) = value.as_str() else {
            issues.push(issue(
                &format!("severity.{rule_id}"),
                "must be info, warn, or block",
            ));
            continue;
        };
        let Some(severity) = parse_severity_name(name) else {
            issues.push(issue(
                &format!("severity.{rule_id}"),
                "must be info, warn, or block",
            ));
            continue;
        };
        target.insert(rule_id.clone(), severity);
    }
}

fn parse_severity_name(name: &str) -> Option<Severity> {
    match name {
        "info" => Some(Severity::Info),
        "warn" => Some(Severity::Warn),
        "block" => Some(Severity::Block),
        _ => None,
    }
}

fn issue(key: &str, reason: &str) -> ConfigIssue {
    ConfigIssue {
        key: key.to_owned(),
        reason: reason.to_owned(),
        message: reason.to_owned(),
    }
}
