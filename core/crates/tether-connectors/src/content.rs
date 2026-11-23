//! Content type and charset helpers.

pub(crate) fn header_charset(value: &str) -> Option<String> {
    value.split(';').skip(1).find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        name.trim()
            .eq_ignore_ascii_case("charset")
            .then(|| value.trim().trim_matches(['"', '\'']).to_owned())
    })
}

pub(crate) fn meta_charset(body: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(body).to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(relative) = text[cursor..].find("<meta") {
        let start = cursor + relative;
        let end = text[start..].find('>')? + start;
        if let Some(charset) = find_charset(&text[start..end]) {
            return Some(charset);
        }
        cursor = end + 1;
    }
    None
}

fn find_charset(value: &str) -> Option<String> {
    let mut cursor = 0;
    while let Some(relative) = value[cursor..].find("charset") {
        let start = cursor + relative + "charset".len();
        let rest = value[start..].trim_start();
        let rest = rest.strip_prefix('=')?.trim_start();
        let end = rest
            .find(|character: char| {
                character == ';' || character.is_whitespace() || character == '>'
            })
            .unwrap_or(rest.len());
        let charset = rest[..end].trim_matches(['"', '\'']);
        if !charset.is_empty() {
            return Some(charset.to_owned());
        }
        cursor = start;
    }
    None
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
