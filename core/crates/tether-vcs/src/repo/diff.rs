use super::ObjectId;
use std::collections::BTreeMap as Map;
use std::fmt::Write as FmtWrite;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct TreeFile {
    id: ObjectId,
    mode: u16,
}

pub(super) fn tree_diff(repo: &gix::Repository, old: ObjectId, new: ObjectId) -> String {
    let mut before = Map::new();
    let mut after = Map::new();
    collect_tree_files(repo, old, "", &mut before);
    collect_tree_files(repo, new, "", &mut after);
    let mut out = String::new();
    for path in before
        .keys()
        .chain(after.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        if before.get(path) == after.get(path) {
            continue;
        }
        render_file_diff(repo, path, before.get(path), after.get(path), &mut out);
    }
    out
}

pub(super) fn tree_entry(repo: &gix::Repository, id: ObjectId, path: &str) -> Option<TreeFile> {
    let mut current = id;
    let mut components = path.split('/').peekable();
    while let Some(component) = components.next() {
        let tree_object = repo.find_tree(current).ok()?;
        let tree = tree_object.decode().ok()?;
        let entry = tree
            .entries
            .iter()
            .find(|entry| entry.filename == component.as_bytes())?;
        current = (*entry.oid).into();
        if components.peek().is_none() {
            return Some(TreeFile {
                id: current,
                mode: entry.mode.value(),
            });
        }
        if !entry.mode.is_tree() {
            return None;
        }
    }
    None
}

fn collect_tree_files(
    repo: &gix::Repository,
    id: ObjectId,
    prefix: &str,
    files: &mut Map<String, TreeFile>,
) {
    let Ok(tree) = repo.find_tree(id) else {
        return;
    };
    let Ok(tree) = tree.decode() else {
        return;
    };
    for entry in tree.entries {
        let name = entry.filename.to_string();
        let path = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let child = (*entry.oid).into();
        if entry.mode.is_tree() {
            collect_tree_files(repo, child, &path, files);
        } else {
            files.insert(
                path,
                TreeFile {
                    id: child,
                    mode: entry.mode.value(),
                },
            );
        }
    }
}

fn render_file_diff(
    repo: &gix::Repository,
    path: &str,
    before: Option<&TreeFile>,
    after: Option<&TreeFile>,
    out: &mut String,
) {
    let old_path = format!("a/{path}");
    let new_path = format!("b/{path}");
    let binary = binary_change(repo, before, after);
    append(out, format_args!("diff --git {old_path} {new_path}\n"));
    match (before, after) {
        (None, Some(new)) => {
            append(out, format_args!("new file mode {:o}\n", new.mode));
            append_index_line(out, before, after);
            append_content_paths(out, binary, "/dev/null", &new_path);
        }
        (Some(old), None) => {
            append(out, format_args!("deleted file mode {:o}\n", old.mode));
            append_index_line(out, before, after);
            append_content_paths(out, binary, &old_path, "/dev/null");
        }
        (Some(old), Some(new)) => {
            let content_changed = old.id != new.id;
            if old.mode != new.mode {
                append(
                    out,
                    format_args!("old mode {:o}\nnew mode {:o}\n", old.mode, new.mode),
                );
            }
            if content_changed {
                append_index_line(out, before, after);
                append_content_paths(out, binary, &old_path, &new_path);
            }
        }
        (None, None) => {}
    }
    render_body(repo, before, after, path, out);
}

fn append_content_paths(out: &mut String, binary: bool, old_path: &str, new_path: &str) {
    if !binary {
        append(out, format_args!("--- {old_path}\n+++ {new_path}\n"));
    }
}

fn binary_change(
    repo: &gix::Repository,
    before: Option<&TreeFile>,
    after: Option<&TreeFile>,
) -> bool {
    [before, after].into_iter().flatten().any(|file| {
        repo.find_blob(file.id)
            .is_ok_and(|blob| blob.data.contains(&0))
    })
}

fn append_index_line(out: &mut String, before: Option<&TreeFile>, after: Option<&TreeFile>) {
    let old = before.map_or_else(|| "0000000".to_owned(), short_id);
    let new = after.map_or_else(|| "0000000".to_owned(), short_id);
    match (before, after) {
        (Some(old_file), Some(new_file)) if old_file.mode == new_file.mode => {
            append(
                out,
                format_args!("index {old}..{new} {:o}\n", old_file.mode),
            );
        }
        (Some(_), None | Some(_)) | (None, Some(_)) => {
            append(out, format_args!("index {old}..{new}\n"));
        }
        _ => {}
    }
}

fn short_id(file: &TreeFile) -> String {
    file.id.to_string().chars().take(7).collect()
}

fn render_body(
    repo: &gix::Repository,
    before: Option<&TreeFile>,
    after: Option<&TreeFile>,
    path: &str,
    out: &mut String,
) {
    let old_path = format!("a/{path}");
    let new_path = format!("b/{path}");
    let old_data = before
        .and_then(|file| repo.find_blob(file.id).ok())
        .map(|blob| blob.data.clone());
    let new_data = after
        .and_then(|file| repo.find_blob(file.id).ok())
        .map(|blob| blob.data.clone());
    if old_data.as_ref().is_some_and(|data| data.contains(&0))
        || new_data.as_ref().is_some_and(|data| data.contains(&0))
    {
        append(
            out,
            format_args!("Binary files {old_path} and {new_path} differ\n"),
        );
    } else {
        let old_data = old_data.unwrap_or_default();
        let new_data = new_data.unwrap_or_default();
        let input = gix::diff::blob::InternedInput::new(old_data.as_slice(), new_data.as_slice());
        let diff = gix::diff::blob::Diff::compute(gix::diff::blob::Algorithm::Myers, &input);
        let rendered = gix::diff::blob::UnifiedDiff::new(
            &diff,
            &input,
            gix::diff::blob::unified_diff::ConsumeBinaryHunk::new(Vec::new(), "\n"),
            gix::diff::blob::unified_diff::ContextSize::symmetrical(3),
        )
        .consume()
        .unwrap_or_default();
        let rendered = String::from_utf8_lossy(&rendered);
        out.push_str(&normalize_hunks(&rendered));
        insert_no_newline_marker(out, old_data.as_slice(), b'-');
        insert_no_newline_marker(out, new_data.as_slice(), b'+');
    }
}

fn normalize_hunks(rendered: &str) -> String {
    rendered
        .lines()
        .map(normalize_hunk)
        .collect::<Vec<_>>()
        .join("\n")
        + if rendered.ends_with('\n') { "\n" } else { "" }
}

fn normalize_hunk(line: &str) -> String {
    if !line.starts_with("@@ ") {
        return line.to_owned();
    }
    let mut parts = line.split_whitespace();
    let Some(prefix) = parts.next() else {
        return line.to_owned();
    };
    let Some(old) = parts.next() else {
        return line.to_owned();
    };
    let Some(new) = parts.next() else {
        return line.to_owned();
    };
    let Some(suffix) = parts.next() else {
        return line.to_owned();
    };
    format!(
        "{prefix} {} {} {suffix}",
        normalize_range(old),
        normalize_range(new)
    )
}

fn normalize_range(range: &str) -> String {
    let Some(sign @ ('-' | '+')) = range.chars().next() else {
        return range.to_owned();
    };
    let range = &range[sign.len_utf8()..];
    let Some((start, count)) = range.split_once(',') else {
        return range.to_owned();
    };
    match count.parse::<usize>() {
        Ok(0) => format!("{sign}0,0"),
        Ok(1) => format!("{sign}{start}"),
        Ok(_) | Err(_) => format!("{sign}{range}"),
    }
}

fn insert_no_newline_marker(out: &mut String, data: &[u8], prefix: u8) {
    if data.ends_with(b"\n") {
        return;
    }
    let data_text = String::from_utf8_lossy(data);
    let last_line = data_text.rsplit('\n').next().unwrap_or_default();
    let target = format!("{}{}", prefix as char, last_line);
    let mut lines: Vec<_> = out.split_inclusive('\n').map(str::to_owned).collect();
    let Some(index) = lines
        .iter()
        .rposition(|line| line.trim_end_matches('\n') == target)
    else {
        return;
    };
    lines.insert(index + 1, String::from("\\ No newline at end of file\n"));
    *out = lines.concat();
}

fn append(out: &mut String, args: std::fmt::Arguments<'_>) {
    if out.write_fmt(args).is_err() {}
}
