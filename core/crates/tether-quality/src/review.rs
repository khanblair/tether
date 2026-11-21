//! Second-model review prompt and response parsing.

use std::fmt;

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
pub fn parse_review(reply: &str, diff: &str) -> Result<ParsedReview, ReviewParseError> {
    let json = extract_object(reply).ok_or_else(|| parse_error("no JSON object found"))?;
    let value = JsonParser::new(json).parse()?;
    let JsonValue::Object(object) = value else {
        return Err(parse_error("review response is not an object"));
    };
    let summary = object
        .get("summary")
        .and_then(|value| value.as_ref().as_string())
        .unwrap_or_default()
        .to_owned();
    let allowed = diff_locations(diff);
    let mut dropped = 0;
    let mut findings = Vec::new();
    if let Some(JsonValue::Array(values)) = object.get("findings").map(Box::as_ref) {
        for value in values {
            let JsonValue::Object(item) = value.as_ref() else {
                dropped += 1;
                continue;
            };
            let Some(path) = item
                .get("path")
                .and_then(|value| value.as_ref().as_string())
            else {
                dropped += 1;
                continue;
            };
            let Some(line) = item.get("line").and_then(|value| value.as_ref().as_i64()) else {
                dropped += 1;
                continue;
            };
            let Ok(line) = u32::try_from(line) else {
                dropped += 1;
                continue;
            };
            if line == 0 || !allowed.contains(&(normalize_path(path), line)) {
                dropped += 1;
                continue;
            }
            let severity = item
                .get("severity")
                .and_then(|value| value.as_ref().as_string())
                .and_then(parse_severity)
                .unwrap_or(Severity::Info);
            let message = item
                .get("message")
                .and_then(|value| value.as_ref().as_string())
                .unwrap_or_default();
            findings.push(ReviewFinding {
                path: path.to_owned(),
                line,
                severity,
                message: truncate_chars(message, 500),
            });
            if findings.len() == 50 {
                break;
            }
        }
    }
    Ok(ParsedReview {
        findings,
        summary,
        dropped,
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
    let prefix = cut.map_or("", |index| &diff[..index + 1]);
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

fn diff_locations(diff: &str) -> std::collections::BTreeSet<(String, u32)> {
    let mut locations = std::collections::BTreeSet::new();
    let mut path = String::new();
    let mut next_line = None;
    for line in diff.lines() {
        if let Some(value) = line.strip_prefix("+++ ") {
            path = normalize_path(value.split('\t').next().unwrap_or(value));
        } else if line.starts_with("@@") {
            next_line = parse_hunk_start(line);
        } else if let Some(line_number) = next_line.as_mut() {
            if line.starts_with('+') && !line.starts_with("+++") {
                locations.insert((path.clone(), *line_number));
                *line_number += 1;
            } else if !line.starts_with('-') {
                locations.insert((path.clone(), *line_number));
                *line_number += 1;
            }
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

#[derive(Clone, Debug)]
enum JsonValue {
    Null,
    Bool,
    Number(i64),
    String(String),
    Array(Vec<Box<JsonValue>>),
    Object(std::collections::BTreeMap<String, Box<JsonValue>>),
}

impl JsonValue {
    fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    text: &'a [u8],
    position: usize,
}

impl<'a> JsonParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text: text.as_bytes(),
            position: 0,
        }
    }

    fn parse(mut self) -> Result<JsonValue, ReviewParseError> {
        let value = self.value()?;
        self.space();
        (self.position == self.text.len())
            .then_some(value)
            .ok_or_else(|| parse_error("trailing JSON data"))
    }

    fn value(&mut self) -> Result<JsonValue, ReviewParseError> {
        self.space();
        match self.text.get(self.position) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(JsonValue::String),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b't') => self.literal(b"true", JsonValue::Bool),
            Some(b'f') => self.literal(b"false", JsonValue::Bool),
            Some(b'n') => self.literal(b"null", JsonValue::Null),
            _ => Err(parse_error("invalid JSON value")),
        }
    }

    fn object(&mut self) -> Result<JsonValue, ReviewParseError> {
        self.position += 1;
        let mut result = std::collections::BTreeMap::new();
        loop {
            self.space();
            if self.take(b'}') {
                return Ok(JsonValue::Object(result));
            }
            let key = self.string()?;
            self.space();
            if !self.take(b':') {
                return Err(parse_error("missing JSON colon"));
            }
            result.insert(key, Box::new(self.value()?));
            self.space();
            if self.take(b'}') {
                return Ok(JsonValue::Object(result));
            }
            if !self.take(b',') {
                return Err(parse_error("missing JSON separator"));
            }
        }
    }

    fn array(&mut self) -> Result<JsonValue, ReviewParseError> {
        self.position += 1;
        let mut result = Vec::new();
        loop {
            self.space();
            if self.take(b']') {
                return Ok(JsonValue::Array(result));
            }
            result.push(Box::new(self.value()?));
            self.space();
            if self.take(b']') {
                return Ok(JsonValue::Array(result));
            }
            if !self.take(b',') {
                return Err(parse_error("missing JSON array separator"));
            }
        }
    }

    fn string(&mut self) -> Result<String, ReviewParseError> {
        if !self.take(b'"') {
            return Err(parse_error("invalid JSON string"));
        }
        let mut result = String::new();
        while let Some(byte) = self.text.get(self.position).copied() {
            self.position += 1;
            match byte {
                b'"' => return Ok(result),
                b'\\' => {
                    let escaped = self
                        .text
                        .get(self.position)
                        .copied()
                        .ok_or_else(|| parse_error("invalid JSON escape"))?;
                    self.position += 1;
                    result.push(match escaped {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        _ => '?',
                    });
                }
                value if value.is_ascii() && !value.is_ascii_control() => {
                    result.push(value as char);
                }
                _ => return Err(parse_error("invalid JSON string")),
            }
        }
        Err(parse_error("unterminated JSON string"))
    }

    fn number(&mut self) -> Result<JsonValue, ReviewParseError> {
        let start = self.position;
        while self
            .text
            .get(self.position)
            .is_some_and(|byte| byte.is_ascii_digit() || *byte == b'-')
        {
            self.position += 1;
        }
        let number = std::str::from_utf8(&self.text[start..self.position])
            .ok()
            .and_then(|text| text.parse().ok())
            .ok_or_else(|| parse_error("invalid JSON number"))?;
        Ok(JsonValue::Number(number))
    }

    fn literal(&mut self, literal: &[u8], value: JsonValue) -> Result<JsonValue, ReviewParseError> {
        if self.text.get(self.position..self.position + literal.len()) == Some(literal) {
            self.position += literal.len();
            Ok(value)
        } else {
            Err(parse_error("invalid JSON literal"))
        }
    }

    fn space(&mut self) {
        while self
            .text
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn take(&mut self, expected: u8) -> bool {
        if self.text.get(self.position) == Some(&expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }
}
