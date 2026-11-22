use std::sync::Arc;

use tether_workspace::PathGuard;

use super::RunTools;
use super::run_support::{append_user_args, trim_output};
use crate::executors::NotConfigured;
use crate::permissions::rule_for;
use crate::tool_loop::{PrepareContext, ToolHandler};

#[test]
fn output_cut_keeps_head_tail_and_exact_omitted_count() {
    let output = format!(
        "{}{}{}",
        "h".repeat(4_000),
        "x".repeat(100_000),
        "t".repeat(12_000)
    );
    let (trimmed, omitted) = trim_output(&output);

    assert_eq!(omitted, 100_000);
    assert!(trimmed.starts_with(&"h".repeat(4_000)));
    assert!(trimmed.contains("[... 100000 characters omitted ...]"));
    assert!(trimmed.ends_with(&"t".repeat(12_000)));
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn user_arguments_follow_the_pack_prefix_without_losing_any() {
    let prefix = strings(&["python", "-I"]);
    let typed = strings(&["python", "-m", "pytest", "-q"]);
    assert_eq!(
        append_user_args(prefix, &typed),
        strings(&["python", "-I", "-m", "pytest", "-q"])
    );
}

#[test]
fn a_pack_without_a_prefix_keeps_the_command_as_typed() {
    let typed = strings(&["ls", "-la"]);
    assert_eq!(append_user_args(strings(&["ls"]), &typed), typed);
    assert_eq!(
        append_user_args(strings(&["ls", "--color"]), &strings(&["ls"])),
        strings(&["ls", "--color"])
    );
}

#[test]
fn a_run_target_is_the_bare_command_so_a_saved_rule_names_the_program() {
    let root = std::env::temp_dir().join(format!("tether-run-prepare-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let guard = PathGuard::new(&root).unwrap();
    let tools = RunTools::new(root.join("packs")).unwrap().with_executors(
        None,
        Arc::new(NotConfigured),
        None,
        Arc::new(|_| false),
    );
    let call = tools
        .prepare(
            "run",
            r#"{"command":"echo hello","executor":"ci"}"#,
            &PrepareContext { guard: &guard },
        )
        .unwrap();
    assert_eq!(call.target, "echo hello");
    assert_eq!(call.display_target(), "[ci] echo hello");
    let rule = rule_for(call.effect, &call.tool, &call.target, call.executor).unwrap();
    assert_eq!(rule.pattern, "echo");
    std::fs::remove_dir_all(root).unwrap();
}
