//! P0-10: the lint fails on a fixture with banned words and passes on a clean one.

use std::process::Command;

fn run(fixture: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_slop-lint"))
        .arg(format!(
            "{}/tests/fixtures/{fixture}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .output()
        .expect("slop-lint runs")
}

#[test]
fn fails_on_banned_words() {
    let out = run("bad.md");
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("banned-word:leverage"));
    assert!(stdout.contains("exclamation-mark"));
}

#[test]
fn passes_on_clean_text() {
    assert!(run("good.md").status.success());
}

#[test]
fn fails_on_a_path_that_does_not_exist() {
    let out = Command::new(env!("CARGO_BIN_EXE_slop-lint"))
        .arg("no/such/path.md")
        .output()
        .expect("slop-lint runs");
    assert_eq!(out.status.code(), Some(2));
}
