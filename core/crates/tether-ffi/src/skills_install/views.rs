//! Platform-facing records and enums for skill installation and upkeep.

use tether_skills::{ScanFinding, ScanReport, ScanSeverity, ScanVerdict, SkillState, UpkeepAction};

/// Scan severity exposed to the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ScanSeverityView {
    /// Informational result.
    Info,
    /// Review result.
    Warn,
    /// Blocking result.
    Block,
}

/// Scan verdict exposed to the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ScanVerdictView {
    /// No findings.
    Clean,
    /// Warnings only.
    Warnings,
    /// Installation is blocked.
    Blocked,
}

/// One scan finding exposed to the platform.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ScanFindingView {
    /// Stable rule id.
    pub rule_id: String,
    /// Finding severity.
    pub severity: ScanSeverityView,
    /// Relative file path.
    pub path: String,
    /// One-based source line.
    pub line: u32,
    /// Safe explanation.
    pub message: String,
}

/// A static scan exposed to the platform.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ScanReportView {
    /// Findings from the scan.
    pub findings: Vec<ScanFindingView>,
    /// Overall verdict.
    pub verdict: ScanVerdictView,
}

/// One quarantined file exposed to the platform.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct QuarantineFileView {
    /// Path relative to the skill folder.
    pub path: String,
    /// File size in bytes.
    pub bytes: u64,
    /// Whether the file is an executor script.
    pub is_script: bool,
}

/// A skill waiting for install approval.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct QuarantineView {
    /// Quarantine id.
    pub id: String,
    /// Skill name.
    pub name: String,
    /// Skill description.
    pub description: String,
    /// Files shown before approval.
    pub files: Vec<QuarantineFileView>,
    /// Static scan result.
    pub scan: ScanReportView,
    /// Whether approval can install the skill.
    pub can_install: bool,
    /// Human-readable source label.
    pub source_label: String,
}

/// Destination for an approved skill.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SkillScopeView {
    /// Private app skill storage.
    User,
    /// Trusted project skill storage.
    Project {
        /// Project id.
        project_id: String,
    },
}

/// One installed skill returned after approval.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct InstalledSkillView {
    /// Installed skill name.
    pub name: String,
    /// Installation scope.
    pub scope: SkillScopeView,
}

/// Skill state exposed to the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SkillStateView {
    /// Available skill.
    Active,
    /// Unused skill.
    Stale,
    /// Archived skill.
    Archived,
}

/// One upkeep state transition.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UpkeepActionView {
    /// Skill name.
    pub name: String,
    /// State before upkeep.
    pub from: SkillStateView,
    /// State after upkeep.
    pub to: SkillStateView,
}

impl From<ScanFinding> for ScanFindingView {
    fn from(finding: ScanFinding) -> Self {
        Self {
            rule_id: finding.rule_id,
            severity: finding.severity.into(),
            path: finding.path,
            line: finding.line,
            message: finding.message,
        }
    }
}
impl From<ScanSeverity> for ScanSeverityView {
    fn from(value: ScanSeverity) -> Self {
        match value {
            ScanSeverity::Info => Self::Info,
            ScanSeverity::Warn => Self::Warn,
            ScanSeverity::Block => Self::Block,
        }
    }
}
impl From<ScanVerdict> for ScanVerdictView {
    fn from(value: ScanVerdict) -> Self {
        match value {
            ScanVerdict::Clean => Self::Clean,
            ScanVerdict::Warnings => Self::Warnings,
            ScanVerdict::Blocked => Self::Blocked,
        }
    }
}
impl From<ScanReport> for ScanReportView {
    fn from(report: ScanReport) -> Self {
        Self {
            findings: report.findings.into_iter().map(Into::into).collect(),
            verdict: report.verdict.into(),
        }
    }
}
impl From<SkillState> for SkillStateView {
    fn from(value: SkillState) -> Self {
        match value {
            SkillState::Active => Self::Active,
            SkillState::Stale => Self::Stale,
            SkillState::Archived => Self::Archived,
        }
    }
}
impl From<UpkeepAction> for UpkeepActionView {
    fn from(action: UpkeepAction) -> Self {
        Self {
            name: action.name,
            from: action.from.into(),
            to: action.to.into(),
        }
    }
}
