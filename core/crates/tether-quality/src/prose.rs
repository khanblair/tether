//! Prose checks shared with the slop-lint data files.

use regex::Regex;
use serde::Deserialize;

use crate::{Finding, Severity};

const WORDS: &str = include_str!("../data/slop_words.txt");
const PATTERNS: &str = include_str!("../data/slop_patterns.toml");
const OFF: &str = "<!-- slop-lint:off -->";
const ON: &str = "<!-- slop-lint:on -->";

/// The prose context used to select language-specific checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProseKind {
    /// Markdown documentation.
    Markdown,
    /// Plain text such as a commit message.
    Plain,
    /// XML element text such as UI strings.
    Xml,
    /// A commit subject and body.
    CommitMessage,
}

#[derive(Deserialize)]
struct PatternFile {
    pattern: Vec<PatternEntry>,
}

#[derive(Deserialize)]
struct PatternEntry {
    name: String,
    regex: String,
    advice: String,
}

struct ProseRule {
    name: String,
    regex: Regex,
    advice: String,
}

/// Checks prose using the shared slop-lint data files.
#[must_use]
pub fn check_prose(text: &str, kind: ProseKind) -> Vec<Finding> {
    let mut findings = Vec::new();
    let rules = prose_rules();
    let mut off = false;
    let mut fence = false;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.replace('\u{2019}', "'");
        if line.contains(OFF) {
            off = true;
        }
        let checked = !off;
        if checked && line.trim_start().as_bytes().starts_with(&[96, 96, 96]) {
            fence = !fence;
        }
        if checked && !(kind == ProseKind::Markdown && fence) {
            check_line(&line, index as u32 + 1, fence, kind, &rules, &mut findings);
        }
        if line.contains(ON) {
            off = false;
        }
    }
    if kind == ProseKind::CommitMessage {
        check_commit_shape(text, &mut findings);
    }
    findings
}

fn prose_rules() -> Vec<ProseRule> {
    let mut rules = WORDS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|word| {
            let pattern = word_pattern(word);
            Regex::new(&pattern).ok().map(|regex| ProseRule {
                name: format!("banned-word:{word}"),
                regex,
                advice: "Use a plainer word. See docs/project-overview.md writing rules."
                    .to_owned(),
            })
        })
        .collect::<Vec<_>>();
    if let Ok(file) = toml::from_str::<PatternFile>(PATTERNS) {
        rules.extend(file.pattern.into_iter().filter_map(|entry| {
            Regex::new(&format!("(?i){}", entry.regex))
                .ok()
                .map(|regex| ProseRule {
                    name: entry.name,
                    regex,
                    advice: entry.advice,
                })
        }));
    }
    rules
}

fn word_pattern(word: &str) -> String {
    let escaped = regex::escape(word);
    if word.contains(' ') {
        format!("(?i){escaped}")
    } else {
        let stem = word.strip_suffix('e').map(regex::escape);
        let ing = stem.map(|stem| format!("|{stem}ing")).unwrap_or_default();
        format!(r"(?i)\b(?:{escaped}(?:s|es|d|ed|ing|ly)?{ing})\b")
    }
}

fn check_line(
    line: &str,
    number: u32,
    in_fence: bool,
    kind: ProseKind,
    rules: &[ProseRule],
    findings: &mut Vec<Finding>,
) {
    for rule in rules.iter().filter(|rule| rule.regex.is_match(line)) {
        findings.push(prose_finding(number, &rule.name, &rule.advice));
    }
    if line.contains('\u{2014}') {
        findings.push(prose_finding(number, "em-dash", "Use two sentences."));
    }
    if !in_fence && has_exclamation(line, kind == ProseKind::Xml) {
        findings.push(prose_finding(
            number,
            "exclamation-mark",
            "No exclamation marks in docs or UI copy.",
        ));
    }
}

fn check_commit_shape(text: &str, findings: &mut Vec<Finding>) {
    let mut lines = text.lines();
    let Some(subject) = lines.next() else { return };
    if subject.chars().count() > 72 {
        findings.push(prose_finding(
            1,
            "slop.commit_subject_length",
            "Keep the commit subject to 72 characters.",
        ));
    }
    if subject.ends_with('.') {
        findings.push(prose_finding(
            1,
            "slop.commit_subject_period",
            "Do not end the commit subject with a period.",
        ));
    }
    for (index, line) in text.lines().enumerate().skip(1) {
        if line.chars().count() > 100 {
            findings.push(prose_finding(
                index as u32 + 1,
                "slop.commit_body_length",
                "Keep commit body lines to 100 characters.",
            ));
        }
    }
}

fn prose_finding(line: u32, rule_id: &str, advice: &str) -> Finding {
    Finding {
        rule_id: rule_id.to_owned(),
        severity: Severity::Warn,
        path: std::path::PathBuf::new(),
        line,
        end_line: line,
        message: rule_id.to_owned(),
        advice: advice.to_owned(),
    }
}

fn has_exclamation(line: &str, is_xml: bool) -> bool {
    let prose = if is_xml {
        xml_text(line)
    } else {
        strip_code(line)
    };
    prose.replace("![", "").contains('!')
}

fn xml_text(line: &str) -> String {
    let Ok(element) = Regex::new(r">([^<]*)<") else {
        return String::new();
    };
    element
        .captures_iter(line)
        .map(|capture| capture[1].to_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_code(line: &str) -> String {
    let Ok(code) = Regex::new(r"\x60[^\x60]*\x60|<!--.*?-->") else {
        return line.to_owned();
    };
    code.replace_all(line, "").into_owned()
}
