//! Content type and charset helpers.

pub(crate) fn header_charset(value: &str) -> Option<String> {
    value.split(';').skip(1).find_map(|part| {
        part.trim()
            .strip_prefix("charset=")
            .map(|value| value.trim_matches('"').to_owned())
    })
}

pub(crate) fn meta_charset(body: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(body).to_ascii_lowercase();
    let start = text.find("<meta")?;
    let end = text[start..].find('>')? + start;
    let value = text[start..end].split("charset=").nth(1)?;
    Some(
        value
            .trim_matches(|character: char| {
                character == '"' || character == '\'' || character.is_whitespace()
            })
            .split_whitespace()
            .next()?
            .to_owned(),
    )
}

pub(crate) fn is_text_type(value: &str) -> bool {
    value.starts_with("text/")
        || matches!(
            value,
            "application/json" | "application/xml" | "application/xhtml+xml"
        )
        || (value.starts_with("application/")
            && (value.ends_with("+json") || value.ends_with("+xml")))
}

pub(crate) fn is_redirect(status: u16) -> bool {
    (300..400).contains(&status)
}
