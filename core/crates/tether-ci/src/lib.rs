//! GitHub Actions workflow management, polling, and failure reports.

mod client;
mod logs;
mod poll;
mod workflow;

pub use client::{
    CiClient, CiError, DispatchInputs, JobRef, JobStep, RunRef, RunSummary, WorkflowMissing,
};
pub use logs::{CiFailure, TrimmedLog, parse_failures, trim_log};
pub use poll::{
    CiOutcome, CiProgress, CiRunState, JobProgress, PollConfig, Sleeper, poll_run, state_from_run,
};
pub use workflow::{
    WorkflowProposal, detect_setup, propose_workflow, workflow_is_current, workflow_template,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_uses_env_for_the_command() {
        let template = workflow_template();

        assert!(template.starts_with("# tether-workflow-version: 1\n"));
        assert!(template.contains("name: tether\n"));
        assert!(template.contains("workflow_dispatch:"));
        assert!(template.contains("TETHER_COMMAND: ${{ inputs.command }}"));
        assert!(
            !template
                .lines()
                .any(|line| line.trim_start().starts_with("run:") && line.contains("${{ inputs."))
        );
    }

    #[test]
    fn setup_detection_uses_python_then_node_then_rust_then_java() {
        assert_eq!(detect_setup(&["Cargo.toml", "package.json"]), Some("node"));
        assert_eq!(detect_setup(&["pom.xml", "build.gradle.kts"]), Some("java"));
        assert_eq!(detect_setup(&["requirements-dev.txt"]), Some("python"));
        assert_eq!(detect_setup(&["README.md"]), None);
        assert_eq!(detect_setup(&["nested/package.json"]), None);
    }

    #[test]
    fn proposal_has_full_new_file_diff_and_current_files_need_no_change() {
        let proposal = propose_workflow(None).expect("new workflow");

        assert_eq!(proposal.path, ".github/workflows/tether.yml");
        assert!(proposal.is_new);
        assert!(proposal.diff.contains("+name: tether"));
        assert!(propose_workflow(Some(workflow_template())).is_none());
        let stale = propose_workflow(Some("# tether-workflow-version: 1\nname: old\n"))
            .expect("stale workflow");
        assert!(!stale.is_new);
        assert_eq!(stale.content, workflow_template());
        assert!(stale.diff.contains("-name: old"));
    }

    #[test]
    fn state_mapping_matches_the_architecture_machine() {
        assert_eq!(state_from_run("queued", None), CiRunState::Queued);
        assert_eq!(state_from_run("in_progress", None), CiRunState::InProgress);
        assert_eq!(
            state_from_run("completed", Some("success")),
            CiRunState::Succeeded
        );
        assert_eq!(
            state_from_run("completed", Some("failure")),
            CiRunState::Failed
        );
        assert_eq!(
            state_from_run("completed", Some("cancelled")),
            CiRunState::Cancelled
        );
        assert_eq!(
            state_from_run("completed", Some("timed_out")),
            CiRunState::TimedOut
        );
    }

    #[test]
    fn parser_reads_pytest_javascript_cargo_gradle_and_rustc() {
        let log = "FAILED tests/test_api.py::test_auth - expected 200\n● suite › rejects\n    at rejects (src/api.test.ts:12:7)\n---- test_auth stdout ----\nthread 'test_auth' panicked at src/lib.rs:8:3:\nassertion failed\n e: file:///home/runner/work/app/app/src/Main.kt:4:2 bad type\nerror[E0001]: bad match\n --> src/main.rs:9:4";
        let failures = parse_failures(log, "/home/runner/work/app/app/");
        assert_eq!(failures.len(), 5);
        assert_eq!(failures[0].framework, "pytest");
        assert_eq!(failures[1].framework, "jest");
        assert_eq!(failures[2].framework, "cargo test");
        assert_eq!(failures[3].framework, "gradle");
        assert_eq!(failures[4].framework, "rustc");
        assert_eq!(failures[2].line, Some(8));
    }

    #[test]
    fn trim_log_keeps_error_tail_and_marks_omitted_lines() {
        let raw = (0..50)
            .map(|index| format!("line {index}"))
            .chain(["##[error] failed".into()])
            .collect::<Vec<_>>()
            .join("\n");
        let trimmed = trim_log(&raw, 80);

        assert!(trimmed.truncated);
        assert!(trimmed.text.contains("[... "));
        assert!(trimmed.text.contains("failed"));
    }
}
