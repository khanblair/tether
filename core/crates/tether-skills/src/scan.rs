//! Static checks for skill files before installation.

use std::path::{Component, Path};

use crate::scan_rules::{
    bypasses_controls, contains_url, has_hidden_character, is_image, is_script,
    overrides_instructions, reads_sensitive_path, sends_secret,
};

const MAX_FILES: usize = 100;
const MAX_TOTAL_BYTES: usize = 1 << 20;
const MAX_IMAGE_BYTES: usize = 200 << 10;
const MAX_ADVISORY_BYTES: usize = 100 << 10;

/// One file supplied to the static scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillFile {
    /// Path relative to the candidate skill folder.
    pub path: String,
    /// File bytes.
    pub bytes: Vec<u8>,
}

/// Finding severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScanSeverity {
    /// A count or size detail.
    Info,
    /// The skill needs a user review.
    Warn,
    /// The skill cannot be installed.
    Block,
}

/// Overall static scan result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanVerdict {
    /// No warnings or blocks were found.
    Clean,
    /// The skill has review findings but no blocks.
    Warnings,
    /// The skill must not be installed.
    Blocked,
}

/// One static scan finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanFinding {
    /// Stable rule identifier.
    pub rule_id: String,
    /// Finding severity.
    pub severity: ScanSeverity,
    /// File path, or an empty path for whole-skill findings.
    pub path: String,
    /// One-based source line, or zero for whole-skill findings.
    pub line: u32,
    /// Short safe explanation.
    pub message: String,
}

/// Result of scanning a candidate skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReport {
    /// Findings in deterministic input order.
    pub findings: Vec<ScanFinding>,
    /// Highest severity found by the scan.
    pub verdict: ScanVerdict,
}

/// Scans skill content without executing it.
#[must_use]
pub fn scan_skill(files: &[SkillFile]) -> ScanReport {
    let mut findings = vec![info("I001", "file count", files.len())];
    let total = files.iter().map(|file| file.bytes.len()).sum::<usize>();
    findings.push(info("I002", "total size", total));
    if files.len() > MAX_FILES || total > MAX_TOTAL_BYTES {
        findings.push(block(
            "B010",
            "",
            0,
            "skill exceeds the file or size limit.",
        ));
    }
    let has_skill = files.iter().any(|file| file.path.ends_with("SKILL.md"));
    if !has_skill {
        findings.push(block("B011", "", 0, "SKILL.md is missing."));
    }
    for file in files {
        scan_path(file, &mut findings);
        scan_bytes(file, &mut findings);
    }
    report(findings)
}

fn scan_path(file: &SkillFile, findings: &mut Vec<ScanFinding>) {
    let path = Path::new(&file.path);
    let unsafe_path = path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || file.path.contains('\\')
        || file.path.contains('\0');
    if unsafe_path {
        findings.push(block(
            "B008",
            &file.path,
            0,
            "file path escapes the skill folder.",
        ));
    }
    if file.path.ends_with("SKILL.md") {
        check_skill_name(file, findings);
        check_description(file, findings);
    }
}

fn scan_bytes(file: &SkillFile, findings: &mut Vec<ScanFinding>) {
    if file.bytes.len() > MAX_ADVISORY_BYTES {
        findings.push(warn("W005", &file.path, 1, "file is larger than 100 KB."));
    }
    if file.bytes.contains(&0) && !(is_image(&file.path) && file.bytes.len() < MAX_IMAGE_BYTES) {
        findings.push(block(
            "B009",
            &file.path,
            1,
            "binary file is not an allowed small image.",
        ));
        return;
    }
    let text = String::from_utf8_lossy(&file.bytes);
    let script = is_script(&file.path);
    if script {
        findings.push(warn(
            "W001",
            &file.path,
            1,
            "script files run in executors only.",
        ));
    }
    for (line, source) in text.lines().enumerate() {
        let line_number = u32::try_from(line + 1).unwrap_or(u32::MAX);
        scan_line(file, script, line_number, source, findings);
    }
}

fn scan_line(
    file: &SkillFile,
    script: bool,
    line_number: u32,
    source: &str,
    findings: &mut Vec<ScanFinding>,
) {
    let lower = source.to_ascii_lowercase();
    check_block_rules(&file.path, line_number, &lower, findings);
    if script && contains_url(&lower) {
        findings.push(warn(
            "W002",
            &file.path,
            line_number,
            "script contains a network URL.",
        ));
    }
    if lower.contains("sudo") || lower.contains("chmod 777") || lower.contains("rm -rf") {
        findings.push(warn(
            "W003",
            &file.path,
            line_number,
            "instruction uses a dangerous system command.",
        ));
    }
    if has_hidden_character(source) {
        findings.push(block(
            "B007",
            &file.path,
            line_number,
            "text contains hidden Unicode characters.",
        ));
    }
}

fn check_block_rules(path: &str, line: u32, lower: &str, findings: &mut Vec<ScanFinding>) {
    if sends_secret(lower) {
        findings.push(block(
            "B001",
            path,
            line,
            "instruction sends secrets to a destination.",
        ));
    }
    if overrides_instructions(lower) {
        findings.push(block(
            "B002",
            path,
            line,
            "instruction overrides earlier instructions.",
        ));
    }
    if bypasses_controls(lower) {
        findings.push(block(
            "B003",
            path,
            line,
            "instruction bypasses approvals or the sandbox.",
        ));
    }
    check_shell_rules(path, line, lower, findings);
    if reads_sensitive_path(lower) {
        findings.push(block(
            "B006",
            path,
            line,
            "instruction reads a sensitive local path.",
        ));
    }
}

fn check_shell_rules(path: &str, line: u32, lower: &str, findings: &mut Vec<ScanFinding>) {
    if (lower.contains("curl") || lower.contains("wget"))
        && (lower.contains("| sh")
            || lower.contains("|sh")
            || lower.contains("| bash")
            || lower.contains("|bash"))
    {
        findings.push(block(
            "B004",
            path,
            line,
            "command downloads and executes a shell script.",
        ));
    }
    if (lower.contains("base64 -d") && (lower.contains("| sh") || lower.contains("| bash")))
        || lower.contains("eval $(")
        || lower.contains("exec(base64")
    {
        findings.push(block(
            "B005",
            path,
            line,
            "command decodes or evaluates executable content.",
        ));
    }
}

fn check_skill_name(file: &SkillFile, findings: &mut Vec<ScanFinding>) {
    let mut parts = file.path.split('/');
    let Some(folder) = parts.next() else {
        return;
    };
    let Some(name) = frontmatter_value(&file.bytes, "name") else {
        return;
    };
    if name != folder {
        let line = value_line(&file.bytes, "name");
        findings.push(block(
            "B012",
            &file.path,
            line,
            "skill name does not match its folder.",
        ));
    }
}

fn check_description(file: &SkillFile, findings: &mut Vec<ScanFinding>) {
    let Some(description) = frontmatter_value(&file.bytes, "description") else {
        return;
    };
    let vague = description.chars().count() < 20
        || matches!(
            description.to_ascii_lowercase().as_str(),
            "a skill" | "helper" | "does things"
        );
    if vague || description.chars().count() > 1_024 {
        let line = value_line(&file.bytes, "description");
        findings.push(warn(
            "W004",
            &file.path,
            line,
            "description is vague or over 1,024 characters.",
        ));
    }
}

fn frontmatter_value(bytes: &[u8], key: &str) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .take_while(|line| line.trim() != "---")
        .find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.trim() == key)
                .map(|(_, value)| value.trim().trim_matches(['"', '\'']).to_owned())
        })
}

fn value_line(bytes: &[u8], key: &str) -> u32 {
    String::from_utf8_lossy(bytes)
        .lines()
        .enumerate()
        .find_map(|(line, source)| source.starts_with(&format!("{key}: ")).then_some(line + 1))
        .and_then(|line| u32::try_from(line).ok())
        .unwrap_or(1)
}

fn report(findings: Vec<ScanFinding>) -> ScanReport {
    let verdict = if findings
        .iter()
        .any(|finding| finding.severity == ScanSeverity::Block)
    {
        ScanVerdict::Blocked
    } else if findings
        .iter()
        .any(|finding| finding.severity == ScanSeverity::Warn)
    {
        ScanVerdict::Warnings
    } else {
        ScanVerdict::Clean
    };
    ScanReport { findings, verdict }
}

fn info(rule_id: &str, label: &str, value: usize) -> ScanFinding {
    ScanFinding {
        rule_id: rule_id.into(),
        severity: ScanSeverity::Info,
        path: String::new(),
        line: 0,
        message: format!("{rule_id} line 0: {label}: {value}."),
    }
}

fn warn(rule_id: &str, path: &str, line: u32, message: &str) -> ScanFinding {
    finding(rule_id, ScanSeverity::Warn, path, line, message)
}

fn block(rule_id: &str, path: &str, line: u32, message: &str) -> ScanFinding {
    finding(rule_id, ScanSeverity::Block, path, line, message)
}

fn finding(
    rule_id: &str,
    severity: ScanSeverity,
    path: &str,
    line: u32,
    message: &str,
) -> ScanFinding {
    ScanFinding {
        rule_id: rule_id.into(),
        severity,
        path: path.into(),
        line,
        message: format!("{rule_id} line {line}: {message}"),
    }
}
