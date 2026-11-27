use std::collections::VecDeque;
use std::future::Future;
use std::io::Write;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use tether_secrets::Secret;
use tether_vcs::{HttpClient, HttpClientError, HttpRequest, HttpResponse};

struct FixedIdentity;

impl GitIdentityReader for FixedIdentity {
    fn identity(&self) -> Result<Option<Identity>, String> {
        Ok(Some(Identity {
            name: "Stored Name".into(),
            email: "stored@example.test".into(),
        }))
    }
}

struct FindingGate(Vec<super::super::GateFinding>);

impl super::super::CommitGate for FindingGate {
    fn check(&self, _input: &super::super::CommitInput) -> Vec<super::super::GateFinding> {
        self.0.clone()
    }
}

struct ReceivePackTransport {
    bare: PathBuf,
}

impl Transport for ReceivePackTransport {
    fn get(
        &self,
        _url: &str,
        _headers: BTreeMap<String, String>,
    ) -> Result<Vec<u8>, TransportError> {
        git_receive_pack(&self.bare, &["--stateless-rpc", "--advertise-refs"], None)
    }

    fn post(
        &self,
        _url: &str,
        _content_type: &str,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, TransportError> {
        git_receive_pack(&self.bare, &["--stateless-rpc"], Some(body))
    }
}

fn git_receive_pack(
    bare: &Path,
    args: &[&str],
    input: Option<Vec<u8>>,
) -> Result<Vec<u8>, TransportError> {
    let mut child = Command::new("git")
        .arg("receive-pack")
        .args(args)
        .arg(bare)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| TransportError)?;
    if let Some(input) = input {
        child
            .stdin
            .take()
            .ok_or(TransportError)?
            .write_all(&input)
            .map_err(|_| TransportError)?;
    }
    let output = child.wait_with_output().map_err(|_| TransportError)?;
    output
        .status
        .success()
        .then_some(output.stdout)
        .ok_or(TransportError)
}

struct ReceivePackFactory(PathBuf);

impl TransportFactory for ReceivePackFactory {
    fn transport(&self) -> Result<Box<dyn Transport>, String> {
        Ok(Box::new(ReceivePackTransport {
            bare: self.0.clone(),
        }))
    }
}

struct AcceptingPush {
    posts: Arc<AtomicUsize>,
}

impl Transport for AcceptingPush {
    fn get(
        &self,
        _url: &str,
        _headers: BTreeMap<String, String>,
    ) -> Result<Vec<u8>, TransportError> {
        let mut response = packet("# service=git-receive-pack\n");
        response.extend(b"0000");
        response.extend(packet(&format!(
            "{} refs/heads/main\0report-status side-band-64k\n",
            "0".repeat(40)
        )));
        response.extend(b"0000");
        Ok(response)
    }

    fn post(
        &self,
        _url: &str,
        _content_type: &str,
        _body: Vec<u8>,
    ) -> Result<Vec<u8>, TransportError> {
        self.posts.fetch_add(1, Ordering::Relaxed);
        let mut response = packet("unpack ok\n");
        response.extend(packet("ok refs/heads/main\n"));
        response.extend(b"0000");
        Ok(response)
    }
}

struct AcceptingPushFactory(Arc<AtomicUsize>);

impl TransportFactory for AcceptingPushFactory {
    fn transport(&self) -> Result<Box<dyn Transport>, String> {
        Ok(Box::new(AcceptingPush {
            posts: Arc::clone(&self.0),
        }))
    }
}

fn packet(payload: &str) -> Vec<u8> {
    format!("{:04x}{payload}", payload.len() + 4).into_bytes()
}

struct NoRemote;
impl GitRemoteWriter for NoRemote {
    fn set_remote(&self, _: &str, _: &str) -> Result<(), String> {
        Ok(())
    }
}

struct RecordingRemote {
    calls: Mutex<Vec<(String, String)>>,
}

impl RecordingRemote {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl GitRemoteWriter for RecordingRemote {
    fn set_remote(&self, remote_url: &str, default_branch: &str) -> Result<(), String> {
        self.calls
            .lock()
            .map_err(|_| "remote recording lock is unavailable".to_owned())?
            .push((remote_url.to_owned(), default_branch.to_owned()));
        Ok(())
    }
}

struct GithubApi {
    responses: Mutex<VecDeque<Result<HttpResponse, HttpClientError>>>,
}

impl GithubApi {
    fn new(responses: Vec<HttpResponse>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into_iter().map(Ok).collect()),
        })
    }
}

impl HttpClient for GithubApi {
    fn send(
        &self,
        _request: HttpRequest,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpClientError>> + Send + '_>> {
        let response = self
            .responses
            .lock()
            .map_err(|_| HttpClientError)
            .and_then(|mut responses| responses.pop_front().ok_or(HttpClientError));
        Box::pin(async move { response? })
    }
}

struct GithubFactory(Arc<GithubApi>);

impl GithubClientFactory for GithubFactory {
    fn client(&self) -> Result<GithubClient, String> {
        let http: Arc<dyn HttpClient> = self.0.clone();
        GithubClient::with_base_url(
            http,
            Secret::from_text("test-token"),
            "https://api.github.test",
        )
        .map_err(|error| error.to_string())
    }
}

fn response(body: &str) -> HttpResponse {
    HttpResponse {
        status: 201,
        headers: BTreeMap::default(),
        body: body.as_bytes().to_vec(),
    }
}

async fn push_call(repo: &Repo, bare: &Path, args: &str) -> Result<String, String> {
    let handler = GitTools::new(GitToolsConfig {
        remote_url: Some("https://github.test/acme/repo.git".into()),
        default_branch: Some("main".into()),
        identity: Arc::new(NoIdentity),
        transport: Arc::new(ReceivePackFactory(bare.to_owned())),
        github: Arc::new(NoGithub),
        github_owner: None,
        remote_writer: Arc::new(NoRemote),
        gates: Vec::new(),
    });
    let guard = PathGuard::new(repo.path()).unwrap();
    let call = handler.prepare("git_push", args, &PrepareContext { guard: &guard })?;
    let session = SessionId::new("push-test");
    let call_id = ToolCallId::new("push-call");
    let mut before_edit = |_: &[PathBuf]| Ok(());
    handler
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
        .await
        .map(|output| output.text)
}

fn local_call(
    repo: &Repo,
    tool: &str,
    args: &str,
    identity: Arc<dyn GitIdentityReader>,
    gates: Vec<Arc<dyn super::super::CommitGate>>,
) -> Result<String, String> {
    let call = prepare(repo, tool, args)?;
    let deps = super::super::local::LocalDeps {
        identity,
        gates,
        cancel: CancellationToken::new(),
    };
    super::super::local::execute(repo.path(), tool, &call.payload, &deps)
}

#[test]
fn read_tools_return_compact_exact_fixture_text() {
    let repo = Repo::on_branch("main");
    repo.git(&["config", "user.name", "T"]);
    repo.git(&["config", "user.email", "t@example.test"]);
    std::fs::write(repo.path().join("tracked.txt"), "one\n").unwrap();
    repo.git(&["add", "tracked.txt"]);
    repo.git(&["commit", "-q", "-m", "base"]);
    std::fs::write(repo.path().join("tracked.txt"), "two\n").unwrap();
    repo.git(&["add", "tracked.txt"]);
    std::fs::write(repo.path().join("tracked.txt"), "three\n").unwrap();
    std::fs::write(repo.path().join("untracked.txt"), "new\n").unwrap();

    assert_eq!(
        local_call(&repo, "git_status", "{}", Arc::new(NoIdentity), Vec::new()).unwrap(),
        "MM tracked.txt\n ? untracked.txt"
    );
    assert_eq!(
        local_call(
            &repo,
            "git_diff",
            r#"{"staged":true}"#,
            Arc::new(NoIdentity),
            Vec::new()
        )
        .unwrap(),
        "diff --git a/tracked.txt b/tracked.txt\nindex 5626abf..f719efd 100644\n--- a/tracked.txt\n+++ b/tracked.txt\n@@ -1 +1 @@\n-one\n+two\n"
    );
    assert!(
        local_call(&repo, "git_diff", "{}", Arc::new(NoIdentity), Vec::new())
            .unwrap()
            .contains("-two\n+three")
    );
    let log = local_call(
        &repo,
        "git_log",
        r#"{"limit":1}"#,
        Arc::new(NoIdentity),
        Vec::new(),
    )
    .unwrap();
    let head = repo.output(&["rev-parse", "HEAD"]);
    assert_eq!(log, format!("{} base", head.trim()));
}

#[test]
fn commit_uses_stored_identity_and_reports_gate_severities_exactly() {
    let repo = Repo::on_branch("main");
    std::fs::write(repo.path().join("file.txt"), "content\n").unwrap();
    let warn = Arc::new(FindingGate(vec![super::super::GateFinding {
        severity: super::super::GateSeverity::Warn,
        rule_id: "quality.warn".into(),
        message: "review this change".into(),
    }]));
    let result = local_call(
        &repo,
        "git_commit",
        r#"{"message":"Add file"}"#,
        Arc::new(FixedIdentity),
        vec![warn],
    )
    .unwrap();
    let head = repo.output(&["log", "-1", "--format=%an,%ae,%cn,%ce"]);
    assert_eq!(
        head.trim(),
        "Stored Name,stored@example.test,Stored Name,stored@example.test"
    );
    assert_eq!(result.lines().nth(1), Some("Warnings:"));
    assert_eq!(
        result.lines().nth(2),
        Some("quality.warn: review this change")
    );
}

#[test]
fn block_gate_stops_before_commit_and_info_does_not_change_success() {
    let repo = Repo::on_branch("main");
    std::fs::write(repo.path().join("file.txt"), "content\n").unwrap();
    let block = Arc::new(FindingGate(vec![
        super::super::GateFinding {
            severity: super::super::GateSeverity::Info,
            rule_id: "quality.info".into(),
            message: "informational".into(),
        },
        super::super::GateFinding {
            severity: super::super::GateSeverity::Block,
            rule_id: "quality.block".into(),
            message: "do not commit".into(),
        },
    ]));
    assert_eq!(
        local_call(
            &repo,
            "git_commit",
            r#"{"message":"Add file"}"#,
            Arc::new(FixedIdentity),
            vec![block],
        )
        .unwrap_err(),
        "Fix these before committing:\nquality.block: do not commit"
    );
    assert_eq!(repo.output(&["rev-list", "--count", "HEAD"]).trim(), "1");
}

#[test]
fn commit_requires_identity_and_rejects_empty_changes_and_messages() {
    let repo = Repo::on_branch("main");
    assert_eq!(
        local_call(
            &repo,
            "git_commit",
            r#"{"message":"Add file"}"#,
            Arc::new(NoIdentity),
            Vec::new()
        )
        .unwrap_err(),
        "Sign in to GitHub or set a name and email in Settings first"
    );
    assert_eq!(
        prepare(&repo, "git_commit", r#"{"message":"  "}"#).unwrap_err(),
        "commit message is empty"
    );
    assert_eq!(
        local_call(
            &repo,
            "git_commit",
            r#"{"message":"Add file"}"#,
            Arc::new(FixedIdentity),
            Vec::new()
        )
        .unwrap_err(),
        "nothing is staged to commit"
    );
    let branch = local_call(
        &repo,
        "git_branch",
        r#"{"switch":false}"#,
        Arc::new(NoIdentity),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(branch, "Created branch tether/work from HEAD");
}

#[test]
fn default_branch_push_never_skips_approver_but_bypass_allows_it() {
    let rule = PermissionRule {
        id: "stored".into(),
        tool: "git_push".into(),
        pattern: "*".into(),
        executor: None,
    };
    let request = crate::permissions::PermissionRequest {
        tool: "git_push",
        target: "push to the default branch main on origin",
        executor: None,
    };
    for mode in [
        PermissionMode::Ask,
        PermissionMode::AcceptEdits,
        PermissionMode::Auto,
    ] {
        assert_eq!(
            permission(mode, crate::permissions::Effect::Push, &rule, request),
            crate::permissions::Decision::Ask
        );
    }
    assert_eq!(
        permission(
            PermissionMode::Bypass,
            crate::permissions::Effect::Push,
            &rule,
            request
        ),
        crate::permissions::Decision::Allow
    );
    assert!(matches!(
        permission(
            PermissionMode::Plan,
            crate::permissions::Effect::Push,
            &rule,
            request
        ),
        crate::permissions::Decision::Block { .. }
    ));
}

#[test]
fn repository_creation_asks_in_every_mode_except_bypass() {
    let repo = Repo::on_branch("main");
    let guard = PathGuard::new(repo.path()).unwrap();
    let call = super::tools("main")
        .prepare(
            "github_repo_create",
            r#"{"name":"new-repo","private":false}"#,
            &PrepareContext { guard: &guard },
        )
        .unwrap();
    assert_eq!(
        call.target,
        "create GitHub repository blair/new-repo (public)"
    );
    let rule = PermissionRule {
        id: "stored".into(),
        tool: "github_repo_create".into(),
        pattern: "*".into(),
        executor: None,
    };
    let request = crate::permissions::PermissionRequest {
        tool: "github_repo_create",
        target: &call.target,
        executor: None,
    };
    for mode in [
        PermissionMode::Ask,
        PermissionMode::AcceptEdits,
        PermissionMode::Auto,
    ] {
        assert_eq!(
            permission(mode, call.effect, &rule, request),
            crate::permissions::Decision::Ask
        );
    }
    assert_eq!(
        permission(PermissionMode::Bypass, call.effect, &rule, request),
        crate::permissions::Decision::Allow
    );
    assert!(matches!(
        permission(PermissionMode::Plan, call.effect, &rule, request),
        crate::permissions::Decision::Block { .. }
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_project_can_init_commit_create_push_and_open_pr() {
    let workspace = tempfile::tempdir().unwrap();
    let push_posts = Arc::new(AtomicUsize::new(0));
    let api = GithubApi::new(vec![
        response(
            r#"{"clone_url":"https://github.com/blair/new-repo.git","html_url":"https://github.com/blair/new-repo","default_branch":"main"}"#,
        ),
        response(
            r#"{"number":7,"html_url":"https://github.com/blair/new-repo/pull/7","title":"Add note","body":"","state":"open","head":{"sha":"head-sha","ref":"main"},"base":{"ref":"main"}}"#,
        ),
    ]);
    let remote_writer = Arc::new(RecordingRemote::new());
    let handler = GitTools::new(GitToolsConfig {
        remote_url: None,
        default_branch: None,
        identity: Arc::new(FixedIdentity),
        transport: Arc::new(AcceptingPushFactory(Arc::clone(&push_posts))),
        github: Arc::new(GithubFactory(Arc::clone(&api))),
        github_owner: Some("blair".into()),
        remote_writer: Arc::clone(&remote_writer) as Arc<dyn GitRemoteWriter>,
        gates: Vec::new(),
    });
    std::fs::write(workspace.path().join("note.txt"), "hello\n").unwrap();
    for (tool, args) in [
        ("git_init", r"{}"),
        ("git_commit", r#"{"message":"Add note"}"#),
        (
            "github_repo_create",
            r#"{"name":"new-repo","private":true,"description":"A test repo"}"#,
        ),
        ("git_push", r"{}"),
        ("pr_create", r#"{"title":"Add note"}"#),
    ] {
        let guard = PathGuard::new(workspace.path()).unwrap();
        let call = handler
            .prepare(tool, args, &PrepareContext { guard: &guard })
            .unwrap();
        execute_call(&handler, workspace.path(), call)
            .await
            .unwrap_or_else(|error| panic!("{tool}: {error}"));
    }
    assert_eq!(
        remote_writer.calls.lock().unwrap().as_slice(),
        [(
            "https://github.com/blair/new-repo.git".into(),
            "main".into()
        )]
    );
    assert!(workspace.path().join(".git/config").exists());
    assert_eq!(push_posts.load(Ordering::Relaxed), 1);
}

#[test]
fn force_push_always_reaches_approver_even_with_a_stored_rule() {
    let rule = PermissionRule {
        id: "stored".into(),
        tool: "git_push".into(),
        pattern: "*".into(),
        executor: None,
    };
    let force_request = crate::permissions::PermissionRequest {
        tool: "git_push",
        target: "FORCE push main to origin (rewrites history)",
        executor: None,
    };
    for mode in [
        PermissionMode::Ask,
        PermissionMode::AcceptEdits,
        PermissionMode::Auto,
        PermissionMode::Bypass,
    ] {
        assert_eq!(
            permission(
                mode,
                crate::permissions::Effect::ForcePush,
                &rule,
                force_request
            ),
            crate::permissions::Decision::Ask
        );
    }
}

fn permission(
    mode: PermissionMode,
    effect: crate::permissions::Effect,
    rule: &PermissionRule,
    request: crate::permissions::PermissionRequest<'_>,
) -> crate::permissions::Decision {
    crate::permissions::decide(mode, effect, std::slice::from_ref(rule), request)
}

#[tokio::test(flavor = "multi_thread")]
async fn push_test_double_covers_new_fast_forward_non_fast_forward_force_and_delete() {
    let root = tempfile::tempdir().unwrap();
    let bare = root.path().join("remote.git");
    std::fs::create_dir_all(&bare).unwrap();
    Command::new("git")
        .args(["init", "--bare", "-q"])
        .current_dir(&bare)
        .status()
        .unwrap();
    let repo = Repo::on_branch("feature/work");
    std::fs::write(repo.path().join("one.txt"), "one\n").unwrap();
    repo.git(&["add", "one.txt"]);
    repo.git(&["commit", "-q", "-m", "one"]);
    assert_eq!(
        push_call(&repo, &bare, "{}").await.unwrap(),
        "refs/heads/feature/work: accepted"
    );
    std::fs::write(repo.path().join("two.txt"), "two\n").unwrap();
    repo.git(&["add", "two.txt"]);
    repo.git(&["commit", "-q", "-m", "two"]);
    assert_eq!(
        push_call(&repo, &bare, "{}").await.unwrap(),
        "refs/heads/feature/work: accepted"
    );
    let ancestor = repo.output(&["rev-parse", "HEAD~1"]);
    repo.git(&["reset", "--hard", ancestor.trim()]);
    std::fs::write(repo.path().join("diverged.txt"), "diverged\n").unwrap();
    repo.git(&["add", "diverged.txt"]);
    repo.git(&["commit", "-q", "-m", "diverged"]);
    assert_eq!(
        push_call(&repo, &bare, "{}").await.unwrap_err(),
        "push is not a fast-forward"
    );
    assert_eq!(
        push_call(&repo, &bare, r#"{"force":true}"#).await.unwrap(),
        "refs/heads/feature/work: accepted"
    );
    assert_eq!(
        push_call(&repo, &bare, r#"{"delete":true}"#).await.unwrap(),
        "refs/heads/feature/work: accepted"
    );
    assert!(
        !Command::new("git")
            .args(["--git-dir"])
            .arg(&bare)
            .args(["show-ref", "--verify", "refs/heads/feature/work"])
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn pull_request_status_caps_comments_and_maps_missing_head() {
    let pull = pr(4, "feature/work", "main");
    let comments = (0..31)
        .map(|index| PullRequestComment {
            author: format!("user{index}"),
            body: "x".repeat(501),
            path: Some("src/lib.rs".into()),
            line: Some(9),
        })
        .collect();
    let text = super::super::remote::test_status_text(
        &pull,
        vec![CheckRun {
            name: "build".into(),
            status: "completed".into(),
            conclusion: Some("success".into()),
            details_url: None,
        }],
        comments,
    );
    assert!(text.starts_with("PR #4: \ncheck build: completed (success)\ncomment user0: "));
    assert_eq!(text.matches("comment ").count(), 30);
    assert_eq!(
        text.lines().nth(2).unwrap(),
        format!("comment user0: {} at src/lib.rs:9", "x".repeat(500))
    );
    assert_eq!(
        super::super::remote::test_create_error(GithubError::Validation {
            messages: vec!["head branch does not exist".into()],
        }),
        "Push the branch first"
    );
    assert_eq!(
        super::super::remote::test_create_error(GithubError::BadCredentials),
        "GitHub credentials were rejected"
    );
    assert_eq!(
        super::super::remote::test_create_error(GithubError::RateLimited { reset_at: 42 }),
        "GitHub rate limit exceeded until 42"
    );
    assert_eq!(
        super::super::remote::test_create_error(GithubError::Validation {
            messages: vec!["title is too short".into()],
        }),
        "GitHub request was invalid: [\"title is too short\"]"
    );
}
