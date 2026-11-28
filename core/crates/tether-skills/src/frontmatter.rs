use std::collections::BTreeMap;

use crate::SkillError;

pub(super) struct Frontmatter {
    pub(super) fields: BTreeMap<String, String>,
    pub(super) metadata: BTreeMap<String, String>,
    pub(super) body: String,
}

#[derive(Clone, Copy)]
enum Chomping {
    Clip,
    Strip,
    Keep,
}

#[derive(Clone, Copy)]
struct BlockMarker {
    folded: bool,
    chomping: Chomping,
    indentation: Option<usize>,
}

pub(super) fn frontmatter_fields(source: &str) -> Result<Frontmatter, SkillError> {
    let lines: Vec<&str> = source.lines().collect();
    if lines.first().map(|line| line.trim()) != Some("---") {
        return Err(SkillError::MissingFrontmatter);
    }

    let mut fields = BTreeMap::new();
    let mut metadata = BTreeMap::new();
    let mut metadata_indent = None;
    let mut metadata_entry_indent = None;
    let mut index = 1;
    while index < lines.len() {
        let line = lines[index];
        if line.trim() == "---" {
            return Ok(Frontmatter {
                fields,
                metadata,
                body: lines[index + 1..].join("\n"),
            });
        }
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            index += 1;
            continue;
        }

        if let Some(next) = parse_metadata_entry(
            &lines,
            index,
            metadata_indent,
            &mut metadata_entry_indent,
            &mut metadata,
        )? {
            index = next;
            continue;
        }
        if metadata_indent.is_some() {
            metadata_indent = None;
            metadata_entry_indent = None;
        }

        if indentation(line) != 0 {
            index += 1;
            continue;
        }
        if line.trim() == "metadata:" {
            metadata_indent = Some(0);
            index += 1;
            continue;
        }

        let (key, value, next) = parse_entry(&lines, index, 0)?;
        fields.insert(key, value);
        index = next;
    }
    Err(SkillError::MissingFrontmatter)
}

fn parse_metadata_entry(
    lines: &[&str],
    index: usize,
    metadata_indent: Option<usize>,
    metadata_entry_indent: &mut Option<usize>,
    metadata: &mut BTreeMap<String, String>,
) -> Result<Option<usize>, SkillError> {
    let Some(parent_indent) = metadata_indent else {
        return Ok(None);
    };
    let line = lines[index];
    let indent = indentation(line);
    if indent <= parent_indent {
        return Ok(None);
    }
    if metadata_entry_indent.is_some_and(|entry| indent != entry)
        || line.trim_start().starts_with('-')
    {
        return Ok(Some(index + 1));
    }
    let (key, value, next) = parse_entry(lines, index, indent)?;
    metadata_entry_indent.get_or_insert(indent);
    metadata.insert(key, value);
    Ok(Some(next))
}

fn parse_entry(
    lines: &[&str],
    index: usize,
    key_indent: usize,
) -> Result<(String, String, usize), SkillError> {
    let (key, raw_value) = lines[index]
        .split_once(':')
        .ok_or(SkillError::InvalidFrontmatter)?;
    let key = key.trim();
    if key.is_empty() {
        return Err(SkillError::InvalidFrontmatter);
    }
    let raw_value = raw_value.trim();
    if let Some(marker) = BlockMarker::parse(raw_value) {
        let (value, next) = read_block(lines, index + 1, key_indent, marker);
        return Ok((key.to_owned(), value, next));
    }
    let (value, next) = read_scalar_continuation(lines, index, key_indent, raw_value);
    Ok((key.to_owned(), value, next))
}

fn read_scalar_continuation(
    lines: &[&str],
    index: usize,
    key_indent: usize,
    raw_value: &str,
) -> (String, usize) {
    let mut parts = vec![raw_value.to_owned()];
    let mut next = index + 1;
    if raw_value.is_empty() {
        return (parts.remove(0), next);
    }
    while next < lines.len() {
        let line = lines[next];
        if line.trim() == "---" {
            break;
        }
        if line.trim().is_empty() {
            next += 1;
            continue;
        }
        if indentation(line) <= key_indent {
            break;
        }
        if !line.trim_start().starts_with('#') {
            parts.push(line.trim().to_owned());
        }
        next += 1;
    }
    (scalar(&parts.join(" ")), next)
}

fn read_block(
    lines: &[&str],
    start: usize,
    key_indent: usize,
    marker: BlockMarker,
) -> (String, usize) {
    let mut content = Vec::new();
    let mut next = start;
    while next < lines.len() {
        let line = lines[next];
        if line.trim() == "---" {
            break;
        }
        if line.trim().is_empty() {
            content.push(line);
            next += 1;
            continue;
        }
        if indentation(line) <= key_indent {
            break;
        }
        if !line.trim_start().starts_with('#') {
            content.push(line);
        }
        next += 1;
    }

    let strip_indent = marker.indentation.map_or_else(
        || content.iter().find_map(|line| nonblank_indentation(line)),
        |indent| Some(key_indent.saturating_add(indent)),
    );
    let normalized = normalize_block_lines(&content, strip_indent.unwrap_or(0));
    let value = if marker.folded {
        fold_block(&normalized)
    } else {
        literal_block(&normalized)
    };
    (chomp(value, marker.chomping), next)
}

fn normalize_block_lines(lines: &[&str], strip_indent: usize) -> Vec<(String, bool)> {
    lines
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                return (String::new(), false);
            }
            let indent = indentation(line);
            let start = strip_indent.min(line.len());
            (line[start..].to_owned(), indent > strip_indent)
        })
        .collect()
}

fn fold_block(lines: &[(String, bool)]) -> String {
    let mut value = String::new();
    let mut previous_more_indented = false;
    for (line, more_indented) in lines {
        if line.is_empty() {
            value.push('\n');
            continue;
        }
        if !value.is_empty() && !value.ends_with('\n') {
            value.push(fold_separator(*more_indented, previous_more_indented));
        }
        value.push_str(line);
        previous_more_indented = *more_indented;
    }
    if !lines.is_empty() {
        value.push('\n');
    }
    value
}

fn fold_separator(more_indented: bool, previous_more_indented: bool) -> char {
    if more_indented || previous_more_indented {
        '\n'
    } else {
        ' '
    }
}

fn literal_block(lines: &[(String, bool)]) -> String {
    let mut value = String::new();
    for (index, (line, _)) in lines.iter().enumerate() {
        if index > 0 {
            value.push('\n');
        }
        value.push_str(line);
    }
    if !lines.is_empty() {
        value.push('\n');
    }
    value
}

fn chomp(mut value: String, chomping: Chomping) -> String {
    match chomping {
        Chomping::Clip => {
            let had_newline = value.ends_with('\n');
            while value.ends_with('\n') {
                value.pop();
            }
            if had_newline {
                value.push('\n');
            }
        }
        Chomping::Strip => {
            while value.ends_with('\n') {
                value.pop();
            }
        }
        Chomping::Keep => {}
    }
    value
}

fn nonblank_indentation(line: &str) -> Option<usize> {
    (!line.trim().is_empty()).then(|| indentation(line))
}

fn indentation(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

fn scalar(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|text| text.strip_suffix('\''))
        })
        .unwrap_or(value)
        .to_owned()
}

impl BlockMarker {
    fn parse(value: &str) -> Option<Self> {
        let marker = value.chars().next()?;
        if marker != '>' && marker != '|' {
            return None;
        }
        if value
            .chars()
            .skip(1)
            .any(|indicator| indicator != '-' && indicator != '+' && !indicator.is_ascii_digit())
        {
            return None;
        }
        let chomping = if value.contains('-') {
            Chomping::Strip
        } else if value.contains('+') {
            Chomping::Keep
        } else {
            Chomping::Clip
        };
        let indentation = value
            .chars()
            .skip(1)
            .find(char::is_ascii_digit)
            .map(|digit| digit as usize - '0' as usize);
        Some(Self {
            folded: marker == '>',
            chomping,
            indentation,
        })
    }
}
