use std::collections::BTreeSet;

use tether_types::Executor;

use super::*;

fn availability(programs: &[&str], linux_ready: bool, ci_ready: bool) -> Availability {
    Availability {
        sandbox_programs: programs.iter().map(|program| (*program).into()).collect(),
        linux_ready,
        ci_ready,
    }
}

fn input(command: &str, availability: Availability) -> RouteInput {
    RouteInput {
        command: command.into(),
        requested: None,
        project_default: None,
        availability,
    }
}

#[test]
fn tokenizer_supports_quotes_and_escapes() {
    assert_eq!(
        tokenize(r#"python -c "print(1)""#),
        Ok(vec!["python", "-c", "print(1)"]
            .into_iter()
            .map(String::from)
            .collect())
    );
    assert_eq!(tokenize("echo a | wc"), Err(ShellFeature::Pipe));
    assert_eq!(
        tokenize("echo 'a | b'"),
        Ok(vec!["echo".into(), "a | b".into()])
    );
    assert_eq!(tokenize("echo a && echo b"), Err(ShellFeature::And));
    assert_eq!(tokenize("echo a || echo b"), Err(ShellFeature::Or));
    assert_eq!(tokenize("echo a; echo b"), Err(ShellFeature::Semicolon));
    assert_eq!(tokenize("echo a &"), Err(ShellFeature::Background));
    assert_eq!(tokenize("(echo a)"), Err(ShellFeature::Subshell));
    assert_eq!(tokenize("echo *.rs"), Err(ShellFeature::Glob));
    assert_eq!(tokenize("echo $HOME"), Err(ShellFeature::Variable));
    assert_eq!(
        tokenize("FOO=bar echo"),
        Err(ShellFeature::EnvironmentAssignment)
    );
    assert_eq!(tokenize("cat <<EOF"), Err(ShellFeature::HereDoc));
}

#[test]
fn router_prefers_sandbox_and_maps_pytest() {
    let route = Router::route(&input(
        "pytest -q",
        availability(&["python", "pytest"], false, true),
    ))
    .unwrap();
    assert_eq!(route.executor, Executor::Sandbox);
    assert_eq!(
        route.argv,
        Some(
            vec!["python", "-m", "pytest", "-q"]
                .into_iter()
                .map(String::from)
                .collect()
        )
    );
    assert_eq!(
        route.reason,
        "pytest is provided by the installed runtime pack"
    );
}

#[test]
fn router_uses_linux_then_ci_for_shell_features() {
    let route = Router::route(&input("echo a | wc", availability(&[], true, true))).unwrap();
    assert_eq!(route.executor, Executor::Linux);
    assert_eq!(
        route.reason,
        "uses a pipe, which the sandbox cannot run, so Linux mode is used"
    );

    let route = Router::route(&input("echo a | wc", availability(&[], false, true))).unwrap();
    assert_eq!(route.executor, Executor::Ci);
    assert_eq!(
        route.reason,
        "uses a pipe, Linux mode is not set up, so CI is used"
    );
}

#[test]
fn router_keeps_toolchains_in_linux_mode_even_when_named_by_a_pack() {
    let route = Router::route(&input("npm test", availability(&["npm"], true, true))).unwrap();
    assert_eq!(route.executor, Executor::Linux);
    assert_eq!(
        route.reason,
        "npm requires Linux mode, so Linux mode is used"
    );
}

#[test]
fn router_reports_missing_executor_and_explicit_unavailability() {
    let error = Router::route(&input("npm test", availability(&[], false, false))).unwrap_err();
    assert_eq!(
        error.to_string(),
        "no executor is available for npm test; set up Linux mode or CI"
    );

    let mut request = input("python", availability(&["python"], false, false));
    request.requested = Some(Executor::Linux);
    let error = Router::route(&request).unwrap_err();
    assert_eq!(
        error.to_string(),
        "requested Linux executor is unavailable: Linux mode is not set up"
    );
}

#[test]
fn router_rejects_git_as_a_run_target() {
    let error = Router::route(&input("git status", availability(&[], true, true))).unwrap_err();
    assert_eq!(error, RouteError::GitTools);
}

#[test]
fn router_accepts_project_default_when_available() {
    let mut request = input("python -c print", availability(&["python"], true, true));
    request.project_default = Some(Executor::Linux);
    let route = Router::route(&request).unwrap();
    assert_eq!(route.executor, Executor::Linux);
    assert_eq!(route.reason, "uses the project default executor");
}

#[test]
fn tokenizer_rejects_newline_and_unterminated_quote() {
    assert_eq!(tokenize("echo\nnext"), Err(ShellFeature::InvalidSyntax));
    assert_eq!(
        tokenize("echo 'unfinished"),
        Err(ShellFeature::InvalidSyntax)
    );
}

#[test]
fn router_rejects_invalid_shell_syntax_for_explicit_linux() {
    let mut request = input("echo\nnext", availability(&[], true, false));
    request.requested = Some(Executor::Linux);
    assert_eq!(Router::route(&request), Err(RouteError::InvalidCommand));
}

#[test]
fn tokenizer_keeps_empty_quoted_arguments() {
    assert_eq!(
        tokenize("echo \"\""),
        Ok(vec!["echo".into(), String::new()])
    );
}

#[test]
fn explicit_sandbox_requires_the_requested_program() {
    let mut request = input("sqlite3 app.db", availability(&["python"], true, true));
    request.requested = Some(Executor::Sandbox);
    let error = Router::route(&request).unwrap_err();
    assert_eq!(
        error.to_string(),
        "requested Sandbox executor is unavailable: no verified runtime pack provides sqlite3"
    );
}

#[test]
fn availability_keeps_program_names_sorted() {
    let value = availability(&["node", "python"], false, false);
    assert_eq!(
        value.sandbox_programs,
        BTreeSet::from(["node".into(), "python".into()])
    );
}
