use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::FileIndex;
use crate::PathGuard;
use crate::ignore_rules::IgnoreRules;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("tether-ignore-{}", ulid::Ulid::new()));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn git(&self, args: &[&str]) -> std::process::ExitStatus {
        Command::new("git")
            .args(args)
            .current_dir(&self.0)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

const FILES: [&str; 31] = [
    "README.md",
    "src/main.rs",
    "debug.log",
    "keep.log",
    "rooted",
    "sub/rooted",
    "docs/a.tmp",
    "docs/deep/b.tmp",
    "src/cache/x.rs",
    "cache/y.rs",
    "out/z.txt",
    "out/keep.txt",
    "src/out/w.txt",
    "logs/a/b/c.txt",
    "logs/important.txt",
    "logs/other.md",
    "temp1.dat",
    "temp12.dat",
    "a.bak",
    "c.bak",
    "sub/file.gen",
    "sub/keep.gen",
    "sub/only-here",
    "sub/deeper/only-here",
    "sub/deeper/other.gen",
    "other/file.gen",
    "excluded.txt",
    "sub/excluded.txt",
    "space name/file with space.txt",
    "sub/keep.cfg",
    "sub/other.cfg",
];

const FOLDERS: [&str; 6] = ["out", "src/cache", "cache", "sub", "logs/a", "space name"];

fn git_fixture() -> Fixture {
    let fixture = Fixture::new();
    assert!(fixture.git(&["init", "-q"]).success());
    fixture.write(
        ".gitignore",
        "*.log\n!keep.log\n/rooted\ndocs/*.tmp\n**/cache/\nout/\n!out/keep.txt\n\
         logs/**/*.txt\n!/logs/important.txt\ntemp?.dat\n[ab].bak\n*.cfg\n",
    );
    fixture.write(
        "sub/.gitignore",
        "*.gen\n!keep.gen\n!keep.cfg\n/only-here\n",
    );
    fixture.write(".git/info/exclude", "excluded.txt\n");
    for file in FILES {
        fixture.write(file, "x\n");
    }
    fixture
}

fn git_says_ignored(fixture: &Fixture, path: &str) -> bool {
    let status = fixture.git(&["check-ignore", "-q", "--no-index", path]);
    match status.code() {
        Some(0) => true,
        Some(1) => false,
        other => panic!("git check-ignore failed for {path}: {other:?}"),
    }
}

fn walked(fixture: &Fixture) -> (IgnoreRules, BTreeSet<String>) {
    let mut rules = IgnoreRules::load(&fixture.0).unwrap();
    let walk = rules.walk(false).unwrap();
    let paths = walk
        .files
        .iter()
        .map(|file| file.relative.to_string_lossy().into_owned())
        .collect();
    (rules, paths)
}

#[test]
fn matches_git_check_ignore_for_every_fixture_path() {
    let fixture = git_fixture();
    let (rules, _) = walked(&fixture);
    let mut compared = 0;
    for file in FILES {
        assert_eq!(
            rules.is_ignored(Path::new(file), false),
            git_says_ignored(&fixture, file),
            "file {file}"
        );
        compared += 1;
    }
    for folder in FOLDERS {
        assert_eq!(
            rules.is_ignored(Path::new(folder), true),
            git_says_ignored(&fixture, folder),
            "folder {folder}"
        );
        compared += 1;
    }
    assert!(compared >= 25);
}

#[test]
fn a_walk_lists_exactly_the_files_git_does_not_ignore() {
    let fixture = git_fixture();
    let (_, listed) = walked(&fixture);
    let mut expected: BTreeSet<String> = FILES
        .iter()
        .filter(|file| !git_says_ignored(&fixture, file))
        .map(|file| (*file).to_owned())
        .collect();
    expected.insert(".gitignore".into());
    expected.insert("sub/.gitignore".into());
    assert_eq!(listed, expected);
}

#[test]
fn defaults_apply_without_a_git_repository_and_tetherignore_can_reinclude() {
    let fixture = Fixture::new();
    for file in [
        "node_modules/a.js",
        "src/node_modules/b.js",
        "target/debug/x",
        "dist/app.js",
        ".DS_Store",
        "docs/.DS_Store",
        "secret/key.txt",
        "src/main.rs",
    ] {
        fixture.write(file, "x\n");
    }
    fixture.write(".tetherignore", "!dist/\nsecret/\n");
    let (rules, listed) = walked(&fixture);
    let expected: BTreeSet<String> = [".tetherignore", "dist/app.js", "src/main.rs"]
        .iter()
        .map(|file| (*file).to_owned())
        .collect();
    assert_eq!(listed, expected);
    assert!(rules.is_ignored(Path::new("node_modules"), true));
    assert!(rules.is_ignored(Path::new("src/node_modules/b.js"), false));
    assert!(!rules.is_ignored(Path::new("dist"), true));
    assert!(rules.is_ignored(Path::new("secret/key.txt"), false));
}

#[test]
fn include_ignored_lists_everything_but_git_and_counts_symlinks() {
    let fixture = git_fixture();
    std::os::unix::fs::symlink(fixture.0.join("README.md"), fixture.0.join("link")).unwrap();
    let mut rules = IgnoreRules::load(&fixture.0).unwrap();
    let walk = rules.walk(true).unwrap();
    let listed: BTreeSet<String> = walk
        .files
        .iter()
        .map(|file| file.relative.to_string_lossy().into_owned())
        .collect();
    assert!(listed.contains("debug.log"));
    assert!(listed.contains("out/z.txt"));
    assert!(!listed.iter().any(|path| path.starts_with(".git/")));
    assert_eq!(walk.skipped_symlinks, 1);
}

#[test]
fn symlinked_rule_files_are_never_read() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    outside.write("rules", "secret.txt\n");
    fixture.write("secret.txt", "x\n");
    fixture.write("sub/secret.txt", "x\n");
    std::os::unix::fs::symlink(outside.0.join("rules"), fixture.0.join(".tetherignore")).unwrap();
    std::os::unix::fs::symlink(outside.0.join("rules"), fixture.0.join("sub/.gitignore")).unwrap();
    let (_, listed) = walked(&fixture);
    let expected: BTreeSet<String> = ["secret.txt", "sub/secret.txt"]
        .iter()
        .map(|file| (*file).to_owned())
        .collect();
    assert_eq!(listed, expected);
}

#[test]
fn the_file_index_uses_the_same_rules_and_hides_nested_git_folders() {
    let fixture = git_fixture();
    fixture.write("vendor/repo/.git/config", "x\n");
    let index = FileIndex::scan(PathGuard::new(&fixture.0).unwrap()).unwrap();
    assert!(index.is_ignored(Path::new("out")));
    assert!(index.is_ignored(Path::new("vendor/repo/.git/config")));
    assert!(!index.is_ignored(Path::new("sub")));
    assert!(!index.is_ignored(Path::new("sub/keep.gen")));
    assert!(index.is_ignored(Path::new("sub/file.gen")));
    assert!(
        index
            .files()
            .iter()
            .all(|file| !file.path.starts_with("out"))
    );
}
