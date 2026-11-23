use std::path::Path;

use super::CiFailure;

pub(super) fn location_from_frame(
    value: &str,
    prefix: &str,
) -> (Option<String>, Option<u32>, Option<u32>) {
    let value = value
        .rsplit_once('(')
        .map_or(value, |(_, rest)| rest.trim_end_matches(')'));
    location(value, prefix)
}

pub(super) fn failure_message(location_text: &str, message: &str, prefix: &str) -> String {
    let path = location_text
        .rsplitn(3, ':')
        .last()
        .unwrap_or(location_text);
    let outside = Path::new(path).is_absolute() && !path.starts_with(prefix);
    let unsafe_path = path.split('/').any(|part| part == "..");
    if outside || unsafe_path {
        format!("{location_text}: {message}")
    } else {
        message.to_owned()
    }
}

pub(super) fn location(value: &str, prefix: &str) -> (Option<String>, Option<u32>, Option<u32>) {
    let value = value.trim_end_matches(':');
    let mut parts = value.rsplitn(3, ':');
    let last = parts.next().and_then(|part| part.parse().ok());
    let middle = parts.next();
    let (path, line_number, column) = match middle.and_then(|part| part.parse().ok()) {
        Some(line_number) => (parts.next().unwrap_or(value), line_number, last),
        None => (middle.unwrap_or(value), last.unwrap_or_default(), None),
    };
    let is_absolute = Path::new(path).is_absolute();
    let outside = is_absolute && !path.starts_with(prefix);
    let unsafe_path = path.split('/').any(|part| part == "..");
    if outside || unsafe_path || last.is_none() {
        return (None, None, None);
    }
    let file = path
        .strip_prefix(prefix)
        .unwrap_or(path)
        .trim_start_matches('/')
        .to_owned();
    (Some(file), Some(line_number), column)
}

pub(super) fn safe_file(path: &str, prefix: &str) -> Option<String> {
    let path = Path::new(path);
    let outside = path.is_absolute() && !path.starts_with(prefix);
    let unsafe_path = path
        .components()
        .any(|part| part == std::path::Component::ParentDir);
    if outside || unsafe_path || path.as_os_str().is_empty() {
        return None;
    }
    let rendered = path.to_string_lossy();
    Some(
        rendered
            .strip_prefix(prefix)
            .unwrap_or(&rendered)
            .trim_start_matches('/')
            .to_owned(),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Failure fields mirror the public parser result and stay at one call site per format."
)]
pub(super) fn failure(
    framework: &str,
    test_name: Option<String>,
    file: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
    message: &str,
    excerpt: &str,
) -> CiFailure {
    CiFailure {
        framework: framework.into(),
        test_name,
        file,
        line,
        column,
        message: message.trim().to_owned(),
        excerpt: excerpt.chars().take(600).collect(),
    }
}
