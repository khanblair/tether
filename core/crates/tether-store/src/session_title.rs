//! Builds the one-line title that a session takes from its first message.

/// Builds a session title on one line: every run of whitespace becomes one space, the result is
/// at most 80 characters, and it never ends with a space.
pub(crate) fn one_line_title(text: &str) -> String {
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = joined.chars().take(80).collect();
    cut.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::one_line_title;

    #[test]
    fn a_cut_that_lands_on_a_space_leaves_no_trailing_space() {
        let text = format!("{} b", "a".repeat(79));
        assert_eq!(one_line_title(&text), "a".repeat(79));
    }

    #[test]
    fn short_text_is_kept_whole() {
        assert_eq!(one_line_title("  fix\nthe  bug "), "fix the bug");
    }
}
