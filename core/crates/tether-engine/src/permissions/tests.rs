use super::*;
use tether_types::{Executor, PermissionMode, PermissionRule};

#[test]
fn every_mode_table_cell_has_the_exact_decision() {
    let cells = table_cells();
    assert_eq!(cells.len(), 35);
    for (mode, effect, expected) in cells {
        assert_eq!(
            decide(mode, effect, &[], request("tool", "target")),
            expected
        );
    }
}

fn table_cells() -> Vec<(PermissionMode, Effect, Decision)> {
    plan_cells()
        .into_iter()
        .chain(ask_cells())
        .chain(accept_edits_cells())
        .chain(auto_cells())
        .chain(bypass_cells())
        .collect()
}

fn plan_cells() -> [(PermissionMode, Effect, Decision); 7] {
    [
        (PermissionMode::Plan, Effect::Read, Decision::Allow),
        (
            PermissionMode::Plan,
            Effect::Edit,
            block(PermissionMode::Plan, "file edits"),
        ),
        (
            PermissionMode::Plan,
            Effect::RunSandbox,
            block(PermissionMode::Plan, "sandbox runs"),
        ),
        (
            PermissionMode::Plan,
            Effect::RunHost,
            block(PermissionMode::Plan, "host runs"),
        ),
        (PermissionMode::Plan, Effect::Network, Decision::Ask),
        (
            PermissionMode::Plan,
            Effect::Push,
            block(PermissionMode::Plan, "pushes"),
        ),
        (
            PermissionMode::Plan,
            Effect::ForcePush,
            block(PermissionMode::Plan, "force pushes"),
        ),
    ]
}

fn ask_cells() -> [(PermissionMode, Effect, Decision); 7] {
    [
        (PermissionMode::Ask, Effect::Read, Decision::Allow),
        (PermissionMode::Ask, Effect::Edit, Decision::Ask),
        (PermissionMode::Ask, Effect::RunSandbox, Decision::Ask),
        (PermissionMode::Ask, Effect::RunHost, Decision::Ask),
        (PermissionMode::Ask, Effect::Network, Decision::Ask),
        (PermissionMode::Ask, Effect::Push, Decision::Ask),
        (PermissionMode::Ask, Effect::ForcePush, Decision::Ask),
    ]
}

fn accept_edits_cells() -> [(PermissionMode, Effect, Decision); 7] {
    [
        (PermissionMode::AcceptEdits, Effect::Read, Decision::Allow),
        (PermissionMode::AcceptEdits, Effect::Edit, Decision::Allow),
        (
            PermissionMode::AcceptEdits,
            Effect::RunSandbox,
            Decision::Ask,
        ),
        (PermissionMode::AcceptEdits, Effect::RunHost, Decision::Ask),
        (PermissionMode::AcceptEdits, Effect::Network, Decision::Ask),
        (PermissionMode::AcceptEdits, Effect::Push, Decision::Ask),
        (
            PermissionMode::AcceptEdits,
            Effect::ForcePush,
            Decision::Ask,
        ),
    ]
}

fn auto_cells() -> [(PermissionMode, Effect, Decision); 7] {
    [
        (PermissionMode::Auto, Effect::Read, Decision::Allow),
        (PermissionMode::Auto, Effect::Edit, Decision::Allow),
        (PermissionMode::Auto, Effect::RunSandbox, Decision::Allow),
        (PermissionMode::Auto, Effect::RunHost, Decision::Ask),
        (PermissionMode::Auto, Effect::Network, Decision::Ask),
        (PermissionMode::Auto, Effect::Push, Decision::Ask),
        (PermissionMode::Auto, Effect::ForcePush, Decision::Ask),
    ]
}

fn bypass_cells() -> [(PermissionMode, Effect, Decision); 7] {
    [
        (PermissionMode::Bypass, Effect::Read, Decision::Allow),
        (PermissionMode::Bypass, Effect::Edit, Decision::Allow),
        (PermissionMode::Bypass, Effect::RunSandbox, Decision::Allow),
        (PermissionMode::Bypass, Effect::RunHost, Decision::Allow),
        (PermissionMode::Bypass, Effect::Network, Decision::Allow),
        (PermissionMode::Bypass, Effect::Push, Decision::Allow),
        (PermissionMode::Bypass, Effect::ForcePush, Decision::Ask),
    ]
}

#[test]
fn internal_and_unknown_are_mode_independent() {
    for mode in [
        PermissionMode::Plan,
        PermissionMode::Ask,
        PermissionMode::AcceptEdits,
        PermissionMode::Auto,
        PermissionMode::Bypass,
    ] {
        assert_eq!(
            decide(mode, Effect::Internal, &[], request("todo", "")),
            Decision::Allow
        );
        assert_eq!(
            decide(mode, Effect::Unknown, &[], request("nope", "")),
            Decision::Block {
                reason: format!("{} mode blocks unknown tools", mode_name(mode))
            }
        );
    }
}

#[test]
fn rules_only_relax_matching_ask_results() {
    let rules = [PermissionRule {
        id: "r1".into(),
        tool: "write_file".into(),
        pattern: "src/*".into(),
        executor: None,
    }];
    assert_eq!(
        decide(
            PermissionMode::Ask,
            Effect::Edit,
            &rules,
            request("write_file", "src/lib.rs")
        ),
        Decision::Allow
    );
    assert_eq!(
        decide(
            PermissionMode::Ask,
            Effect::Edit,
            &rules,
            request("edit_file", "src/lib.rs")
        ),
        Decision::Ask
    );
    assert_eq!(
        decide(
            PermissionMode::Ask,
            Effect::Edit,
            &rules,
            request("write_file", "test.rs")
        ),
        Decision::Ask
    );
    assert!(matches!(
        decide(
            PermissionMode::Plan,
            Effect::Edit,
            &rules,
            request("write_file", "src/lib.rs")
        ),
        Decision::Block { .. }
    ));
    assert_eq!(
        decide(
            PermissionMode::Ask,
            Effect::Push,
            &rules,
            request("write_file", "src/lib.rs")
        ),
        Decision::Ask
    );
}

#[test]
fn run_rules_use_only_a_clean_first_program_token() {
    let rule = PermissionRule {
        id: "r1".into(),
        tool: "run".into(),
        pattern: "pytest".into(),
        executor: Some(Executor::Sandbox),
    };
    assert_eq!(
        decide(
            PermissionMode::Ask,
            Effect::RunSandbox,
            std::slice::from_ref(&rule),
            request_with_executor("run", "pytest -q tests", Some(Executor::Sandbox))
        ),
        Decision::Allow
    );
    for command in [
        "pytest && curl x",
        "pytest; ls",
        "pytest $(id)",
        "pytest > out",
        "pytest\\x",
        "pytest\nls",
        "pytester",
    ] {
        assert_eq!(
            decide(
                PermissionMode::Ask,
                Effect::RunSandbox,
                std::slice::from_ref(&rule),
                request_with_executor("run", command, Some(Executor::Sandbox))
            ),
            Decision::Ask,
            "{command:?}"
        );
    }
}

#[test]
fn rule_for_returns_only_safe_exact_patterns() {
    assert_eq!(
        rule_for(Effect::Edit, "write_file", "src/main.rs", None),
        Some(NewRule {
            tool: "write_file".into(),
            pattern: "src/main.rs".into(),
            executor: None,
        })
    );
    assert_eq!(
        rule_for(
            Effect::RunSandbox,
            "run",
            "pytest -q tests",
            Some(Executor::Sandbox)
        ),
        Some(NewRule {
            tool: "run".into(),
            pattern: "pytest".into(),
            executor: Some(Executor::Sandbox),
        })
    );
    assert_eq!(
        rule_for(
            Effect::RunSandbox,
            "run",
            "pytest && curl x",
            Some(Executor::Sandbox)
        ),
        None
    );
    assert_eq!(rule_for(Effect::Edit, "move_file", "a -> b", None), None);
}

fn request<'a>(tool: &'a str, target: &'a str) -> PermissionRequest<'a> {
    request_with_executor(tool, target, None)
}

fn request_with_executor<'a>(
    tool: &'a str,
    target: &'a str,
    executor: Option<Executor>,
) -> PermissionRequest<'a> {
    PermissionRequest {
        tool,
        target,
        executor,
    }
}
