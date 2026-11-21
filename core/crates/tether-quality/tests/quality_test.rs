//! Quality rule and manifest tests.

use std::collections::BTreeMap;
use std::path::PathBuf;

use tether_quality::{
    Batch, FileChange, ProjectManifest, Registry, Severity, Thresholds, changed_lines,
};

fn change(path: &str, old: Option<&str>, new: Option<&str>) -> FileChange {
    FileChange {
        path: PathBuf::from(path),
        old_text: old.map(str::to_owned),
        new_text: new.map(str::to_owned),
    }
}

#[test]
fn changed_lines_covers_new_middle_and_end_lines() {
    assert_eq!(changed_lines(Some("a\nb\nc"), Some("a\nx\nc")), vec![2..=2]);
    assert_eq!(changed_lines(Some("a\nb"), Some("a\nb\nc")), vec![3..=3]);
    assert_eq!(changed_lines(None, Some("a\nb")), vec![1..=2]);
}

#[test]
fn registry_reports_changed_python_rules() {
    let source = "# TODO: implement\ndef _unused(value, other, third, fourth, fifth, sixth):\n    pass\nprint(42)\n";
    let batch = Batch {
        files: vec![change("sample.py", None, Some(source))],
    };
    let findings =
        Registry::default().run(&batch, &Thresholds::default(), &ProjectManifest::default());
    let ids: Vec<_> = findings
        .iter()
        .map(|finding| finding.rule_id.as_str())
        .collect();
    assert!(ids.contains(&"slop.placeholder_body"));
    assert!(ids.contains(&"smell.debug_print"));
    assert!(ids.contains(&"smell.todo_without_issue"));
}

#[test]
fn registry_filters_outside_changes_and_applies_settings() {
    let old = "def _unused():\n    return 4\n\n\ndef _other():\n    return 5\n";
    let new = "def _unused():\n    return 4\n\n\ndef _other():\n    return 6\n";
    let mut thresholds = Thresholds {
        max_findings: 1,
        ..Thresholds::default()
    };
    thresholds
        .disabled_rules
        .insert("smell.dead_code".to_owned());
    thresholds
        .severity_overrides
        .insert("smell.magic_number".to_owned(), Severity::Info);
    let findings = Registry::default().run(
        &Batch {
            files: vec![change("sample.py", Some(old), Some(new))],
        },
        &thresholds,
        &ProjectManifest::default(),
    );
    assert!(findings.len() <= 1);
    assert!(
        findings
            .iter()
            .all(|finding| finding.rule_id != "smell.dead_code")
    );
}

#[test]
fn deleted_test_blocks_with_code_change() {
    let batch = Batch {
        files: vec![
            change(
                "thing.py",
                Some("def run():\n    return 1\n"),
                Some("def run():\n    return 2\n"),
            ),
            change(
                "test_thing.py",
                Some("def test_run():\n    assert run()\n"),
                None,
            ),
        ],
    };
    let findings =
        Registry::default().run(&batch, &Thresholds::default(), &ProjectManifest::default());
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule_id == "slop.test_removed_or_skipped"
                && finding.severity == Severity::Block)
    );
}

#[test]
fn manifest_reader_collects_supported_packages() {
    let files = BTreeMap::from([
        ("Cargo.toml", "[dependencies]\nserde-json = \"1\"\n"),
        ("package.json", "{\"dependencies\":{\"react\":\"1\"}}"),
        ("requirements.txt", "requests==1\n"),
        (
            "go.mod",
            "module example.com/app\nrequire example.com/lib v1\n",
        ),
    ]);
    let manifest = tether_quality::manifest_from_files(&|path| {
        files.get(path).map(|value| (*value).to_owned())
    });
    assert!(manifest.declared_packages.contains("serde_json"));
    assert!(manifest.declared_packages.contains("react"));
    assert!(manifest.declared_packages.contains("requests"));
    assert!(manifest.declared_packages.contains("example.com/app"));
}
