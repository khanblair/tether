//! Fixture and panic-safety tests for the quality gate.

use std::path::Path;

use tether_quality::{Batch, FileChange, Language, ProjectManifest, Registry, Thresholds, parse};

#[test]
fn every_language_fixture_parses_without_errors() {
    let fixtures = [
        (Language::Python, "python/basic.py"),
        (Language::JavaScript, "javascript/basic.js"),
        (Language::TypeScript, "typescript/basic.ts"),
        (Language::Tsx, "tsx/basic.tsx"),
        (Language::Rust, "rust/basic.rs"),
        (Language::Go, "go/basic.go"),
        (Language::Kotlin, "kotlin/basic.kt"),
        (Language::Java, "java/basic.java"),
        (Language::Swift, "swift/basic.swift"),
        (Language::C, "c/basic.c"),
        (Language::Cpp, "cpp/basic.cpp"),
        (Language::Ruby, "ruby/basic.rb"),
        (Language::Php, "php/basic.php"),
        (Language::Html, "html/basic.html"),
        (Language::Css, "css/basic.css"),
        (Language::Json, "json/basic.json"),
        (Language::Yaml, "yaml/basic.yaml"),
        (Language::Toml, "toml/basic.toml"),
        (Language::Markdown, "markdown/basic.md"),
        (Language::Bash, "bash/basic.sh"),
    ];
    for (language, relative) in fixtures {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(relative);
        let source = std::fs::read_to_string(path).unwrap();
        let parsed = parse(language, &source).unwrap();
        assert!(!parsed.has_errors(), "fixture has errors: {relative}");
    }
}

#[test]
fn every_rule_handles_fixtures_and_unfriendly_text() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files = Vec::new();
    for entry in walkdir(&root) {
        let source = std::fs::read_to_string(&entry).unwrap();
        files.push(FileChange {
            path: entry.strip_prefix(&root).unwrap().to_path_buf(),
            old_text: Some(source.clone()),
            new_text: Some(source),
        });
    }
    files.push(FileChange {
        path: "bad.py".into(),
        old_text: None,
        new_text: Some("def broken(:\n\0".to_owned()),
    });
    let findings = Registry::default().run(
        &Batch { files },
        &Thresholds::default(),
        &ProjectManifest::default(),
    );
    assert!(findings.len() <= 200);
}

fn walkdir(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walkdir(&path));
        } else {
            files.push(path);
        }
    }
    files
}
