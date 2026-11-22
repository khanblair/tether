//! AI slop rules for source code.

use regex::Regex;

use super::util::{changed, comment_text, identifiers, is_test_file, line_count};
use crate::{Finding, Language, Rule, RuleContext, Severity, language_profile};

struct SlopRule {
    id: &'static str,
    check_fn: fn(&RuleContext<'_>) -> Vec<Finding>,
}

impl Rule for SlopRule {
    fn id(&self) -> &'static str {
        self.id
    }
    fn default_severity(&self) -> Severity {
        Severity::Warn
    }
    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        (self.check_fn)(ctx)
    }
}

/// Returns all code slop rules.
pub(crate) fn slop_rules() -> Vec<Box<dyn Rule>> {
    [
        (
            "slop.restating_comment",
            restating_comment as fn(&RuleContext<'_>) -> Vec<Finding>,
        ),
        ("slop.placeholder_body", placeholder_body),
        ("slop.unknown_import", unknown_import),
        ("slop.broad_catch", broad_catch),
        ("slop.unused_added_function", unused_added_function),
        ("slop.generic_name", generic_name),
        ("slop.banned_word_identifier", banned_word_identifier),
        ("slop.emoji_in_log", emoji_in_log),
    ]
    .into_iter()
    .map(|(id, check_fn)| Box::new(SlopRule { id, check_fn }) as Box<dyn Rule>)
    .collect()
}

fn restating_comment(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let lines: Vec<_> = parsed.source.lines().collect();
    let mut result = Vec::new();
    for index in 0..lines.len().saturating_sub(1) {
        let Some(comment) = comment_text(lines[index]) else {
            continue;
        };
        let words: Vec<_> = identifiers(comment)
            .into_iter()
            .filter(|word| !stop_word(word))
            .collect();
        if words.len() < 3 {
            continue;
        }
        let next = identifiers(lines[index + 1]);
        if words.iter().all(|word| next.contains(word))
            && changed(ctx, index as u32 + 1, index as u32 + 1)
        {
            result.push(super::util::candidate(
                ctx,
                "slop.restating_comment",
                Severity::Warn,
                index as u32 + 1,
                index as u32 + 1,
                "Comment repeats the next code line.",
                "Keep comments for intent or remove this one.",
            ));
        }
    }
    result
}

fn stop_word(word: &str) -> bool {
    [
        "a", "an", "the", "and", "or", "to", "of", "in", "on", "is", "for", "with", "this", "that",
    ]
    .contains(&word)
}

fn placeholder_body(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    if is_test_file(ctx.path, &parsed.source) {
        return Vec::new();
    }
    let pattern = match parsed.language {
        Language::Python => r"(?m)^\s*(?:pass|raise\s+NotImplementedError|\.\.\.)\s*$",
        Language::Rust => r"\b(?:todo|unimplemented)!\s*\(\s*\)",
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            r#"throw\s+new\s+Error\(\s*["']not implemented["']\s*\)"#
        }
        Language::Kotlin => r"\bTODO\s*\(\s*\)",
        Language::Go => r#"panic\(\s*["']not implemented["']\s*\)"#,
        _ => r"(?!)",
    };
    let Ok(pattern) = Regex::new(pattern) else {
        return Vec::new();
    };
    pattern
        .find_iter(&parsed.source)
        .filter_map(|found| {
            let line = line_count(&parsed.source[..found.start()]) + 1;
            if !changed(ctx, line, line) {
                return None;
            }
            let before = &parsed.source[..found.start()];
            let declaration = before.lines().rev().take(4).any(|line| {
                line.contains("trait ")
                    || line.contains("abstract")
                    || line.contains("protocol ")
                    || line.contains("interface ")
            });
            (!declaration).then(|| {
                super::util::candidate(
                    ctx,
                    "slop.placeholder_body",
                    Severity::Warn,
                    line,
                    line,
                    "Function body is a placeholder.",
                    "Implement the body or remove the function.",
                )
            })
        })
        .collect()
}

fn unknown_import(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let pattern =
        Regex::new(r#"(?m)^\s*(?:import|from|use|require)\s+(?:[\"']?)([A-Za-z_][A-Za-z0-9_.-]*)"#)
            .ok();
    let Some(pattern) = pattern else {
        return Vec::new();
    };
    pattern
        .captures_iter(&parsed.source)
        .filter_map(|capture| {
            let package = capture.get(1)?.as_str();
            if package.starts_with('.') || package.starts_with('/') {
                return None;
            }
            let root = package
                .split('.')
                .next()
                .unwrap_or(package)
                .replace('-', "_");
            let profile = language_profile(parsed.language);
            if matches!(root.as_str(), "crate" | "self" | "super") {
                return None;
            }
            let known = ctx
                .manifest
                .declared_packages
                .iter()
                .any(|item| item == &root || item.starts_with(&format!("{root}_")))
                || profile.standard_packages.iter().any(|item| *item == root);
            if known {
                return None;
            }
            let line = line_count(&parsed.source[..capture.get(0)?.start()]) + 1;
            changed(ctx, line, line).then(|| {
                super::util::candidate(
                    ctx,
                    "slop.unknown_import",
                    Severity::Warn,
                    line,
                    line,
                    "Imported package is not declared.",
                    "Declare the package or remove the import.",
                )
            })
        })
        .collect()
}

fn broad_catch(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let pattern = Regex::new(
        r"(?im)^\s*(except\s*(?:Exception)?|catch\s*\([^)]*(?:Exception|Throwable|e)?[^)]*\))\s*[:{]",
    )
    .ok();
    let Some(pattern) = pattern else {
        return Vec::new();
    };
    pattern
        .captures_iter(&parsed.source)
        .filter_map(|capture| {
            let start = capture.get(0)?.end();
            let body = parsed.source[start..].lines().next().unwrap_or_default();
            let line = line_count(&parsed.source[..capture.get(0)?.start()]) + 1;
            let statements = body
                .split(';')
                .filter(|item| !item.trim().is_empty())
                .count();
            let has_call = body.contains('(');
            (statements <= 3 && !has_call && changed(ctx, line, line)).then(|| {
                super::util::candidate(
                    ctx,
                    "slop.broad_catch",
                    Severity::Warn,
                    line,
                    line,
                    "Catch-all handler wraps code that cannot call out.",
                    "Catch a specific error or remove the handler.",
                )
            })
        })
        .collect()
}

fn unused_added_function(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let pattern = Regex::new(
        r"(?m)^\s*(?:(?:async)\s+)?(?:fn|function|def|fun|func)\s+([A-Za-z_][A-Za-z0-9_]*)",
    )
    .ok();
    let Some(pattern) = pattern else {
        return Vec::new();
    };
    pattern
        .captures_iter(&parsed.source)
        .filter_map(|capture| {
            let name = capture.get(1)?.as_str();
            let start = line_count(&parsed.source[..capture.get(0)?.start()]) + 1;
            let old = ctx.old_text.unwrap_or_default();
            if old.contains(&format!("{name}("))
                || parsed.source.matches(name).count() > 1
                || capture.get(0)?.as_str().contains("pub")
                || capture.get(0)?.as_str().contains("export")
            {
                return None;
            }
            changed(ctx, start, start).then(|| {
                super::util::candidate(
                    ctx,
                    "slop.unused_added_function",
                    Severity::Warn,
                    start,
                    start,
                    "New private function is unused in the batch.",
                    "Remove it or add a call that needs it.",
                )
            })
        })
        .collect()
}

fn generic_name(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let names: Vec<String> = include_str!("../../data/generic_names.txt")
        .lines()
        .map(str::to_owned)
        .collect();
    let pattern = Regex::new(r"\b[A-Za-z_][A-Za-z0-9_]*\b").ok();
    let Some(pattern) = pattern else {
        return Vec::new();
    };
    pattern
        .find_iter(&parsed.source)
        .filter_map(|found| {
            let name = found.as_str();
            let lower = name.to_ascii_lowercase();
            let numbered = Regex::new(r"(?:foo|bar|data|temp|utils|helper)\d+$")
                .is_ok_and(|re| re.is_match(&lower));
            if !(names.iter().any(|item| item.eq_ignore_ascii_case(name)) || numbered) {
                return None;
            }
            let line = line_count(&parsed.source[..found.start()]) + 1;
            changed(ctx, line, line).then(|| {
                super::util::candidate(
                    ctx,
                    "slop.generic_name",
                    Severity::Warn,
                    line,
                    line,
                    "Identifier is too generic.",
                    "Name the action or value by its domain meaning.",
                )
            })
        })
        .collect()
}

fn banned_word_identifier(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let words: Vec<String> = include_str!("../../data/slop_words.txt")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.contains(' '))
        .map(str::to_ascii_lowercase)
        .collect();
    let pattern = Regex::new(r#"\b[A-Za-z_][A-Za-z0-9_]*\b|\"[^\"]*\""#).ok();
    let Some(pattern) = pattern else {
        return Vec::new();
    };
    pattern
        .find_iter(&parsed.source)
        .filter_map(|found| {
            let text = found.as_str();
            let lower = text.trim_matches('"').to_ascii_lowercase();
            let matches_word = words
                .iter()
                .any(|word| lower == *word || lower.starts_with(&format!("{word}_")));
            let matches_string =
                text.starts_with('"') && words.iter().any(|word| lower.contains(word));
            if !matches_word && !matches_string {
                return None;
            }
            let line = line_count(&parsed.source[..found.start()]) + 1;
            changed(ctx, line, line).then(|| {
                super::util::candidate(
                    ctx,
                    "slop.banned_word_identifier",
                    Severity::Warn,
                    line,
                    line,
                    "Identifier or declared text uses a banned word.",
                    "Use a plain, specific word.",
                )
            })
        })
        .collect()
}

fn emoji_in_log(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let pattern = Regex::new(r#"(?s)(?:println!|console\.log|print\s*\(|NSLog|logger\.[A-Za-z]+|fmt\.Println).*?[\"']([^\"']*)[\"']"#).ok();
    let Some(pattern) = pattern else {
        return Vec::new();
    };
    pattern
        .find_iter(&parsed.source)
        .filter_map(|found| {
            if !found.as_str().chars().any(is_emoji) {
                return None;
            }
            let line = line_count(&parsed.source[..found.start()]) + 1;
            changed(ctx, line, line).then(|| {
                super::util::candidate(
                    ctx,
                    "slop.emoji_in_log",
                    Severity::Warn,
                    line,
                    line,
                    "Log message contains an emoji.",
                    "Use plain text in logs.",
                )
            })
        })
        .collect()
}

fn is_emoji(character: char) -> bool {
    matches!(character as u32, 0x1F300..=0x1FAFF | 0x2600..=0x27BF)
}
