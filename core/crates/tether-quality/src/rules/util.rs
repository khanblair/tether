//! Shared helpers for heuristic rules.

use std::path::Path;

use regex::Regex;
use tree_sitter::Node;

use crate::{Finding, Language, RuleContext, Severity, language_profile};

pub(crate) fn node_text<'a>(node: Node<'a>, source: &'a str) -> &'a str {
    source.get(node.byte_range()).unwrap_or_default()
}

pub(crate) fn walk<'a>(node: Node<'a>, output: &mut Vec<Node<'a>>) {
    output.push(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, output);
    }
}

pub(crate) fn nodes<'a>(ctx: &'a RuleContext<'a>) -> Vec<Node<'a>> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let mut output = Vec::new();
    walk(parsed.tree.root_node(), &mut output);
    output
}

pub(crate) fn is_error_region(node: Node<'_>) -> bool {
    let mut current = Some(node);
    while let Some(value) = current {
        if value.is_error() || value.is_missing() {
            return true;
        }
        current = value.parent();
    }
    false
}

pub(crate) fn line(node: Node<'_>) -> u32 {
    node.start_position().row as u32 + 1
}

pub(crate) fn end_line(node: Node<'_>) -> u32 {
    node.end_position().row as u32 + 1
}

pub(crate) fn changed(ctx: &RuleContext<'_>, start: u32, end: u32) -> bool {
    ctx.changed_lines
        .iter()
        .any(|range| *range.start() <= end && *range.end() >= start)
}

pub(crate) fn candidate(
    ctx: &RuleContext<'_>,
    rule_id: &str,
    severity: Severity,
    start: u32,
    end: u32,
    message: &str,
    advice: &str,
) -> Finding {
    Finding {
        rule_id: rule_id.to_owned(),
        severity,
        path: ctx.path.to_path_buf(),
        line: start,
        end_line: end.max(start),
        message: message.to_owned(),
        advice: advice.to_owned(),
    }
}

pub(crate) fn profile_node(node: Node<'_>, language: Language, kinds: &[&str]) -> bool {
    let _profile = language_profile(language);
    kinds.contains(&node.kind())
}

pub(crate) fn is_test_file(path: &Path, source: &str) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.contains("test")
        || name.contains("spec")
        || source
            .lines()
            .take(20)
            .any(|line| line.contains("@Test") || line.contains("#[test]"))
}

pub(crate) fn is_generated(source: &str) -> bool {
    source.lines().take(10).any(|line| {
        let lowered = line.to_ascii_lowercase();
        lowered.contains("generated") || lowered.contains("do not edit")
    })
}

pub(crate) fn comment_text(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    for marker in ["//", "#", "--", "/*", "*", "<!--"] {
        if let Some(text) = trimmed.strip_prefix(marker) {
            return Some(text.trim().trim_end_matches("*/").trim());
        }
    }
    None
}

pub(crate) fn line_count(source: &str) -> u32 {
    if source.is_empty() {
        0
    } else {
        source.lines().count() as u32
    }
}

pub(crate) fn identifiers(text: &str) -> Vec<String> {
    Regex::new(r"[A-Za-z_][A-Za-z0-9_]*")
        .map(|regex| {
            regex
                .find_iter(text)
                .map(|m| m.as_str().to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn has_changed_line(ctx: &RuleContext<'_>, line: u32) -> bool {
    changed(ctx, line, line)
}

pub(crate) fn parse_comment_code(language: Language, text: &str) -> bool {
    let stripped = text.trim();
    let _ = language;
    let source = stripped.to_owned();
    let Ok(parsed) = crate::parse(language, &source) else {
        return false;
    };
    !parsed.has_errors()
        && (source.contains('(')
            || source.contains('=')
            || source.contains(" if ")
            || source.contains("return "))
}
