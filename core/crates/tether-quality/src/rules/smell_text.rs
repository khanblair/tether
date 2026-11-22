//! Text-oriented smell rules.

use regex::Regex;

use super::util::{
    changed, comment_text, has_changed_line, is_test_file, line, node_text, nodes,
    parse_comment_code, profile_node,
};
use crate::{Finding, Language, RuleContext, Severity, language_profile};

pub(super) fn dead_code(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let mut result = Vec::new();
    let Some(pattern) = Regex::new(r"(?m)^\s*(?:(?:pub|export)\s+)?(?:async\s+)?(?:fn|function|def|fun|func)\s+([A-Za-z_][A-Za-z0-9_]*)").ok() else { return result };
    for capture in pattern.captures_iter(&parsed.source) {
        let Some(name) = capture.get(1) else { continue };
        let declaration = capture.get(0).map(|m| m.as_str()).unwrap_or_default();
        if declaration.contains("pub ") || declaration.contains("export ") {
            continue;
        }
        if parsed.language == Language::Python && !name.as_str().starts_with('_') {
            continue;
        }
        if matches!(parsed.language, Language::Kotlin | Language::Swift)
            && !declaration.contains("private")
        {
            continue;
        }
        let uses = parsed.source.matches(name.as_str()).count();
        let start = parsed.source[..name.start()].lines().count() as u32;
        if uses <= 1 && changed(ctx, start, start) {
            result.push(super::util::candidate(
                ctx,
                "smell.dead_code",
                Severity::Warn,
                start,
                start,
                "Private function is never referenced.",
                "Remove it or call it from the file.",
            ));
        }
    }
    result
}

pub(super) fn commented_out_code(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let comments: Vec<_> = parsed
        .source
        .lines()
        .enumerate()
        .filter_map(|(index, text)| comment_text(text).map(|value| (index as u32 + 1, value)))
        .collect();
    let mut result = Vec::new();
    for pair in comments.windows(2) {
        if pair[1].0 != pair[0].0 + 1
            || !parse_comment_code(parsed.language, &format!("{}\n{}", pair[0].1, pair[1].1))
        {
            continue;
        }
        if changed(ctx, pair[0].0, pair[1].0) {
            result.push(super::util::candidate(
                ctx,
                "smell.commented_out_code",
                Severity::Warn,
                pair[0].0,
                pair[1].0,
                "Commented lines look like removed code.",
                "Delete the old code or restore it as code.",
            ));
        }
    }
    result
}

pub(super) fn magic_number(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    if is_test_file(ctx.path, &parsed.source) {
        return Vec::new();
    }
    let profile = language_profile(parsed.language);
    let mut result = Vec::new();
    for node in nodes(ctx)
        .into_iter()
        .filter(|node| profile_node(*node, parsed.language, profile.number_kinds))
    {
        let text = node_text(node, &parsed.source).trim();
        if ["0", "1", "-1"].contains(&text) || text.is_empty() || !has_changed_line(ctx, line(node))
        {
            continue;
        }
        let before = parsed
            .source
            .get(node.start_byte().saturating_sub(40)..node.start_byte())
            .unwrap_or_default();
        if before.contains("const ")
            || before.contains("static ")
            || before.contains("enum ")
            || before.contains("default")
        {
            continue;
        }
        result.push(super::util::candidate(
            ctx,
            "smell.magic_number",
            Severity::Warn,
            line(node),
            line(node),
            "Numeric literal needs a named explanation.",
            "Name the value or document why it is fixed.",
        ));
    }
    result
}

pub(super) fn swallowed_error(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let mut result = Vec::new();
    if let Ok(empty) = Regex::new(
        r"(?s)(?:catch\s*(?:\([^)]*\))?|except[^:]*:|if\s+err\s*!=\s*nil\s*)\s*(?:\{\s*\}|:\s*(?:pass\s*)?(?:\n|$))",
    ) {
        for capture in empty.find_iter(&parsed.source) {
            let start = parsed.source[..capture.start()].lines().count() as u32 + 1;
            if changed(ctx, start, start) {
                result.push(super::util::candidate(
                    ctx,
                    "smell.swallowed_error",
                    Severity::Warn,
                    start,
                    start,
                    "Error handler has an empty body.",
                    "Handle the error or return it.",
                ));
            }
        }
    }
    if parsed.language == Language::Rust
        && let Ok(pattern) =
            Regex::new(r"let\s*_\s*=\s*([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s*\(")
    {
        for capture in pattern.captures_iter(&parsed.source) {
            let callee = capture.get(1).map(|m| m.as_str()).unwrap_or_default();
            if !language_profile(parsed.language)
                .ignored_result_calls
                .iter()
                .any(|name| callee.ends_with(name))
            {
                continue;
            }
            let start = parsed.source[..capture.get(0).map_or(0, |value| value.start())]
                .lines()
                .count() as u32
                + 1;
            if changed(ctx, start, start) {
                result.push(super::util::candidate(
                    ctx,
                    "smell.swallowed_error",
                    Severity::Warn,
                    start,
                    start,
                    "Result is ignored.",
                    "Handle the returned error.",
                ));
            }
        }
    }
    result
}

pub(super) fn debug_print(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    if is_test_file(ctx.path, &parsed.source)
        || ctx.path.to_string_lossy().ends_with("main.rs")
        || ctx.path.to_string_lossy().contains("/bin/")
    {
        return Vec::new();
    }
    let Some(pattern) = Regex::new(r"(?i)(println!|dbg!|eprintln!|console\.log|print\s*\(|System\.out\.println|fmt\.Println|NSLog|\bputs\s*\(|var_dump|\becho\s+)").ok() else { return Vec::new() };
    pattern
        .find_iter(&parsed.source)
        .filter_map(|found| {
            let start = parsed.source[..found.start()].lines().count() as u32 + 1;
            changed(ctx, start, start).then(|| {
                super::util::candidate(
                    ctx,
                    "smell.debug_print",
                    Severity::Warn,
                    start,
                    start,
                    "Debug output is present in production code.",
                    "Use the project's logging path or remove the print.",
                )
            })
        })
        .collect()
}

pub(super) fn todo_without_issue(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    parsed
        .source
        .lines()
        .enumerate()
        .filter_map(|(index, text)| {
            let comment = comment_text(text)?;
            if !Regex::new(r"(?i)\b(TODO|FIXME|XXX)\b").is_ok_and(|re| re.is_match(comment)) {
                return None;
            }
            let has_reference =
                Regex::new(r"(#\d+|P\d+-\d+|https?://)").is_ok_and(|re| re.is_match(comment));
            let line = index as u32 + 1;
            (has_changed_line(ctx, line) && !has_reference).then(|| {
                super::util::candidate(
                    ctx,
                    "smell.todo_without_issue",
                    Severity::Warn,
                    line,
                    line,
                    "TODO comment has no issue reference.",
                    "Add an issue or task id to the comment.",
                )
            })
        })
        .collect()
}
