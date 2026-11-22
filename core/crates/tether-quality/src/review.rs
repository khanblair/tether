//! Second-model review prompt and response parsing.

use std::collections::BTreeSet;
use std::fmt;

use serde_json::Value;

use crate::{QualityReport, Severity};

/// Input used to construct a second-model review prompt.
pub struct ReviewInput<'a> {
    /// Unified diff to review.
    pub diff: &'a str,
    /// User task that frames the review.
    pub task: &'a str,
    /// Quality gate report for the same change.
    pub report: &'a QualityReport,
    /// Maximum diff characters included in the prompt.
    pub max_diff_chars: usize,
}

/// The system and user messages sent to a reviewer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewPrompt {
    /// Reviewer behavior instructions.
    pub system: String,
    /// Review task and diff.
    pub user: String,
}

/// One accepted reviewer finding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewFinding {
    /// Repository-relative path.
    pub path: String,
    /// One-based line in the new file.
    pub line: u32,
    /// Finding severity.
    pub severity: Severity,
    /// Review message.
    pub message: String,
}

/// Parsed reviewer output and the number of rejected findings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedReview {
    /// Findings that refer to the supplied diff.
    pub findings: Vec<ReviewFinding>,
    /// Reviewer summary.
    pub summary: String,
    /// Findings dropped during validation.
    pub dropped: usize,
}

/// An invalid reviewer response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewParseError {
    /// Plain parser error.
    pub message: String,
}

impl fmt::Display for ReviewParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ReviewParseError {}

const MAX_FINDINGS: usize = 50;

/// Builds the reviewer messages.
#[must_use]
pub fn review_prompt(input: &ReviewInput<'_>) -> ReviewPrompt {
    let diff = truncate_diff(input.diff, input.max_diff_chars);
    let system = "Review the change as a careful code reviewer. Use short plain sentences. Report only real bugs, risks, and missing tests. Reply with one JSON object and nothing else. Use this shape: {\"findings\":[{\"path\":\"...\",\"line\":1,\"severity\":\"info|warn|block\",\"message\":\"...\"}],\"summary\":\"...\"}".to_owned();
    let user = format!(
        "Task:\n{}\n\nQuality report:\n{}\n\nDiff:\n{}",
        input.task,
        input.report.render_summary(),
        diff
    );
    ReviewPrompt { system, user }
}

/// Parses a reviewer response and validates finding locations against a diff.
///
/// A finding whose path and line are not in the diff is dropped and counted. At most 50
/// findings are kept.
///
/// # Errors
/// Returns an error when the reply holds no JSON object or the object is malformed.
pub fn parse_review(reply: &str, diff: &str) -> Result<ParsedReview, ReviewParseError> {
    let json = extract_object(reply).ok_or_else(|| parse_error("no JSON object found"))?;
    let value: Value =
        serde_json::from_str(json).map_err(|_| parse_error("review response is not valid JSON"))?;
    let Some(object) = value.as_object() else {
        return Err(parse_error("review response is not an object"));
    };
    let summary = object
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let allowed = diff_locations(diff);
    let items = object
        .get("findings")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let mut dropped = 0;
    let mut findings = Vec::new();
    for item in items {
        match parse_finding(item, &allowed) {
            Some(finding) => findings.push(finding),
            None => dropped += 1,
        }
        if findings.len() == MAX_FINDINGS {
            break;
        }
    }
    Ok(ParsedReview {
        findings,
        summary,
        dropped,
    })
}

/// Reads one finding, or gives `None` when it is malformed or points outside the diff.
fn parse_finding(item: &Value, allowed: &BTreeSet<(String, u32)>) -> Option<ReviewFinding> {
    let item = item.as_object()?;
    let path = item.get("path")?.as_str()?;
    let line = u32::try_from(item.get("line")?.as_i64()?).ok()?;
    if line == 0 || !allowed.contains(&(normalize_path(path), line)) {
        return None;
    }
    let severity = item
        .get("severity")
        .and_then(Value::as_str)
        .and_then(parse_severity)
        .unwrap_or(Severity::Info);
    let message = item
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Some(ReviewFinding {
        path: path.to_owned(),
        line,
        severity,
        message: truncate_chars(message, 500),
    })
}

fn truncate_diff(diff: &str, max: usize) -> String {
    if diff.chars().count() <= max {
        return diff.to_owned();
    }
    let marker = "\n[diff truncated at hunk boundary]";
    let byte_limit = diff
        .char_indices()
        .take_while(|(index, _)| *index <= max)
        .last()
        .map_or(0, |(index, character)| index + character.len_utf8());
    let prefix = &diff[..byte_limit.min(diff.len())];
    let cut = prefix
        .rfind("@@")
        .and_then(|start| prefix[..start].rfind('\n'));
    let prefix = cut.map_or("", |index| &diff[..=index]);
    format!("{prefix}{marker}")
}

fn extract_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0_u32;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate().skip(start) {
        if quoted {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return text.get(start..=index);
                }
            }
            _ => {}
        }
    }
    None
}

fn diff_locations(diff: &str) -> BTreeSet<(String, u32)> {
    let mut locations = BTreeSet::new();
    let mut path = String::new();
    let mut next_line = None;
    for line in diff.lines() {
        if let Some(value) = line.strip_prefix("+++ ") {
            path = normalize_path(value.split('\t').next().unwrap_or(value));
        } else if line.starts_with("@@") {
            next_line = parse_hunk_start(line);
        } else if let Some(line_number) = next_line.as_mut()
            && ((line.starts_with('+') && !line.starts_with("+++")) || !line.starts_with('-'))
        {
            locations.insert((path.clone(), *line_number));
            *line_number += 1;
        }
    }
    locations
}

fn parse_hunk_start(line: &str) -> Option<u32> {
    let plus = line.split_whitespace().find(|part| part.starts_with('+'))?;
    let value = plus.trim_start_matches('+').split(',').next()?;
    value.parse().ok()
}

fn normalize_path(path: &str) -> String {
    path.trim_start_matches("b/")
        .trim_start_matches("a/")
        .to_owned()
}

fn parse_severity(value: &str) -> Option<Severity> {
    match value {
        "info" => Some(Severity::Info),
        "warn" => Some(Severity::Warn),
        "block" => Some(Severity::Block),
        _ => None,
    }
}

fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn parse_error(message: &str) -> ReviewParseError {
    ReviewParseError {
        message: message.to_owned(),
    }
}
