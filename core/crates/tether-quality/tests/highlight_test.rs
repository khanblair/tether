//! Syntax highlighting tests.

use tether_quality::{HighlightClass, Language, highlight};

#[test]
fn highlight_returns_sorted_utf8_safe_spans() {
    let source = "fn greet() { let value = \"héllo\"; // note\n}";
    let spans = highlight(Language::Rust, source);
    assert!(
        spans
            .windows(2)
            .all(|pair| pair[0].start_byte <= pair[1].start_byte)
    );
    assert!(
        spans
            .iter()
            .all(|span| source.is_char_boundary(span.start_byte)
                && source.is_char_boundary(span.end_byte))
    );
    assert!(
        spans
            .iter()
            .any(|span| span.class == HighlightClass::Keyword)
    );
    assert!(
        spans
            .iter()
            .any(|span| span.class == HighlightClass::String)
    );
}

#[test]
fn highlight_skips_oversize_source() {
    let source = "x".repeat(512 * 1024 + 1);
    assert_eq!(highlight(Language::Rust, &source), Vec::new());
}
