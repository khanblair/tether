//! HTML cleanup and readable text conversion.

use crate::fetch::FetchError;

pub(crate) fn html_text(html: &str) -> Result<String, FetchError> {
    let clean = remove_blocks(html, "script");
    let clean = remove_blocks(&clean, "style");
    let rendered = html2text::from_read(clean.as_bytes(), 100).map_err(|_| FetchError::Decode)?;
    Ok(convert_footnotes(&rendered))
}

pub(crate) fn html_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title>")? + 7;
    let end = lower[start..].find("</title>")? + start;
    Some(html[start..end].trim().to_owned())
}

fn remove_blocks(input: &str, tag: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let start = format!("<{tag}");
    let end = format!("</{tag}>");
    let mut output = String::new();
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find(&start) {
        let begin = cursor + relative;
        output.push_str(&input[cursor..begin]);
        let Some(close) = lower[begin..].find(&end) else {
            return output;
        };
        cursor = begin + close + end.len();
    }
    output.push_str(&input[cursor..]);
    output
}

fn convert_footnotes(rendered: &str) -> String {
    let mut definitions = std::collections::HashMap::new();
    let mut content = String::new();
    for line in rendered.lines() {
        if let Some((key, url)) = line
            .strip_prefix('[')
            .and_then(|line| line.split_once("]: "))
        {
            definitions.insert(key.to_owned(), url.to_owned());
        } else {
            content.push_str(line);
            content.push('\n');
        }
    }
    let mut result = content;
    for (key, url) in definitions {
        result = replace_reference(&result, &key, &url);
    }
    result.trim_end().to_owned()
}

fn replace_reference(text: &str, key: &str, url: &str) -> String {
    let marker = format!("][{key}]");
    let mut result = text.to_owned();
    let mut search_from = 0;
    while let Some(relative) = result[search_from..].find(&marker) {
        let marker_start = search_from + relative;
        let Some(label_start) = result[..marker_start].rfind('[') else {
            break;
        };
        let end = marker_start + marker.len();
        let label = result[label_start + 1..marker_start].to_owned();
        result.replace_range(label_start..end, &format!("{label} ({url})"));
        search_from = label_start + 1;
    }
    result
}
