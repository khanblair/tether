//! Public data types shared by the quality rules.

use std::collections::BTreeSet;
use std::ops::RangeInclusive;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

use crate::language::ParsedFile;

/// The three quality gate severities.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Severity {
    /// A low-priority observation.
    Info,
    /// A finding that should normally be fixed before review.
    #[default]
    Warn,
    /// A finding that blocks an automatic commit.
    Block,
}

/// One quality gate finding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    /// Stable rule identifier.
    pub rule_id: String,
    /// Effective finding severity.
    pub severity: Severity,
    /// Repository-relative path.
    pub path: PathBuf,
    /// First affected line, one based.
    pub line: u32,
    /// Last affected line, one based.
    pub end_line: u32,
    /// Short plain-language explanation.
    pub message: String,
    /// Short suggested action.
    pub advice: String,
}

/// The old and new contents of one changed file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileChange {
    /// Repository-relative path.
    pub path: PathBuf,
    /// Previous text, or `None` for a created file.
    pub old_text: Option<String>,
    /// New text, or `None` for a deleted file.
    pub new_text: Option<String>,
}

/// A group of file changes from one edit batch.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Batch {
    /// Files changed by this batch.
    pub files: Vec<FileChange>,
}

/// Packages and language hints collected from project manifests.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectManifest {
    /// Declared package names, normalized where the manifest has a canonical spelling.
    pub declared_packages: BTreeSet<String>,
    /// Optional language hints from the manifests.
    pub language_hints: BTreeSet<String>,
}

/// Context supplied to one rule for one changed file.
pub struct RuleContext<'a> {
    /// The parsed new file, or `None` for a deleted file.
    pub parsed: Option<&'a ParsedFile>,
    /// The repository-relative file path.
    pub path: &'a std::path::Path,
    /// The previous file text, when available.
    pub old_text: Option<&'a str>,
    /// Lines changed in the new file.
    pub changed_lines: &'a [RangeInclusive<u32>],
    /// The full edit batch.
    pub batch: &'a Batch,
    /// Active thresholds.
    pub thresholds: &'a crate::Thresholds,
    /// Project manifest data.
    pub manifest: &'a ProjectManifest,
}

/// A quality rule implementation.
pub trait Rule: Send + Sync {
    /// Returns the stable rule identifier.
    fn id(&self) -> &'static str;
    /// Returns the default severity before overrides.
    fn default_severity(&self) -> Severity;
    /// Checks one file and returns its candidate findings.
    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding>;
}

/// Finds one-based new-text line ranges that were inserted or modified.
#[must_use]
pub fn changed_lines(old: Option<&str>, new: Option<&str>) -> Vec<RangeInclusive<u32>> {
    let Some(new) = new else { return Vec::new() };
    let old = old.unwrap_or_default();
    let old_lines: Vec<_> = old.lines().collect();
    let new_lines: Vec<_> = new.lines().collect();
    let diff = TextDiff::from_slices(old_lines.as_slice(), new_lines.as_slice());
    let mut changed = Vec::new();
    let mut new_line = 1_u32;
    let mut pending_start = None;
    for change in diff.iter_all_changes() {
        let count = 1_u32;
        match change.tag() {
            ChangeTag::Equal => {
                push_range(
                    &mut changed,
                    pending_start.take(),
                    new_line.saturating_sub(1),
                );
                new_line = new_line.saturating_add(count);
            }
            ChangeTag::Insert => {
                pending_start.get_or_insert(new_line);
                new_line = new_line.saturating_add(count);
            }
            ChangeTag::Delete => {
                pending_start.get_or_insert(new_line);
            }
        }
    }
    push_range(&mut changed, pending_start, new_line.saturating_sub(1));
    merge_ranges(changed)
}

fn push_range(ranges: &mut Vec<RangeInclusive<u32>>, start: Option<u32>, end: u32) {
    if let Some(start) = start
        && start <= end
    {
        ranges.push(start..=end);
    }
}

fn merge_ranges(ranges: Vec<RangeInclusive<u32>>) -> Vec<RangeInclusive<u32>> {
    let mut merged: Vec<RangeInclusive<u32>> = Vec::new();
    for range in ranges {
        let start = *range.start();
        let end = *range.end();
        if let Some(previous) = merged.last_mut()
            && start <= previous.end().saturating_add(1)
        {
            let previous_start = *previous.start();
            *previous = previous_start..=(*previous.end()).max(end);
            continue;
        }
        merged.push(start..=end);
    }
    merged
}
