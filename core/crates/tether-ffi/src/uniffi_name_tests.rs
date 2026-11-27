use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_METHODS: [&str; 2] = ["pub fn close(", "pub fn destroy("];

#[test]
fn exported_impls_do_not_use_kotlin_generated_method_names() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rust_files(&source_root, &mut files);

    let violations = files
        .iter()
        .flat_map(|path| {
            let source = fs::read_to_string(path).unwrap();
            find_violations(path, &strip_comments(&source))
        })
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "UniFFI exported impls must not export Kotlin-generated methods: {}",
        violations.join(", ")
    );
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn find_violations(path: &Path, source: &str) -> Vec<String> {
    let path = path.display().to_string();
    exported_impl_blocks(source)
        .into_iter()
        .flat_map(|block| {
            FORBIDDEN_METHODS
                .into_iter()
                .filter(move |method| block.contains(method))
                .map(|method| format!("{path} contains {method}"))
        })
        .collect()
}

fn strip_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut block_depth = 0;
    for line in source.lines() {
        output.push_str(&strip_comment_line(line, &mut block_depth));
        output.push('\n');
    }
    output
}

#[expect(
    clippy::excessive_nesting,
    reason = "The comment scanner handles mutually exclusive parser states."
)]
fn strip_comment_line(line: &str, block_depth: &mut usize) -> String {
    let mut output = String::new();
    let mut remainder = line;
    loop {
        if *block_depth > 0 {
            let Some(end) = remainder.find("*/") else {
                return output;
            };
            *block_depth -= 1;
            remainder = &remainder[end + 2..];
            continue;
        }

        let block_start = remainder.find("/*");
        let line_start = remainder.find("//");
        let Some(start) = first_comment_start(block_start, line_start) else {
            output.push_str(remainder);
            return output;
        };
        output.push_str(&remainder[..start]);
        if line_start == Some(start) {
            return output;
        }
        *block_depth = 1;
        remainder = &remainder[start + 2..];
    }
}

fn first_comment_start(block_start: Option<usize>, line_start: Option<usize>) -> Option<usize> {
    match (block_start, line_start) {
        (Some(block), Some(line)) => Some(block.min(line)),
        (Some(block), None) => Some(block),
        (None, Some(line)) => Some(line),
        (None, None) => None,
    }
}

#[expect(
    clippy::excessive_nesting,
    reason = "The source scanner tracks export and brace states."
)]
fn exported_impl_blocks(source: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut pending_export = false;
    let mut depth = 0;
    let mut block = String::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if depth == 0 {
            if trimmed.starts_with("#[uniffi::export") {
                pending_export = true;
            } else if pending_export && is_impl_line(trimmed) {
                depth = brace_delta(line);
                block = line.to_owned();
                pending_export = false;
            } else if pending_export && !trimmed.is_empty() && !trimmed.starts_with('#') {
                pending_export = false;
            }
        } else {
            block.push('\n');
            block.push_str(line);
            depth += brace_delta(line);
            if depth == 0 {
                blocks.push(std::mem::take(&mut block));
            }
        }
    }

    blocks
}

fn is_impl_line(line: &str) -> bool {
    line.starts_with("impl ") || line.starts_with("impl<")
}

fn brace_delta(line: &str) -> i32 {
    line.bytes().fold(0, |depth, byte| match byte {
        b'{' => depth + 1,
        b'}' => depth - 1,
        _ => depth,
    })
}
