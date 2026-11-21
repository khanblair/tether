//! Rule registry and batch execution.

use std::collections::HashSet;

use crate::language::ParsedFile;
use crate::rules::{self, util::is_error_region};
use crate::{Batch, Finding, ProjectManifest, Rule, RuleContext, Thresholds, changed_lines};

/// The built-in rule registry.
pub struct Registry {
    rules: Vec<Box<dyn Rule>>,
}

impl Default for Registry {
    fn default() -> Self {
        let mut rules = rules::smell_rules();
        rules.extend(rules::slop_rules());
        rules.extend(rules::test_rules());
        Self { rules }
    }
}

impl Registry {
    /// Builds a registry from custom rules.
    #[must_use]
    pub fn new(rules: Vec<Box<dyn Rule>>) -> Self {
        Self { rules }
    }

    /// Runs every enabled rule over a batch.
    #[must_use]
    pub fn run(
        &self,
        batch: &Batch,
        thresholds: &Thresholds,
        manifest: &ProjectManifest,
    ) -> Vec<Finding> {
        let parsed: Vec<Option<ParsedFile>> = batch
            .files
            .iter()
            .map(|file| {
                file.new_text.as_deref().and_then(|source| {
                    crate::Language::from_path(&file.path)
                        .and_then(|language| crate::parse(language, source).ok())
                })
            })
            .collect();
        let changes: Vec<_> = batch
            .files
            .iter()
            .map(|file| changed_lines(file.old_text.as_deref(), file.new_text.as_deref()))
            .collect();
        let mut findings = Vec::new();
        for rule in &self.rules {
            if thresholds.disabled_rules.contains(rule.id()) {
                continue;
            }
            for (index, file) in batch.files.iter().enumerate() {
                let context = RuleContext {
                    parsed: parsed[index].as_ref(),
                    path: &file.path,
                    old_text: file.old_text.as_deref(),
                    changed_lines: &changes[index],
                    batch,
                    thresholds,
                    manifest,
                };
                let whole_file = rule.id() == "smell.file_length";
                for mut finding in rule.check(&context) {
                    let deleted_test_rule =
                        file.new_text.is_none() && rule.id() == "slop.test_removed_or_skipped";
                    if !whole_file && !deleted_test_rule && !intersects(&finding, &changes[index]) {
                        continue;
                    }
                    if !whole_file
                        && parsed[index]
                            .as_ref()
                            .is_some_and(|file| error_at(file, finding.line))
                    {
                        continue;
                    }
                    if let Some(severity) = thresholds.severity_overrides.get(rule.id()) {
                        finding.severity = *severity;
                    }
                    findings.push(finding);
                }
            }
        }
        findings.sort_by(|left, right| {
            (&left.path, left.line, left.end_line, &left.rule_id).cmp(&(
                &right.path,
                right.line,
                right.end_line,
                &right.rule_id,
            ))
        });
        deduplicate(findings, thresholds.max_findings)
    }
}

fn intersects(finding: &Finding, ranges: &[std::ops::RangeInclusive<u32>]) -> bool {
    ranges
        .iter()
        .any(|range| *range.start() <= finding.end_line && *range.end() >= finding.line)
}

fn error_at(file: &ParsedFile, line: u32) -> bool {
    let mut nodes = Vec::new();
    crate::rules::util::walk(file.tree.root_node(), &mut nodes);
    nodes.into_iter().any(|node| {
        is_error_region(node)
            && (node.start_position().row as u32) < line
            && node.end_position().row as u32 + 1 >= line
    })
}

fn deduplicate(findings: Vec<Finding>, cap: usize) -> Vec<Finding> {
    let mut seen = HashSet::new();
    findings
        .into_iter()
        .filter(|finding| {
            seen.insert((
                finding.rule_id.clone(),
                finding.path.clone(),
                finding.line,
                finding.end_line,
                finding.message.clone(),
            ))
        })
        .take(cap)
        .collect()
}
