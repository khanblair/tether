//! Test quality rules.

use regex::Regex;

use super::util::{changed, is_test_file, line_count};
use crate::{Finding, Rule, RuleContext, Severity};

struct TestRule {
    id: &'static str,
    severity: Severity,
    check_fn: fn(&RuleContext<'_>) -> Vec<Finding>,
}

impl Rule for TestRule {
    fn id(&self) -> &'static str {
        self.id
    }
    fn default_severity(&self) -> Severity {
        self.severity
    }
    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        (self.check_fn)(ctx)
    }
}

/// Returns all test slop rules.
pub(crate) fn test_rules() -> Vec<Box<dyn Rule>> {
    [
        (
            "slop.test_constant_assertion",
            Severity::Warn,
            test_constant_assertion as fn(&RuleContext<'_>) -> Vec<Finding>,
        ),
        (
            "slop.test_without_assertion",
            Severity::Warn,
            test_without_assertion,
        ),
        (
            "slop.test_removed_or_skipped",
            Severity::Block,
            test_removed_or_skipped,
        ),
        (
            "slop.test_mocks_unit_under_test",
            Severity::Warn,
            test_mocks_unit_under_test,
        ),
        (
            "slop.test_snapshot_without_change",
            Severity::Warn,
            test_snapshot_without_change,
        ),
    ]
    .into_iter()
    .map(|(id, severity, check_fn)| {
        Box::new(TestRule {
            id,
            severity,
            check_fn,
        }) as Box<dyn Rule>
    })
    .collect()
}

fn test_constant_assertion(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    if !is_test_file(ctx.path, &parsed.source) {
        return Vec::new();
    }
    let simple =
        Regex::new(r"(?i)assert\s*!?\s*\(\s*(?:true|1\s*==\s*1)\s*\)|assertTrue\s*\(\s*true\s*\)")
            .ok();
    let equal = Regex::new(r"(?i)(?:assert\s*!?\s*\(?|assert_eq!\s*\()\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?:==|,)\s*([A-Za-z_][A-Za-z0-9_]*)").ok();
    let expect = Regex::new(
        r"(?i)expect\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*\.toBe\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)",
    )
    .ok();
    parsed
        .source
        .lines()
        .enumerate()
        .filter_map(|(index, text)| {
            let simple_match = simple.as_ref().is_some_and(|re| re.is_match(text));
            let equal_match =
                equal
                    .as_ref()
                    .and_then(|re| re.captures(text))
                    .is_some_and(|capture| {
                        capture.get(1).map(|value| value.as_str())
                            == capture.get(2).map(|value| value.as_str())
                    });
            let expect_match = expect
                .as_ref()
                .and_then(|re| re.captures(text))
                .is_some_and(|capture| {
                    capture.get(1).map(|value| value.as_str())
                        == capture.get(2).map(|value| value.as_str())
                });
            let line = index as u32 + 1;
            (simple_match || equal_match || expect_match)
                .then_some(line)
                .filter(|line| changed(ctx, *line, *line))
                .map(|line| {
                    super::util::candidate(
                        ctx,
                        "slop.test_constant_assertion",
                        Severity::Warn,
                        line,
                        line,
                        "Assertion cannot fail.",
                        "Assert the behavior under test.",
                    )
                })
        })
        .collect()
}

fn test_without_assertion(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    if !is_test_file(ctx.path, &parsed.source) {
        return Vec::new();
    }
    let definitions = test_definitions(&parsed.source);
    definitions
        .into_iter()
        .filter_map(|(line, body)| {
            let assertion = Regex::new(
                r"(?i)(assert|expect|should|verify|\.toBe|\.toEqual|assert_eq!|assert!)",
            )
            .is_ok_and(|re| re.is_match(&body));
            (!assertion && changed(ctx, line, line)).then(|| {
                super::util::candidate(
                    ctx,
                    "slop.test_without_assertion",
                    Severity::Warn,
                    line,
                    line,
                    "Test has no assertion.",
                    "Assert the behavior that the test covers.",
                )
            })
        })
        .collect()
}

fn test_definitions(source: &str) -> Vec<(u32, String)> {
    let mut result = Vec::new();
    let lines: Vec<_> = source.lines().collect();
    for (index, text) in lines.iter().enumerate() {
        let trimmed = text.trim_start();
        let is_definition = trimmed.starts_with("def test_")
            || trimmed.starts_with("fn test_")
            || trimmed.contains("TestXxx")
            || trimmed.starts_with("func test")
            || trimmed.starts_with("@Test")
            || trimmed.starts_with("it(")
            || trimmed.starts_with("test(");
        if !is_definition {
            continue;
        }
        let end = (index + 8).min(lines.len());
        result.push((index as u32 + 1, lines[index..end].join("\n")));
    }
    result
}

fn test_removed_or_skipped(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let old = ctx.old_text.unwrap_or_default();
    let new = ctx
        .parsed
        .map(|file| file.source.as_str())
        .unwrap_or_default();
    let is_test = is_test_file(ctx.path, old) || is_test_file(ctx.path, new);
    let code_changed = ctx.batch.files.iter().any(|file| {
        !is_test_file(&file.path, file.new_text.as_deref().unwrap_or_default())
            && !snapshot_path(&file.path)
    });
    if !is_test || !code_changed {
        return Vec::new();
    }
    let mut result = Vec::new();
    if ctx.parsed.is_none() && !old.is_empty() && !test_definitions(old).is_empty() {
        result.push(super::util::candidate(
            ctx,
            "slop.test_removed_or_skipped",
            Severity::Block,
            1,
            1,
            "A test file was deleted with code changes.",
            "Restore the test or explain its removal.",
        ));
    }
    let old_names: Vec<_> = test_definitions(old)
        .into_iter()
        .map(|(_, body)| first_test_name(&body))
        .collect();
    let new_names: Vec<_> = test_definitions(new)
        .into_iter()
        .map(|(_, body)| first_test_name(&body))
        .collect();
    if old_names
        .iter()
        .any(|name| !name.is_empty() && !new_names.contains(name))
    {
        result.push(super::util::candidate(
            ctx,
            "slop.test_removed_or_skipped",
            Severity::Block,
            1,
            1,
            "A test was removed with code changes.",
            "Restore the test or explain its removal.",
        ));
    }
    let skip = Regex::new(r"(?m)(#\[ignore\]|@pytest\.mark\.skip|pytest\.skip|it\.skip|\bxit\b|@Ignore|@Disabled|t\.Skip)").ok();
    if let Some(skip) = skip {
        let old_count = skip.find_iter(old).count();
        if skip.find_iter(new).count() > old_count {
            result.push(super::util::candidate(
                ctx,
                "slop.test_removed_or_skipped",
                Severity::Block,
                1,
                1,
                "A test was skipped with code changes.",
                "Run the test or explain the skip.",
            ));
        }
    }
    result
}

fn first_test_name(body: &str) -> String {
    Regex::new(r"(?:def|fn|func)\s+(test[_A-Za-z0-9]*)")
        .ok()
        .and_then(|re| re.captures(body))
        .and_then(|capture| capture.get(1).map(|value| value.as_str().to_owned()))
        .unwrap_or_else(|| body.lines().next().unwrap_or_default().trim().to_owned())
}

fn test_mocks_unit_under_test(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(parsed) = ctx.parsed else {
        return Vec::new();
    };
    if !is_test_file(ctx.path, &parsed.source) {
        return Vec::new();
    }
    let name = ctx
        .path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .replace("_test", "")
        .replace("Test", "")
        .to_ascii_lowercase();
    if name.is_empty() {
        return Vec::new();
    }
    let pattern =
        Regex::new(r#"(?i)(?:mock|stub|patch)\s*[(<\"']\s*([A-Za-z_][A-Za-z0-9_]*)"#).ok();
    let Some(pattern) = pattern else {
        return Vec::new();
    };
    pattern
        .captures_iter(&parsed.source)
        .filter_map(|capture| {
            let mocked = capture.get(1)?.as_str().to_ascii_lowercase();
            if !mocked.contains(&name) && !name.contains(&mocked) {
                return None;
            }
            let line = line_count(&parsed.source[..capture.get(0)?.start()]) + 1;
            changed(ctx, line, line).then(|| {
                super::util::candidate(
                    ctx,
                    "slop.test_mocks_unit_under_test",
                    Severity::Warn,
                    line,
                    line,
                    "Test mocks the unit under test.",
                    "Mock only the unit's dependencies.",
                )
            })
        })
        .collect()
}

fn test_snapshot_without_change(ctx: &RuleContext<'_>) -> Vec<Finding> {
    if !snapshot_path(ctx.path) {
        return Vec::new();
    }
    let code_change = ctx.batch.files.iter().any(|file| {
        !is_test_file(&file.path, file.new_text.as_deref().unwrap_or_default())
            && !snapshot_path(&file.path)
    });
    if code_change || ctx.changed_lines.is_empty() {
        return Vec::new();
    }
    vec![super::util::candidate(
        ctx,
        "slop.test_snapshot_without_change",
        Severity::Warn,
        1,
        1,
        "Snapshot changed without a code change.",
        "Explain the snapshot update or revert it.",
    )]
}

fn snapshot_path(path: &std::path::Path) -> bool {
    let text = path.to_string_lossy();
    text.contains("__snapshots__") || text.ends_with(".snap") || text.contains("snapshots/")
}
