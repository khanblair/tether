//! slop-lint: checks docs, UI strings and commit messages for banned words and AI slop.
//!
//! Usage: `slop-lint <path>...` where each path is a file or a directory. Directories are
//! searched for `.md` and `.xml` files. Exits 1 when any finding exists.

mod rules;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rules::Rules;

fn main() -> ExitCode {
    let paths: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if paths.is_empty() {
        eprintln!("usage: slop-lint <file-or-directory>...");
        return ExitCode::from(2);
    }

    if let Some(missing) = paths.iter().find(|p| !p.exists()) {
        eprintln!("slop-lint: path does not exist: {}", missing.display());
        return ExitCode::from(2);
    }

    let rules = Rules::load();
    let mut total = 0;
    for file in paths.iter().flat_map(|p| collect(p)) {
        total += report(&rules, &file);
    }

    if total > 0 {
        eprintln!("slop-lint: {total} finding(s)");
        ExitCode::FAILURE
    } else {
        println!("slop-lint: clean");
        ExitCode::SUCCESS
    }
}

fn collect(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .flat_map(|p| collect(&p))
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("md" | "xml")))
        .collect();
    files.sort();
    files
}

fn report(rules: &Rules, file: &Path) -> usize {
    let Ok(text) = fs::read_to_string(file) else {
        eprintln!("{}: cannot read as text", file.display());
        return 1;
    };
    let is_xml = file.extension().is_some_and(|e| e == "xml");
    let findings = rules.check(&text, is_xml);
    for f in &findings {
        println!("{}:{}: {}: {}", file.display(), f.line, f.rule, f.advice);
    }
    findings.len()
}
