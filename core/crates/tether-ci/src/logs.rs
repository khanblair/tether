use std::path::Path;

mod support;
use support::{failure, failure_message, location, location_from_frame, safe_file};

/// A bounded log after control sequences and Actions wrappers are removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrimmedLog {
    /// Cleaned text.
    pub text: String,
    /// Whether content was removed.
    pub truncated: bool,
}

/// One parsed test or compiler failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiFailure {
    /// Detected framework.
    pub framework: String,
    /// Test name when present.
    pub test_name: Option<String>,
    /// Repository-relative file when safe.
    pub file: Option<String>,
    /// One-based line.
    pub line: Option<u32>,
    /// One-based column.
    pub column: Option<u32>,
    /// Failure message.
    pub message: String,
    /// Relevant log excerpt.
    pub excerpt: String,
}

/// Removes terminal and Actions formatting and keeps the useful tail.
#[must_use]
pub fn trim_log(raw: &str, max_chars: usize) -> TrimmedLog {
    let cleaned = raw.lines().map(clean_line).collect::<Vec<_>>();
    let marker = cleaned.iter().position(|line| is_failure_marker(line));
    let start = marker.map_or(cleaned.len().saturating_sub(200), |index| {
        index.saturating_sub(40)
    });
    let selected = cleaned[start..].join("\n");
    if selected.chars().count() <= max_chars {
        return TrimmedLog {
            text: selected,
            truncated: false,
        };
    }
    let lines = selected.lines().collect::<Vec<_>>();
    let omitted_marker = |count: usize| format!("[... {count} lines omitted ...]");
    let marker_len = omitted_marker(lines.len()).chars().count() + 1;
    let mut kept = Vec::new();
    let mut count = 0;
    for line in lines.iter().rev() {
        let added = line.chars().count() + usize::from(!kept.is_empty());
        if count + added > max_chars.saturating_sub(marker_len) {
            break;
        }
        kept.push(*line);
        count += added;
    }
    kept.reverse();
    let omitted = lines.len().saturating_sub(kept.len());
    let marker = omitted_marker(omitted);
    let text = if marker.chars().count() >= max_chars {
        marker.chars().take(max_chars).collect()
    } else {
        format!("{marker}\n{}", kept.join("\n"))
    };
    TrimmedLog {
        text,
        truncated: true,
    }
}

/// Parses common pytest, JavaScript, Rust, Gradle, and compiler failures.
#[must_use]
pub fn parse_failures(log: &str, workspace_prefix: &str) -> Vec<CiFailure> {
    let cleaned = log.lines().map(clean_line).collect::<Vec<_>>();
    let lines = cleaned.iter().map(String::as_str).collect::<Vec<_>>();
    let mut failures = Vec::new();
    let mut index = 0;
    while index < lines.len() && failures.len() < 50 {
        let line = lines[index].to_owned();
        if let Some(failure) = parse_pytest(&line, &lines, workspace_prefix) {
            failures.push(failure);
        } else if let Some(failure) = parse_javascript(&line, &lines, index, workspace_prefix) {
            failures.push(failure);
        } else if let Some(failure) = parse_cargo(&line, &lines, index, workspace_prefix) {
            failures.push(failure);
        } else if let Some(failure) = parse_gradle(&line, &lines, index, workspace_prefix) {
            failures.push(failure);
        } else if let Some(failure) = parse_rustc(&line, &lines, index, workspace_prefix) {
            failures.push(failure);
        } else if let Some(failure) = parse_generic(&line, workspace_prefix) {
            failures.push(failure);
        }
        index += 1;
    }
    failures
}

fn clean_line(line: &str) -> String {
    let mut output = strip_ansi(line);
    if let Some(space) = output.find(' ') {
        let prefix = &output[..space];
        if prefix.ends_with('Z')
            && prefix.len() >= 21
            && prefix.bytes().all(|byte| {
                byte == b'Z'
                    || byte == b'T'
                    || byte == b'-'
                    || byte == b':'
                    || byte == b'.'
                    || byte.is_ascii_digit()
            })
        {
            output = output[space + 1..].to_owned();
        }
    }
    if let Some(title) = output.strip_prefix("##[group]") {
        return title.to_owned();
    }
    if output.starts_with("##[endgroup]") {
        return String::new();
    }
    output
}

fn strip_ansi(value: &str) -> String {
    let mut output = String::new();
    let mut escape = false;
    for character in value.chars() {
        if escape {
            escape = !character.is_ascii_alphabetic();
            continue;
        }
        if character == '\u{1b}' {
            escape = true;
        } else {
            output.push(character);
        }
    }
    output
}

fn is_failure_marker(line: &str) -> bool {
    line.contains("##[error]")
        || line.starts_with("FAILED ")
        || line.starts_with("FAIL ")
        || line.contains("panicked at")
        || line.contains("error[")
}

fn parse_pytest(line: &str, lines: &[&str], prefix: &str) -> Option<CiFailure> {
    if let Some(rest) = line.strip_prefix("FAILED ") {
        let (path_test, message) = rest.split_once(" - ").unwrap_or((rest, "pytest failure"));
        let (path, test_name) = path_test.split_once("::").unwrap_or((path_test, path_test));
        let (file, line_number, column) = location(path, prefix);
        let file = file.or_else(|| safe_file(path, prefix));
        let excerpt = lines
            .iter()
            .find(|item| item.contains(path))
            .map_or_else(|| line.to_owned(), |item| (*item).to_owned());
        return Some(failure(
            "pytest",
            Some(test_name.to_owned()),
            file,
            line_number,
            column,
            message,
            &excerpt,
        ));
    }
    let (path, _) = line.split_once(": AssertionError")?;
    let (file, line_number, column) = location(path, prefix);
    Some(failure(
        "pytest",
        None,
        file,
        line_number,
        column,
        "AssertionError",
        line,
    ))
}

fn parse_javascript(line: &str, lines: &[&str], index: usize, prefix: &str) -> Option<CiFailure> {
    let line = line.trim_start();
    if line.starts_with("FAIL ")
        && lines[index.saturating_add(1).min(lines.len())..index.saturating_add(4).min(lines.len())]
            .iter()
            .any(|item| item.trim_start().starts_with('●'))
    {
        return None;
    }
    let fail_path = line.strip_prefix("FAIL ").map(str::trim);
    let vitest_name = line
        .strip_prefix('❯')
        .map(str::trim)
        .filter(|name| name.contains(" > "));
    let name = line
        .strip_prefix('●')
        .map(str::trim)
        .or(vitest_name)
        .or(fail_path)?;
    let frame = lines[index.saturating_add(1).min(lines.len())..]
        .iter()
        .find_map(|item| {
            let value = item.trim().strip_prefix("at ")?;
            (!value.contains("node_modules")).then(|| location_from_frame(value, prefix))
        });
    let (file, line_number, column) = frame.unwrap_or((None, None, None));
    let file = file.or_else(|| fail_path.and_then(|path| safe_file(path, prefix)));
    Some(failure(
        if vitest_name.is_some() {
            "vitest"
        } else {
            "jest"
        },
        fail_path.is_none().then(|| name.to_owned()),
        file,
        line_number,
        column,
        "JavaScript test failed",
        line,
    ))
}

fn parse_cargo(line: &str, lines: &[&str], index: usize, prefix: &str) -> Option<CiFailure> {
    let name = line.strip_prefix("---- ")?.strip_suffix(" stdout ----")?;
    let panic_index = lines[index.saturating_add(1).min(lines.len())..]
        .iter()
        .position(|item| item.contains("panicked at"))?
        + index
        + 1;
    let panic = lines[panic_index];
    let location_text = panic.split("panicked at ").nth(1)?.trim_end_matches(':');
    let (file, line_number, column) = location(location_text, prefix);
    let message = lines
        .get(panic_index + 1)
        .copied()
        .unwrap_or("cargo test failed");
    Some(failure(
        "cargo test",
        Some(name.to_owned()),
        file,
        line_number,
        column,
        message,
        &lines[index..=panic_index.min(lines.len() - 1)].join("\n"),
    ))
}

fn parse_gradle(line: &str, lines: &[&str], index: usize, prefix: &str) -> Option<CiFailure> {
    let line = line.trim_start();
    if let Some(rest) = line.strip_prefix("Test > ") {
        return parse_gradle_test(rest, line, lines, index, prefix);
    }
    if let Some(rest) = line.strip_prefix("e: file://") {
        return Some(parse_gradle_compile(rest, line, prefix));
    }
    if line.contains(": error:") {
        return parse_gradle_java(line, prefix);
    }
    None
}

fn parse_gradle_test(
    rest: &str,
    line: &str,
    lines: &[&str],
    index: usize,
    prefix: &str,
) -> Option<CiFailure> {
    let name = rest.strip_suffix(" FAILED")?;
    let frame = lines[index.saturating_add(1).min(lines.len())..]
        .iter()
        .find_map(|item| {
            item.trim()
                .strip_prefix("at ")
                .map(|value| location_from_frame(value, prefix))
        });
    let (file, line_number, column) = frame.unwrap_or((None, None, None));
    Some(failure(
        "gradle",
        Some(name.to_owned()),
        file,
        line_number,
        column,
        "Gradle test failed",
        line,
    ))
}

fn parse_gradle_compile(rest: &str, line: &str, prefix: &str) -> CiFailure {
    let (location_text, message) = rest
        .split_once(' ')
        .unwrap_or((rest, "Gradle compilation failed"));
    let (file, line_number, column) = location(location_text, prefix);
    failure(
        "gradle",
        None,
        file,
        line_number,
        column,
        &failure_message(location_text, message, prefix),
        line,
    )
}

fn parse_gradle_java(line: &str, prefix: &str) -> Option<CiFailure> {
    let (location_text, message) = line.split_once(" error: ")?;
    let java_path = location_text
        .trim_end_matches(':')
        .rsplit_once(':')
        .map_or(location_text.trim_end_matches(':'), |(path, _)| path);
    if !Path::new(java_path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("java"))
    {
        return None;
    }
    let (file, line_number, column) = location(location_text, prefix);
    Some(failure(
        "gradle",
        None,
        file,
        line_number,
        column,
        &failure_message(location_text, message, prefix),
        line,
    ))
}

fn parse_generic(line: &str, prefix: &str) -> Option<CiFailure> {
    let (location_text, message) = line
        .split_once(": error: ")
        .or_else(|| line.split_once(" error: "))?;
    let (file, line_number, column) = location(location_text, prefix);
    Some(failure(
        "generic",
        None,
        file,
        line_number,
        column,
        &failure_message(location_text, message, prefix),
        line,
    ))
}

fn parse_rustc(line: &str, lines: &[&str], index: usize, prefix: &str) -> Option<CiFailure> {
    let message = line.strip_prefix("error[")?.split_once("]:")?.1.trim();
    let location_text = lines[index.saturating_add(1).min(lines.len())..]
        .iter()
        .find_map(|item| item.trim().strip_prefix("--> "))?;
    let (file, line_number, column) = location(location_text, prefix);
    Some(failure(
        "rustc",
        None,
        file,
        line_number,
        column,
        message,
        line,
    ))
}
