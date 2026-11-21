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
            && bytes[index] == b'@'
            && (index == 0 || bytes[index - 1].is_ascii_whitespace())
        {
            if index + 1 < bytes.len() && bytes[index + 1] != b'@' {
                let end = path_end(bytes, index + 1);
                if end > index + 1 {
                    let path = &text[index + 1..end];
                    if safe_mention_path(path) {
                        spans.push(MentionSpan {
                            start: index,
                            end,
                            path: path.to_owned(),
                        });
                    }
                    index = end;
                    line_start = false;
                    continue;
                }
            }
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
        if result
            .attached
            .iter()
            .map(|file| file.text.len())
            .sum::<usize>()
            >= TOTAL_CAP
        {
            result
                .skipped
                .push(skip(&mention.path, "message file limit reached"));
            continue;
        }
        let Ok(path) = guard.resolve(&mention.path) else {
            result.skipped.push(skip(
                &mention.path,
                "path is outside the workspace or is a symlink",
            ));
            continue;
        };
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            result
                .skipped
                .push(skip(&mention.path, "file does not exist"));
            continue;
        };
        if !metadata.is_file() {
            result
                .skipped
                .push(skip(&mention.path, "path is not a file"));
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            result
                .skipped
                .push(skip(&mention.path, "file could not be read"));
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            result
                .skipped
                .push(skip(&mention.path, "file is not UTF-8"));
            continue;
        };
        let available = TOTAL_CAP.saturating_sub(
            result
                .attached
                .iter()
                .map(|file| file.text.len())
                .sum::<usize>(),
        );
        let cap = FILE_CAP.min(available);
        let text = truncate_text(&text, cap);
        if cap == 0 {
            result
                .skipped
                .push(skip(&mention.path, "file content limit reached"));
            continue;
        }
        result.attached.push(MentionAttachment {
            path: mention.path,
            text,
        });
    }
    result
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
