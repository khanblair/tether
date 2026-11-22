//! Reviewer prompt and reply parsing tests.

use tether_quality::{QualityReport, ReviewInput, Severity, parse_review, review_prompt};

const DIFF: &str = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,2 +1,3 @@\n fn a() {}\n+let x = 1;\n+let y = 2;\n";

fn reply(findings: &str) -> String {
    format!("Here is my review.\n{{\"findings\":{findings},\"summary\":\"ok\"}}\nThanks.")
}

#[test]
fn a_finding_inside_the_diff_is_kept_with_its_severity() {
    let parsed = parse_review(
        &reply(r#"[{"path":"src/a.rs","line":2,"severity":"block","message":"bug"}]"#),
        DIFF,
    )
    .unwrap();
    assert_eq!(parsed.summary, "ok");
    assert_eq!(parsed.dropped, 0);
    assert_eq!(parsed.findings.len(), 1);
    assert_eq!(parsed.findings[0].path, "src/a.rs");
    assert_eq!(parsed.findings[0].line, 2);
    assert_eq!(parsed.findings[0].severity, Severity::Block);
    assert_eq!(parsed.findings[0].message, "bug");
}

#[test]
fn findings_outside_the_diff_or_malformed_are_dropped_and_counted() {
    let findings = r#"[
        {"path":"src/a.rs","line":3,"message":"fine"},
        {"path":"src/other.rs","line":2,"message":"wrong file"},
        {"path":"src/a.rs","line":99,"message":"wrong line"},
        {"path":"src/a.rs","line":0,"message":"zero"},
        {"path":"src/a.rs","line":-4,"message":"negative"},
        {"path":"src/a.rs","message":"no line"},
        {"line":2,"message":"no path"},
        "not an object"
    ]"#;
    let parsed = parse_review(&reply(findings), DIFF).unwrap();
    assert_eq!(parsed.findings.len(), 1);
    assert_eq!(parsed.findings[0].severity, Severity::Info);
    assert_eq!(parsed.dropped, 7);
}

#[test]
fn a_reply_without_a_usable_object_is_an_error() {
    assert!(parse_review("no json here", DIFF).is_err());
    assert!(parse_review("{\"findings\": [", DIFF).is_err());
    assert!(parse_review("{not json}", DIFF).is_err());
    assert_eq!(
        parse_review("[1, 2, 3] {\"summary\": 1}", DIFF)
            .unwrap()
            .summary,
        ""
    );
}

#[test]
fn braces_inside_strings_do_not_end_the_object_early() {
    let text = r#"{"findings":[{"path":"src/a.rs","line":2,"message":"use } carefully {"}],"summary":"a } b"}"#;
    let parsed = parse_review(text, DIFF).unwrap();
    assert_eq!(parsed.summary, "a } b");
    assert_eq!(parsed.findings[0].message, "use } carefully {");
}

#[test]
fn at_most_fifty_findings_and_five_hundred_message_characters_are_kept() {
    let item = format!(
        r#"{{"path":"src/a.rs","line":2,"message":"{}"}}"#,
        "x".repeat(900)
    );
    let findings = format!("[{}]", vec![item; 60].join(","));
    let parsed = parse_review(&reply(&findings), DIFF).unwrap();
    assert_eq!(parsed.findings.len(), 50);
    assert_eq!(parsed.findings[0].message.chars().count(), 500);
}

#[test]
fn the_prompt_names_the_task_and_cuts_a_long_diff_at_a_hunk() {
    let report = QualityReport::new(Vec::new());
    let big = format!("{DIFF}{}", "+filler line\n".repeat(400));
    let prompt = review_prompt(&ReviewInput {
        diff: &big,
        task: "fix the parser",
        report: &report,
        max_diff_chars: 200,
    });
    assert!(prompt.user.contains("fix the parser"));
    assert!(prompt.user.contains("[diff truncated at hunk boundary]"));
    assert!(prompt.user.len() < big.len());
    assert!(prompt.system.contains("JSON"));
    let short = review_prompt(&ReviewInput {
        diff: DIFF,
        task: "t",
        report: &report,
        max_diff_chars: 10_000,
    });
    assert!(!short.user.contains("truncated"));
}
