use std::fs;
use std::sync::Arc;

use tempfile::TempDir;
use tether_engine::commands::CommandRegistry;

use crate::{
    CoreEventView, EventSink, SecretVault, SecretsPlatform, SecretsPlatformFailure, SessionRuntime,
};

struct PlainPlatform;

impl SecretsPlatform for PlainPlatform {
    fn wrap(
        &self,
        plaintext: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, SecretsPlatformFailure> {
        Ok(plaintext)
    }

    fn unwrap(
        &self,
        wrapped: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, SecretsPlatformFailure> {
        Ok(wrapped)
    }
}

struct NoopSink;

impl EventSink for NoopSink {
    fn on_event(&self, _event: CoreEventView) {}
}

fn fixture() -> (TempDir, Arc<SessionRuntime>) {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("tether.db");
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime =
        SessionRuntime::new(database.to_str().unwrap(), vault, Arc::new(NoopSink)).unwrap();
    (dir, runtime)
}

fn write_skill(root: &std::path::Path, name: &str, description: &str) {
    write_skill_source(
        root,
        name,
        &format!("---\nname: {name}\ndescription: {description}\n---\nInstructions."),
    );
}

fn write_skill_source(root: &std::path::Path, name: &str, source: &str) {
    let directory = root.join("skills").join(name);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("SKILL.md"), source).unwrap();
}

#[test]
fn command_sheet_lists_skills_after_builtins_in_name_order() {
    let (dir, runtime) = fixture();
    write_skill(dir.path(), "zeta", "Zeta skill");
    write_skill(dir.path(), "alpha", "Alpha skill");

    let sheet = runtime.command_sheet();
    let builtin_count = CommandRegistry::builtin().commands().len();
    assert_eq!(
        sheet[..builtin_count]
            .iter()
            .map(|command| command.name.as_str())
            .collect::<Vec<_>>(),
        CommandRegistry::builtin()
            .commands()
            .iter()
            .map(|command| command.name.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        sheet[builtin_count..]
            .iter()
            .map(|command| command.name.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "zeta"]
    );
    assert!(sheet[builtin_count..].iter().all(|command| {
        command.available && command.args_hint.is_none() && command.requires.is_none()
    }));
}

#[test]
fn resolving_a_skill_uses_the_skill_kind_and_prompt() {
    let (dir, runtime) = fixture();
    write_skill(dir.path(), "tdd", "Test first");

    let resolved = runtime.resolve_command("/tdd fix the bug").unwrap();
    assert_eq!(resolved.name, "tdd");
    assert_eq!(resolved.arguments, "fix the bug");
    assert_eq!(resolved.kind, "skill");
    assert_eq!(
        resolved.prompt.as_deref(),
        Some(
            "Use the \"tdd\" skill for this request. Load it first with skill_view, then follow its instructions. fix the bug"
        )
    );

    let without_arguments = runtime.resolve_command("/tdd").unwrap();
    assert_eq!(without_arguments.kind, "skill");
    assert_eq!(
        without_arguments.prompt.as_deref(),
        Some(
            "Use the \"tdd\" skill for this request. Load it first with skill_view, then follow its instructions."
        )
    );
}

#[test]
fn disabled_skills_are_not_commands() {
    let (dir, runtime) = fixture();
    write_skill(dir.path(), "disabled", "Do not show me");
    runtime.set_skill_enabled("disabled", false).unwrap();

    assert!(
        !runtime
            .command_sheet()
            .iter()
            .any(|command| command.name == "disabled")
    );
    assert!(runtime.resolve_command("/disabled").is_err());
}

#[test]
fn builtins_win_and_long_skill_names_are_skipped() {
    let (dir, runtime) = fixture();
    write_skill(dir.path(), "help", "Not the built-in");
    let long_name = "a".repeat(41);
    write_skill(dir.path(), &long_name, "Too long");

    let help = runtime.resolve_command("/help").unwrap();
    assert_eq!(help.kind, "open");
    assert_eq!(help.prompt, None);
    assert!(
        !runtime
            .command_sheet()
            .iter()
            .any(|command| command.name == long_name)
    );
}

#[test]
fn skill_descriptions_are_single_line_and_bounded() {
    let (dir, runtime) = fixture();
    write_skill(dir.path(), "long", &"x".repeat(300));
    write_skill_source(
        dir.path(),
        "folded",
        "---\nname: folded\ndescription: >\n  First line.\n  Second line.\n---\nInstructions.",
    );

    let sheet = runtime.command_sheet();
    let long = sheet.iter().find(|command| command.name == "long").unwrap();
    assert_eq!(long.description.chars().count(), 100);
    assert!(long.description.ends_with('…'));
    assert!(!long.description.contains('\n'));

    let folded = sheet
        .iter()
        .find(|command| command.name == "folded")
        .unwrap();
    assert_eq!(folded.description, "First line. Second line.");
}
