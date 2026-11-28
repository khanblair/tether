//! Builds a session's one-line title from its first message, capped at 60 characters and whole
//! words when possible.

/// Builds a session title on one line: every run of whitespace becomes one space, the result is
/// at most 60 characters, and it ends at a whole word when possible.
pub(crate) fn one_line_title(text: &str) -> String {
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = joined.chars().collect();
    if chars.len() <= 60 {
        return joined;
    }

    cut_title(&chars)
}

fn cut_title(chars: &[char]) -> String {
    let mut end = 59;
    if chars[59] != ' '
        && chars[..59].last() != Some(&' ')
        && let Some(space) = chars[..59].iter().rposition(|&character| character == ' ')
    {
        end = space;
    }
    while end > 0 && chars[end - 1] == ' ' {
        end -= 1;
    }

    let mut title: String = chars[..end].iter().collect();
    title.push('…');
    title
}

#[cfg(test)]
mod tests {
    use super::one_line_title;

    #[test]
    fn a_cut_that_lands_on_a_word_end_leaves_no_trailing_space() {
        let text = format!("{} b", "a".repeat(59));
        assert_eq!(one_line_title(&text), format!("{}…", "a".repeat(59)));
    }

    #[test]
    fn a_60_character_title_is_kept_whole() {
        let text = "a".repeat(60);

        assert_eq!(one_line_title(&text), text);
    }

    #[test]
    fn a_61_character_title_gets_an_ellipsis() {
        assert_eq!(
            one_line_title(&"a".repeat(61)),
            format!("{}…", "a".repeat(59))
        );
    }

    #[test]
    fn blairs_long_title_ends_at_a_whole_word() {
        let text = "create a simple hotel site then, simple vite code, random name, data, proper styling, run it on localhost, display link here";

        assert_eq!(
            one_line_title(text),
            "create a simple hotel site then, simple vite code, random…"
        );
    }

    #[test]
    fn multi_byte_characters_are_counted_as_characters() {
        let text = "é ".repeat(40);

        assert_eq!(one_line_title(&text), format!("{}…", "é ".repeat(29) + "é"));
        assert_eq!(one_line_title(&text).chars().count(), 60);
    }

    #[test]
    fn short_text_is_kept_whole() {
        assert_eq!(one_line_title("  fix\nthe  bug "), "fix the bug");
    }
}
