use super::*;
use std::fs;
use std::path::Path;
use tether_providers::ToolSpec;
use tether_types::{Executor, PermissionMode};

fn inputs<'a>(tools: &'a [ToolSpec], today: &'a str) -> PromptInputs<'a> {
    static EXECUTORS: [Executor; 1] = [Executor::Sandbox];

    PromptInputs {
        project_name: "Demo",
        executors: &EXECUTORS,
        branch: Some("main"),
        mode: PermissionMode::Ask,
        tool_specs: tools,
        global_instructions: "global",
        project_instructions: "project",
        memory_block: "memory",
        vault_block: "vault",
        skills_index: Some("skills"),
        today,
    }
}

fn spec(name: &str) -> ToolSpec {
    ToolSpec {
        name: name.into(),
        description: "test".into(),
        input_schema_json: "{}".into(),
    }
}

#[test]
fn prompt_sections_keep_the_fixed_order_and_date_is_last() {
    let tools = [spec("run")];
    let prompt = PromptBuilder::build(&inputs(&tools, "2026-11-21"));
    let names: Vec<_> = prompt
        .sections
        .iter()
        .map(|section| section.name.as_str())
        .collect();

    assert_eq!(
        names,
        [
            "Base",
            "Mode note",
            "Environment",
            "Instructions",
            "Memory",
            "Vault slice",
            "Skills index",
            "Date",
        ]
    );
    assert!(prompt.text.ends_with("Today's date: 2026-11-21"));
    assert!(prompt.text.contains("Global instructions:\nglobal"));
    assert!(prompt.text.contains("Project instructions:\nproject"));
}

#[test]
fn base_names_only_tools_present_in_the_registry() {
    let without_tools = PromptBuilder::build(&inputs(&[], "today"));
    assert!(!without_tools.text.contains("Use run"));
    assert!(!without_tools.text.contains("todo_write"));
    assert!(!without_tools.text.contains("ask_user"));
    assert!(!without_tools.text.contains("submit_plan"));

    let tools = [
        spec("run"),
        spec("todo_write"),
        spec("ask_user"),
        spec("submit_plan"),
    ];
    let with_tools = PromptBuilder::build(&PromptInputs {
        mode: PermissionMode::Plan,
        ..inputs(&tools, "today")
    });
    assert!(with_tools.text.contains("Use run"));
    assert!(with_tools.text.contains("todo_write"));
    assert!(with_tools.text.contains("ask_user"));
    assert!(with_tools.text.contains("submit_plan"));
    assert!(!with_tools.text.contains("Bypass"));
}

#[test]
fn base_mentions_github_api_only_when_it_is_available() {
    let without_tool = PromptBuilder::build(&inputs(&[], "today"));
    assert!(!without_tool.text.contains("Use github_api"));
    assert!(!without_tool.text.contains("sign in under Settings, GitHub"));

    let tools = [spec("github_api")];
    let with_tool = PromptBuilder::build(&inputs(&tools, "today"));
    assert!(with_tool.text.contains(
        "Use github_api for anything on GitHub. It acts as the account the user signed in with in Settings, so never ask for a token."
    ));
    assert!(with_tool
        .text
        .contains("If github_api says GitHub is not signed in, tell the user to sign in under Settings, GitHub."));
}

#[test]
fn changing_only_today_changes_only_the_last_line() {
    let tools: [ToolSpec; 0] = [];
    let first = PromptBuilder::build(&inputs(&tools, "first"));
    let second = PromptBuilder::build(&inputs(&tools, "second"));
    let first_prefix = first.text.rsplit_once('\n').map_or("", |value| value.0);
    let second_prefix = second.text.rsplit_once('\n').map_or("", |value| value.0);

    assert_eq!(first_prefix, second_prefix);
    assert_ne!(first.text, second.text);
}

#[test]
fn instruction_precedence_truncation_and_safe_skips_are_visible() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("AGENTS.md"), "agents").unwrap();
    fs::write(dir.path().join("CLAUDE.md"), "claude").unwrap();
    assert_eq!(
        load_instructions(None, Some(dir.path())).project,
        "project instruction AGENTS.md\n<instructions>\nagents\n</instructions>"
    );

    fs::write(dir.path().join("TETHER.md"), "line\n".repeat(20_000)).unwrap();
    let project = load_instructions(None, Some(dir.path())).project;
    assert!(project.contains("[truncated]"));
    assert!(project.len() < 33 * 1024);

    let target = dir.path().join("TETHER.md");
    fs::remove_file(&target).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.path().join("CLAUDE.md"), &target).unwrap();
    #[cfg(unix)]
    assert!(
        load_instructions(None, Some(dir.path()))
            .project
            .contains("symlinked")
    );

    fs::remove_file(&target).unwrap();
    fs::write(&target, [0xff, 0xfe]).unwrap();
    assert!(
        load_instructions(None, Some(dir.path()))
            .project
            .contains("non-UTF-8")
    );
}

#[test]
fn project_instruction_preview_uses_the_prompt_file_order() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("AGENTS.md"), "one\ntwo").unwrap();
    let ProjectInstructionRead::Found(instructions) = load_project_instructions(dir.path()) else {
        panic!("expected the AGENTS.md file to be found");
    };
    assert_eq!(instructions.file_name, "AGENTS.md");
    assert_eq!(instructions.text.lines().count(), 2);
}

#[test]
fn project_instruction_preview_prefers_tether_and_handles_missing_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("TETHER.md"), "tether").unwrap();
    fs::write(dir.path().join("CLAUDE.md"), "claude").unwrap();
    let ProjectInstructionRead::Found(instructions) = load_project_instructions(dir.path()) else {
        panic!("expected the TETHER.md file to be found");
    };
    assert_eq!(instructions.file_name, "TETHER.md");
    let empty = tempfile::tempdir().unwrap();
    assert_eq!(
        load_project_instructions(empty.path()),
        ProjectInstructionRead::Missing
    );
}

#[test]
fn project_instruction_preview_reports_a_missing_root_as_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-such-root");
    assert_eq!(
        load_project_instructions(&missing),
        ProjectInstructionRead::Skipped {
            file_name: String::new(),
            note: "[project root is unavailable]".to_owned(),
        }
    );
}

#[test]
fn project_instruction_preview_counts_the_capped_text() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("AGENTS.md"), "line\n".repeat(20_000)).unwrap();
    let ProjectInstructionRead::Found(instructions) = load_project_instructions(dir.path()) else {
        panic!("expected the AGENTS.md file to be found");
    };
    assert!(instructions.text.lines().count() < 20_000);
}

#[cfg(unix)]
#[test]
fn project_instruction_preview_skips_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("AGENTS.md"), "outside").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("AGENTS.md"),
        dir.path().join("AGENTS.md"),
    )
    .unwrap();
    let ProjectInstructionRead::Skipped { file_name, note } = load_project_instructions(dir.path())
    else {
        panic!("expected the symlinked file to be skipped");
    };
    assert_eq!(file_name, "AGENTS.md");
    assert!(note.contains("symlinked"));
}

#[test]
fn project_instruction_preview_skips_non_utf8_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("AGENTS.md"), [0xff, 0xfe]).unwrap();
    let ProjectInstructionRead::Skipped { file_name, note } = load_project_instructions(dir.path())
    else {
        panic!("expected the non-UTF-8 file to be skipped");
    };
    assert_eq!(file_name, "AGENTS.md");
    assert_eq!(
        note,
        "[project instruction AGENTS.md were skipped: non-UTF-8 file]"
    );
}

#[test]
fn vault_slice_finds_map_by_alias_and_caps_the_combined_text() {
    let dir = tempfile::tempdir().unwrap();
    let store = tether_memory::vault::VaultStore::new(dir.path());
    store
        .write("index.md", "---\ndescription: Index\n---\nindex\n")
        .unwrap();
    store
        .write("log.md", "---\ndescription: Log\n---\nold\nnew\n")
        .unwrap();
    store
        .write(
            "Map.md",
            "---\ntitle: Work map\ndescription: Map\ntype: moc\naliases: [Demo]\n---\nmap\n",
        )
        .unwrap();
    let slice = load_vault_slice(dir.path(), "Demo");
    assert!(slice.contains("Active project map note"));
    assert!(slice.contains("Work map"));

    let long = "x".repeat(100_000);
    store
        .write("index.md", &format!("---\ndescription: Index\n---\n{long}"))
        .unwrap();
    let slice = load_vault_slice(dir.path(), "Demo");
    assert!(slice.chars().count() <= 6_000);
    assert_eq!(load_vault_slice(Path::new("/missing/vault"), "Demo"), "");
}
