use std::collections::HashSet;
use std::fs;
use tether_workspace::PathGuard;

const FILE_CAP: usize = 64 * 1024;
const TOTAL_CAP: usize = 256 * 1024;
const FILE_COUNT_CAP: usize = 8;

/// A file mention span in the original user text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MentionSpan {
    /// Byte offset at the `@`.
    pub start: usize,
    /// Exclusive byte offset after the path.
    pub end: usize,
    /// Workspace-relative path without `@`.
    pub path: String,
}

/// A file attached to a model request but not added to the transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MentionAttachment {
    /// Workspace-relative path.
    pub path: String,
    /// UTF-8 content, capped and line-truncated.
    pub text: String,
}

/// A mention that was not attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MentionSkip {
    /// Workspace-relative path.
    pub path: String,
    /// Typed reason safe to show to the user.
    pub reason: String,
}

/// Results of resolving all mentions in one message.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MentionResolution {
    /// Files included in the model context.
    pub attached: Vec<MentionAttachment>,
    /// Mentions skipped with a reason.
    pub skipped: Vec<MentionSkip>,
}

/// Finds workspace file mentions outside inline and fenced code.
#[must_use]
pub fn parse_mentions(text: &str) -> Vec<MentionSpan> {
    let mut spans = Vec::new();
    let mut inline = false;
    let mut fence = false;
    let mut line_start = true;
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if line_start && bytes[index..].starts_with(b"```") {
            fence = !fence;
            index = line_end(bytes, index);
            line_start = true;
            continue;
        }
        if !fence && bytes[index] == b'`' {
            inline = !inline;
            index += 1;
            line_start = false;
            continue;
        }
        if !fence
            && !inline
            && let Some(end) = mention_end(bytes, index)
        {
            let path = &text[index + 1..end];
            spans.extend(safe_mention_path(path).then(|| MentionSpan {
                start: index,
                end,
                path: path.to_owned(),
            }));
            index = end;
            line_start = false;
            continue;
        }
        line_start = bytes[index] == b'\n';
        index += 1;
    }
    spans
}

/// Resolves mentions through a workspace guard and applies the size caps.
#[must_use]
pub fn resolve_mentions(guard: &PathGuard, text: &str) -> MentionResolution {
    let mut result = MentionResolution::default();
    let mut seen = HashSet::new();
    for mention in parse_mentions(text) {
        if result.attached.len() >= FILE_COUNT_CAP || !seen.insert(mention.path.clone()) {
            continue;
        }
        match attach_mention(guard, &mention.path, attached_bytes(&result)) {
            Ok(attachment) => result.attached.push(attachment),
            Err(reason) => result.skipped.push(skip(&mention.path, reason)),
        }
    }
    result
}

/// Reads one mention and caps it to the room left in the message.
///
/// `used` is the byte count already attached to the message.
fn attach_mention(
    guard: &PathGuard,
    path: &str,
    used: usize,
) -> Result<MentionAttachment, &'static str> {
    if used >= TOTAL_CAP {
        return Err("message file limit reached");
    }
    let text = read_mention(guard, path)?;
    let cap = FILE_CAP.min(TOTAL_CAP.saturating_sub(used));
    if cap == 0 {
        return Err("file content limit reached");
    }
    Ok(MentionAttachment {
        path: path.to_owned(),
        text: truncate_text(&text, cap),
    })
}

/// Reads a mentioned file as UTF-8, or returns the reason it cannot be read.
fn read_mention(guard: &PathGuard, path: &str) -> Result<String, &'static str> {
    let resolved = guard
        .resolve(path)
        .map_err(|_| "path is outside the workspace or is a symlink")?;
    let metadata = fs::symlink_metadata(&resolved).map_err(|_| "file does not exist")?;
    if !metadata.is_file() {
        return Err("path is not a file");
    }
    let bytes = fs::read(&resolved).map_err(|_| "file could not be read")?;
    String::from_utf8(bytes).map_err(|_| "file is not UTF-8")
}

fn attached_bytes(result: &MentionResolution) -> usize {
    result.attached.iter().map(|file| file.text.len()).sum()
}

/// Adds the attachments to a request-only context block.
#[must_use]
pub fn context_block(files: &[MentionAttachment]) -> String {
    let mut block = String::from("<mentioned_files>\n");
    for file in files {
        block.push_str("<file path=\"");
        block.push_str(&file.path);
        block.push_str("\">\n");
        block.push_str(&file.text);
        if !file.text.ends_with('\n') {
            block.push('\n');
        }
        block.push_str("</file>\n");
    }
    block.push_str("</mentioned_files>");
    block
}

/// Returns the end of the mention that starts with `@` at `index`, if any.
///
/// The `@` must start a word and must not be doubled. The path must be non-empty.
fn mention_end(bytes: &[u8], index: usize) -> Option<usize> {
    let starts_word = index == 0 || bytes[index - 1].is_ascii_whitespace();
    if bytes[index] != b'@' || !starts_word {
        return None;
    }
    let next = *bytes.get(index + 1)?;
    if next == b'@' {
        return None;
    }
    let end = path_end(bytes, index + 1);
    (end > index + 1).then_some(end)
}

fn path_end(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len()
        && (bytes[index].is_ascii_alphanumeric() || b"._/-".contains(&bytes[index]))
    {
        index += 1;
    }
    index
}

fn safe_mention_path(path: &str) -> bool {
    !path.is_empty() && !path.split('/').any(|segment| segment == "..")
}

fn line_end(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index] != b'\n' {
        index += 1;
    }
    index
}

fn truncate_text(text: &str, cap: usize) -> String {
    if text.len() <= cap {
        return text.to_owned();
    }
    let marker = "[truncated]";
    if cap <= marker.len() {
        return marker.chars().take(cap).collect();
    }
    let budget = cap.saturating_sub(marker.len() + 1);
    let cut = text
        .as_bytes()
        .get(..budget)
        .and_then(|prefix| prefix.iter().rposition(|byte| *byte == b'\n'))
        .unwrap_or(0);
    format!("{}\n{marker}", &text[..cut])
}

fn skip(path: &str, reason: &str) -> MentionSkip {
    MentionSkip {
        path: path.into(),
        reason: reason.into(),
    }
}
