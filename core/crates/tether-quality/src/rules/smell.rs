//! Tree and batch based smell rules.

use std::collections::HashMap;

use super::smell_text::{
    commented_out_code, dead_code, debug_print, magic_number, swallowed_error, todo_without_issue,
};
use super::util::{
    changed, end_line, is_error_region, is_generated, is_test_file, line, node_text, nodes,
    profile_node,
};
use crate::{Finding, Language, Rule, RuleContext, Severity, language_profile};

struct SmellRule {
    id: &'static str,
    check_fn: fn(&RuleContext<'_>) -> Vec<Finding>,
}

impl Rule for SmellRule {
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

/// Returns all code smell rules.
pub(crate) fn smell_rules() -> Vec<Box<dyn Rule>> {
    [
        (
            "smell.function_length",
            function_length as fn(&RuleContext<'_>) -> Vec<Finding>,
        ),
        ("smell.nesting_depth", nesting_depth),
        ("smell.parameter_count", parameter_count),
        ("smell.file_length", file_length),
        ("smell.large_type", large_type),
        ("smell.duplicate_block", duplicate_block),
        ("smell.dead_code", dead_code),
        ("smell.commented_out_code", commented_out_code),
        ("smell.magic_number", magic_number),
        ("smell.swallowed_error", swallowed_error),
        ("smell.debug_print", debug_print),
        ("smell.todo_without_issue", todo_without_issue),
    ]
    .into_iter()
    .map(|(id, check_fn)| Box::new(SmellRule { id, check_fn }) as Box<dyn Rule>)
    .collect()
}

fn function_length(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let profile = language_profile(parsed.language);
    nodes(ctx)
        .into_iter()
        .filter_map(|node| {
            if !profile_node(node, parsed.language, profile.function_kinds) || is_error_region(node)
            {
                return None;
            }
            let limit = if parsed.language == Language::Kotlin
                && node_text(node, &parsed.source).contains("@Composable")
            {
                ctx.thresholds.kotlin_composable_lines
            } else {
                ctx.thresholds.function_lines
            };
            let start = line(node);
            let end = end_line(node);
            (end.saturating_sub(start) + 1 > limit && changed(ctx, start, end)).then(|| {
                super::util::candidate(
                    ctx,
                    "smell.function_length",
                    Severity::Warn,
                    start,
                    end,
                    "Function exceeds the line limit.",
                    "Split the function into smaller operations.",
                )
            })
        })
        .collect()
}

fn nesting_depth(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let profile = language_profile(parsed.language);
    let all = nodes(ctx);
    let mut result = Vec::new();
    for function in all
        .iter()
        .copied()
        .filter(|node| profile_node(*node, parsed.language, profile.function_kinds))
    {
        let mut deepest = None;
        for node in all.iter().copied().filter(|node| {
            function.start_byte() <= node.start_byte()
                && node.end_byte() <= function.end_byte()
                && profile_node(*node, parsed.language, profile.block_kinds)
        }) {
            let depth = all
                .iter()
                .copied()
                .filter(|ancestor| {
                    ancestor.start_byte() >= function.start_byte()
                        && ancestor.start_byte() <= node.start_byte()
                        && node.end_byte() <= ancestor.end_byte()
                        && profile_node(*ancestor, parsed.language, profile.block_kinds)
                })
                .count() as u32;
            if deepest.is_none_or(|(_, current)| depth > current) {
                deepest = Some((line(node), depth));
            }
        }
        if let Some((at, depth)) = deepest
            && depth > ctx.thresholds.nesting
            && changed(ctx, at, at)
        {
            result.push(super::util::candidate(
                ctx,
                "smell.nesting_depth",
                Severity::Warn,
                at,
                at,
                "Nesting exceeds the depth limit.",
                "Flatten the branches or extract a helper.",
            ));
        }
    }
    result
}

fn parameter_count(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let profile = language_profile(parsed.language);
    nodes(ctx)
        .into_iter()
        .filter_map(|function| {
            if !profile_node(function, parsed.language, profile.function_kinds) {
                return None;
            }
            let text = node_text(function, &parsed.source);
            if parsed.language == Language::Kotlin && text.contains("@Composable") {
                return None;
            }
            let count = nodes(ctx)
                .into_iter()
                .find(|node| {
                    function.start_byte() <= node.start_byte()
                        && node.end_byte() <= function.end_byte()
                        && profile_node(*node, parsed.language, profile.parameter_kinds)
                })
                .map_or(0, |node| {
                    node_text(node, &parsed.source)
                        .split(',')
                        .filter(|part| !part.trim().is_empty())
                        .count() as u32
                });
            let start = line(function);
            (count > ctx.thresholds.parameters && changed(ctx, start, end_line(function))).then(
                || {
                    super::util::candidate(
                        ctx,
                        "smell.parameter_count",
                        Severity::Warn,
                        start,
                        start,
                        "Function has too many parameters.",
                        "Group related values into a type.",
                    )
                },
            )
        })
        .collect()
}

fn file_length(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    if is_generated(&parsed.source) {
        return Vec::new();
    }
    let limit = if is_test_file(ctx.path, &parsed.source) {
        ctx.thresholds.test_file_lines
    } else {
        ctx.thresholds.file_lines
    };
    let count = parsed.source.lines().count() as u32;
    (count > limit && !ctx.changed_lines.is_empty())
        .then(|| {
            super::util::candidate(
                ctx,
                "smell.file_length",
                Severity::Warn,
                1,
                count,
                "File exceeds the line limit.",
                "Split the file by responsibility.",
            )
        })
        .into_iter()
        .collect()
}

fn large_type(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    let kinds = [
        "struct_item",
        "class_declaration",
        "class_definition",
        "interface_declaration",
        "enum_declaration",
        "object_declaration",
    ];
    nodes(ctx)
        .into_iter()
        .filter_map(|node| {
            if !kinds.contains(&node.kind()) {
                return None;
            }
            let text = node_text(node, &parsed.source);
            let fields = text
                .lines()
                .filter(|line| {
                    line.contains(':') || line.contains(" var ") || line.contains(" val ")
                })
                .count() as u32;
            let functions = nodes(ctx)
                .into_iter()
                .filter(|child| {
                    node.start_byte() <= child.start_byte()
                        && child.end_byte() <= node.end_byte()
                        && language_profile(parsed.language)
                            .function_kinds
                            .contains(&child.kind())
                })
                .count() as u32;
            (fields > ctx.thresholds.struct_fields || functions > ctx.thresholds.struct_functions)
                .then(|| {
                    super::util::candidate(
                        ctx,
                        "smell.large_type",
                        Severity::Warn,
                        line(node),
                        end_line(node),
                        "Type has too many members.",
                        "Split the type into focused types.",
                    )
                })
                .filter(|_| changed(ctx, line(node), end_line(node)))
        })
        .collect()
}

fn duplicate_block(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let size = ctx.thresholds.duplicate_lines as usize;
    if size == 0 {
        return Vec::new();
    }
    let mut blocks: HashMap<String, Vec<(std::path::PathBuf, u32, u32)>> = HashMap::new();
    for file in &ctx.batch.files {
        let Some(text) = file.new_text.as_deref() else {
            continue;
        };
        let lines: Vec<_> = text.lines().collect();
        if lines.len() < size {
            continue;
        }
        for start in 0..lines.len().saturating_sub(size).saturating_add(1) {
            let block = lines[start..start + size]
                .iter()
                .map(|line| {
                    line.split("//")
                        .next()
                        .unwrap_or_default()
                        .split('#')
                        .next()
                        .unwrap_or_default()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("\n");
            if block.lines().all(str::is_empty) {
                continue;
            }
            blocks.entry(block).or_default().push((
                file.path.clone(),
                start as u32 + 1,
                start as u32 + size as u32,
            ));
        }
    }
    blocks
        .values()
        .filter(|values| values.len() > 1)
        .flat_map(|values| values.iter())
        .filter_map(|(path, start, end)| {
            if path != ctx.path || !changed(ctx, *start, *end) {
                return None;
            }
            Some({
                super::util::candidate(
                    ctx,
                    "smell.duplicate_block",
                    Severity::Warn,
                    *start,
                    *end,
                    "Identical code appears elsewhere in the batch.",
                    "Extract shared code or change the repeated logic.",
                )
            })
        })
        .collect()
}
