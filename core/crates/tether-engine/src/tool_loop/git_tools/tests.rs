use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use tether_types::{SessionId, ToolCallId};
use tether_vcs::{GithubClient, Identity, PullRequestInfo, Transport, TransportError};
use tether_workspace::PathGuard;
use tokio_util::sync::CancellationToken;

use super::remote::find_pr_number;
use super::{
    GitIdentityReader, GitTools, GitToolsConfig, GithubClientFactory, TransportFactory, cap_diff,
};
use crate::tool_loop::{ExecuteContext, PrepareContext, ToolHandler};

struct Repo(PathBuf);

impl Repo {
    fn on_branch(branch: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "tether-git-tools-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            branch.replace('/', "-")
        ));
        drop(std::fs::remove_dir_all(&root));
        std::fs::create_dir_all(&root).unwrap();
        let repo = Self(root);
        repo.git(&["init", "-q", "-b", branch]);
        repo.git(&["commit", "-q", "--allow-empty", "-m", "start"]);
        repo
    }

    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .args(["-c", "user.name=T", "-c", "user.email=t@example.test"])
            .args(args)
            .current_dir(&self.0)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.0));
    }
}

struct NoIdentity;
impl GitIdentityReader for NoIdentity {
    fn identity(&self) -> Result<Option<Identity>, String> {
        Ok(None)
    }
}

struct NoGithub;
impl GithubClientFactory for NoGithub {
    fn client(&self) -> Result<GithubClient, String> {
        Err("no client in tests".into())
    }
}

/// Fails like the production transport does when it is used on an async thread: it starts
/// its own runtime, which panics inside a running one.
struct NestedRuntimeTransport;
impl Transport for NestedRuntimeTransport {
    fn get(&self, _: &str, _: BTreeMap<String, String>) -> Result<Vec<u8>, TransportError> {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .map_err(|_| TransportError)?
            .block_on(async {});
        Err(TransportError)
    }

    fn post(&self, _: &str, _: &str, _: Vec<u8>) -> Result<Vec<u8>, TransportError> {
        Err(TransportError)
    }
}

struct NestedFactory;
impl TransportFactory for NestedFactory {
    fn transport(&self) -> Result<Box<dyn Transport>, String> {
        Ok(Box::new(NestedRuntimeTransport))
    }
}

fn tools(default_branch: &str) -> GitTools {
    GitTools::new(GitToolsConfig {
        remote_url: Some("https://github.com/acme/repo.git".into()),
        default_branch: Some(default_branch.into()),
        identity: Arc::new(NoIdentity),
        transport: Arc::new(NestedFactory),
        github: Arc::new(NoGithub),
        gates: Vec::new(),
    })
}

fn prepare(repo: &Repo, tool: &str, args: &str) -> Result<crate::tool_loop::PreparedCall, String> {
    let guard = PathGuard::new(repo.path()).unwrap();
    tools("main").prepare(tool, args, &PrepareContext { guard: &guard })
}

#[test]
fn a_push_without_a_branch_names_and_freezes_the_checked_out_branch() {
    let repo = Repo::on_branch("feature/work");
    let call = prepare(&repo, "git_push", "{}").unwrap();
    assert_eq!(call.target, "push feature/work to origin");
    assert_eq!(call.payload["branch"], "feature/work");
    let delete = prepare(&repo, "git_push", r#"{"delete":true}"#).unwrap();
    assert_eq!(delete.target, "delete branch feature/work on origin");
    assert_eq!(delete.payload["branch"], "feature/work");
    let force = prepare(&repo, "git_push", r#"{"force":true}"#).unwrap();
    assert!(force.target.starts_with("FORCE push feature/work"));
}

#[test]
fn a_push_on_the_default_branch_says_so_and_an_explicit_branch_is_kept() {
    let repo = Repo::on_branch("main");
    let call = prepare(&repo, "git_push", "{}").unwrap();
    assert_eq!(call.target, "push to the default branch main on origin");
    let other = prepare(&repo, "git_push", r#"{"branch":"topic"}"#).unwrap();
    assert_eq!(other.target, "push topic to origin");
    assert_eq!(other.payload["branch"], "topic");
}

#[test]
fn a_pull_request_names_the_head_branch_it_will_use() {
    let repo = Repo::on_branch("feature/work");
    let call = prepare(&repo, "pr_create", r#"{"title":"Add it"}"#).unwrap();
    assert_eq!(
        call.target,
        "open pull request \"Add it\" from feature/work into main"
    );
    assert_eq!(call.payload["head"], "feature/work");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_push_runs_off_the_async_threads_so_a_blocking_transport_cannot_panic() {
    let repo = Repo::on_branch("feature/work");
    let guard = PathGuard::new(repo.path()).unwrap();
    let handler = tools("main");
    let call = handler
        .prepare("git_push", "{}", &PrepareContext { guard: &guard })
        .unwrap();
    let session = SessionId::new("s".to_owned());
    let call_id = ToolCallId::new("c");
    let mut before_edit = |_: &[PathBuf]| Ok(());
    let result = handler
        .execute(
            call,
            ExecuteContext {
                session_id: &session,
                tool_call_id: &call_id,
                cancel: CancellationToken::new(),
                sink: None,
                guard: &guard,
                before_edit: &mut before_edit,
            },
        )
        .await;
    // The double answers with a transport error; reaching it without a panic is the point.
    assert!(result.is_err());
}

fn pr(number: u64, head_ref: &str, base: &str) -> PullRequestInfo {
    PullRequestInfo {
        number,
        url: String::new(),
        title: String::new(),
        body: String::new(),
        state: "open".into(),
        head_sha: "abc123".into(),
        head_ref: head_ref.into(),
        base: base.into(),
    }
}

#[test]
fn the_current_branch_finds_its_pull_request_by_head_branch_only() {
    let open = [pr(1, "other", "main"), pr(2, "feature/work", "main")];
    assert_eq!(find_pr_number(&open, "feature/work"), Ok(2));
    // Being the base of a PR, or matching a head SHA, is not the same as owning one.
    assert!(find_pr_number(&open, "main").is_err());
    assert!(find_pr_number(&open, "abc123").is_err());
}

#[test]
fn a_long_diff_is_cut_on_a_character_boundary() {
    for offset in 0..4 {
        let text = format!("{}é{}", "a".repeat(64 * 1024 - offset), "b".repeat(100));
        let capped = cap_diff(text);
        assert!(capped.ends_with("[diff truncated at 64 KiB]\n"));
    }
    assert_eq!(cap_diff("short".into()), "short");
}

#[test]
fn optional_text_arguments_that_are_supplied_validate() {
    let repo = Repo::on_branch("main");
    for (tool, args) in [
        ("git_diff", r#"{"path":"src/a.rs"}"#),
        ("git_log", r#"{"path":"src/a.rs","limit":5}"#),
        ("git_branch", r#"{"name":"topic","from":"main"}"#),
        (
            "pr_create",
            r#"{"title":"T","body":"B","base":"main","draft":true}"#,
        ),
    ] {
        assert!(prepare(&repo, tool, args).is_ok(), "{tool} {args}");
    }
    for (tool, args) in [
        ("git_diff", r#"{"path":3}"#),
        ("git_push", r#"{"branch":false}"#),
        ("pr_create", r#"{"title":"T","base":5}"#),
    ] {
        assert!(prepare(&repo, tool, args).is_err(), "{tool} {args}");
    }
}
