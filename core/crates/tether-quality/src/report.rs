//! Quality report construction and model rendering.

use crate::{Finding, Severity};

/// Counts of findings by severity.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    /// Number of informational findings.
    pub info: usize,
    /// Number of warning findings.
    pub warn: usize,
    /// Number of blocking findings.
    pub block: usize,
}

/// Findings and the blocking state for one edit batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualityReport {
    /// Findings in registry order.
    pub findings: Vec<Finding>,
    /// Counts by severity.
    pub counts: Counts,
    /// Whether a block finding is present.
    pub blocked: bool,
}

impl QualityReport {
    /// Builds a report and computes its counts.
    #[must_use]
    pub fn new(findings: Vec<Finding>) -> Self {
        let mut counts = Counts::default();
        for finding in &findings {
            match finding.severity {
                Severity::Info => counts.info += 1,
                Severity::Warn => counts.warn += 1,
                Severity::Block => counts.block += 1,
            }
        }
        Self {
            findings,
            counts,
            blocked: counts.block > 0,
        }
    }

    /// Renders findings for an agent to fix.
    #[must_use]
    pub fn render_for_model(&self, max_findings: usize) -> String {
        if self.findings.is_empty() {
            return String::new();
        }
        let mut findings = self.findings.iter().collect::<Vec<_>>();
        findings.sort_by_key(|finding| std::cmp::Reverse(finding.severity));
        let mut lines = Vec::<String>::new();
        lines.push(
            (if self.blocked {
                "Fix these before committing:"
            } else {
                "Quality notes:"
            })
            .to_owned(),
        );
        for finding in findings.iter().take(max_findings) {
            lines.push(format_finding(finding));
        }
        let remaining = findings.len().saturating_sub(max_findings);
        if remaining > 0 {
            lines.push(format!("and {remaining} more"));
        }
        lines.join("\n")
    }

    /// Renders a short severity summary.
    #[must_use]
    pub fn render_summary(&self) -> String {
        if self.findings.is_empty() {
            return "no findings".to_owned();
        }
        let mut parts = Vec::new();
        if self.counts.block > 0 {
            parts.push(format_count(self.counts.block, "block", "blocks"));
        }
        if self.counts.warn > 0 {
            parts.push(format_count(self.counts.warn, "warning", "warnings"));
        }
        if self.counts.info > 0 {
            parts.push(format_count(self.counts.info, "info", "infos"));
        }
        parts.join(", ")
    }
}

fn format_finding(finding: &Finding) -> String {
    format!(
        "{}:{} [{}] {} {}",
        finding.path.display(),
        finding.line,
        severity_name(finding.severity),
        finding.rule_id,
        finding.message
    )
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warn => "warn",
        Severity::Block => "block",
    }
}

fn format_count(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}
