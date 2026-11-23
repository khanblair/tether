use std::fs;

use tether_workspace::PathGuard;

use super::{QualityGate, pending_change};

#[test]
fn gate_reports_a_function_over_the_default_limit() {
    let root = tempfile::tempdir().expect("tempdir");
    let text = format!("pub fn too_long(input: i32) {{\n{}\n}}\n", "\n".repeat(53));
    fs::write(root.path().join("lib.rs"), &text).expect("write source");
    let gate = std::sync::Arc::new(QualityGate::new(
        PathGuard::new(root.path()).expect("guard"),
    ));

    let report = gate
        .run_blocking(vec![pending_change("lib.rs".into(), None)])
        .expect("quality report");

    assert_eq!(report.skipped, 0);
    assert_eq!(report.report.counts.warn, 1);
    assert_eq!(report.report.findings[0].rule_id, "smell.function_length");
    assert_eq!(report.report.findings[0].line, 1);
}
