use super::run_support::trim_output;

#[test]
fn output_cut_keeps_head_tail_and_exact_omitted_count() {
    let output = format!(
        "{}{}{}",
        "h".repeat(4_000),
        "x".repeat(100_000),
        "t".repeat(12_000)
    );
    let (trimmed, omitted) = trim_output(&output);

    assert_eq!(omitted, 100_000);
    assert!(trimmed.starts_with(&"h".repeat(4_000)));
    assert!(trimmed.contains("[... 100000 characters omitted ...]"));
    assert!(trimmed.ends_with(&"t".repeat(12_000)));
}
