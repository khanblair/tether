//! Planted noncredential strings exercise the log and export redaction policy.

use crate::{Redactor, Secret};

#[test]
fn known_key_shapes_are_hidden_in_prose() {
    let redactor = Redactor::default();
    for prefix in ["sk-ant-", "sk-proj-", "sk-", "AIza", "ghp_", "github_pat_", "gsk_"] {
        let planted = format!("{prefix}abcdefgh12345678");
        let output = redactor.redact(&format!("before {planted} after"));
        assert_eq!(output, "before [REDACTED] after");
        assert!(!output.contains(&planted));
    }
}

#[test]
fn registered_unknown_values_are_hidden() {
    let mut redactor = Redactor::default();
    let planted = Secret::from_text("private-value-12345");
    redactor.register(&planted);
    let output = redactor.redact("request private-value-12345 finished");
    assert_eq!(output, "request [REDACTED] finished");
}

#[test]
fn ordinary_text_is_kept() {
    let redactor = Redactor::default();
    assert_eq!(redactor.redact("sk-short and a normal answer"), "sk-short and a normal answer");
}
