use std::collections::{BTreeMap, VecDeque};
use std::future::{Future, ready};
use std::path::Path;
use std::pin::Pin;
use std::process::Command;
use std::sync::{Arc, Mutex, Weak};

use tether_store::Store;
use tether_vcs::{HttpClient, HttpClientError, HttpRequest, HttpResponse};

use super::super::SignInStateView;
use super::super::{
    CloneProgressView, GitDiffSpecView, GitRuntime, GitRuntimeError, GitStatusView,
    GithubStatusView, SecretVault, SecretsPlatform,
};

struct PlainPlatform;

impl SecretsPlatform for PlainPlatform {
    fn wrap(
        &self,
        plaintext: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, super::super::SecretsPlatformFailure> {
        Ok(plaintext)
    }

    fn unwrap(
        &self,
        wrapped: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, super::super::SecretsPlatformFailure> {
        Ok(wrapped)
    }
}

struct FakeHttp {
    responses: Mutex<VecDeque<HttpResponse>>,
}

struct Progress {
    phases: Mutex<Vec<String>>,
    runtime: Mutex<Option<Weak<GitRuntime>>>,
    cancel: bool,
}

impl CloneProgressView for Progress {
    fn on_progress(&self, phase: String, _received: u64, _total: Option<u64>) {
        self.phases.lock().expect("phase lock").push(phase.clone());
        if self.cancel
            && phase == "Receiving"
            && let Some(runtime) = self.runtime.lock().expect("runtime lock").as_ref()
            && let Some(runtime) = runtime.upgrade()
        {
            runtime.cancel_clone();
        }
    }
}

impl HttpClient for FakeHttp {
    fn send(
        &self,
        _request: HttpRequest,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpClientError>> + Send + '_>> {
        Box::pin(ready(
            self.responses
                .lock()
                .expect("response lock")
                .pop_front()
                .ok_or(HttpClientError),
        ))
    }
}

fn response(body: &str) -> HttpResponse {
    HttpResponse {
        status: 200,
        headers: BTreeMap::new(),
        body: body.as_bytes().to_vec(),
    }
}

fn scoped_user() -> HttpResponse {
    let mut value =
        response(r#"{"login":"blair","id":7,"name":"Blair","email":"blair@example.test"}"#);
    value
        .headers
        .insert("x-oauth-scopes".into(), "repo, read:user".into());
    value
}

fn runtime(responses: impl IntoIterator<Item = HttpResponse>) -> Arc<GitRuntime> {
    let dir = tempfile::tempdir().expect("fixture directory");
    let database = dir.path().join("tether.db");
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = GitRuntime::new(
        database.to_str().expect("database path"),
        dir.path()
            .join("workspaces")
            .to_str()
            .expect("workspace path"),
        vault,
    )
    .expect("runtime");
    std::mem::forget(dir);
    runtime.set_http_for_tests(Arc::new(FakeHttp {
        responses: Mutex::new(responses.into_iter().collect()),
    }));
    runtime
}

fn project_runtime() -> (
    tempfile::TempDir,
    Arc<GitRuntime>,
    String,
    std::path::PathBuf,
) {
    let dir = tempfile::tempdir().expect("fixture directory");
    let database = dir.path().join("tether.db");
    let workspace = dir.path().join("workspaces");
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = GitRuntime::new(
        database.to_str().expect("database path"),
        workspace.to_str().expect("workspace path"),
        vault,
    )
    .expect("runtime");
    let mut store = Store::open(&database).expect("store");
    let project = store.create_project("fixture", 1).expect("project");
    let root = workspace.join(&project.path);
    std::fs::create_dir_all(&root).expect("workspace");
    (dir, runtime, project.id.to_string(), root)
}

#[test]
fn manual_identity_validation_rejects_empty_newline_and_angle_brackets() {
    let runtime = runtime([]);
    for (name, email) in [
        ("", "a@example.test"),
        ("A\nB", "a@example.test"),
        ("A", "a>b@example.test"),
    ] {
        assert!(matches!(
            runtime.set_git_identity(name.into(), email.into()),
            Err(GitRuntimeError::InvalidInput)
        ));
    }
    runtime
        .set_git_identity("A Name".into(), "a@example.test".into())
        .expect("identity");
}

#[test]
fn a_good_token_records_scopes_and_sign_out_clears_token_and_identity() {
    let runtime = runtime([scoped_user(), scoped_user()]);
    runtime
        .github_sign_in_with_token("recognizable-fake-token")
        .expect("sign in");
    assert_eq!(
        runtime.github_status().expect("status"),
        GithubStatusView {
            signed_in: true,
            login: Some("blair".into()),
            name: Some("Blair".into()),
            email: Some("blair@example.test".into()),
            scopes: vec!["repo".into(), "read:user".into()],
        }
    );
    runtime.github_sign_out().expect("sign out");
    assert!(!runtime.github_status().expect("signed out").signed_in);
    assert_eq!(runtime.github_status().expect("signed out").login, None);
    assert_eq!(
        runtime.github_status().expect("signed out").scopes,
        Vec::<String>::new()
    );
}

#[test]
fn an_under_scoped_token_names_the_missing_scope_and_is_not_saved() {
    let runtime = runtime([response(
        r#"{"login":"blair","id":7,"name":"Blair","email":"blair@example.test"}"#,
    )]);
    assert!(matches!(
        runtime.github_sign_in_with_token("token"),
        Err(GitRuntimeError::MissingScope { scope }) if scope == "repo"
    ));
    assert!(!runtime.github_status().expect("status").signed_in);
}

#[test]
fn repo_state_calls_are_bounded() {
    let (_dir, runtime, project_id, root) = project_runtime();
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["config", "user.name", "T"]);
    git(&root, &["config", "user.email", "t@example.test"]);
    assert_repo_state(&runtime, &project_id, &root);
}

fn assert_repo_state(runtime: &GitRuntime, project_id: &str, root: &Path) {
    std::fs::write(root.join("tracked.txt"), "one\n").expect("tracked");
    git(root, &["add", "tracked.txt"]);
    git(root, &["commit", "-q", "-m", "base"]);
    std::fs::write(root.join("tracked.txt"), "two\n").expect("staged");
    git(root, &["add", "tracked.txt"]);
    std::fs::write(root.join("tracked.txt"), "three\n").expect("worktree");
    std::fs::write(root.join("untracked.txt"), "new\n").expect("untracked");
    assert_eq!(
        runtime.git_status(project_id).expect("status"),
        vec![
            GitStatusView {
                path: "tracked.txt".into(),
                index_state: "Modified".into(),
                worktree_state: "Modified".into(),
            },
            GitStatusView {
                path: "untracked.txt".into(),
                index_state: String::new(),
                worktree_state: "Untracked".into(),
            },
        ]
    );
    let diff = runtime
        .git_diff(
            project_id,
            GitDiffSpecView {
                staged: true,
                path: Some("tracked.txt".into()),
                from: None,
                to: None,
            },
        )
        .expect("diff");
    assert!(diff.contains("-one\n+two"));
    assert!(!diff.contains("untracked.txt"));
    assert_eq!(
        runtime.git_log(project_id, 1).expect("log")[0].subject,
        "base"
    );
    assert_eq!(
        runtime.current_branch(project_id).expect("branch"),
        Some("main".into())
    );
    assert_eq!(
        runtime.git_branches(project_id).expect("branches")[0].name,
        "main"
    );
}

#[test]
fn git_init_uses_the_requested_branch() {
    let (_dir, runtime, project_id, root) = project_runtime();
    runtime.git_init(&project_id, "develop").expect("init");
    assert!(root.join(".git").exists());
    assert_eq!(
        runtime.current_branch(&project_id).expect("branch"),
        Some("develop".into())
    );
}

#[test]
fn auto_clone_choice_uses_the_two_hundred_megabyte_rule() {
    let large = runtime([response(
        r#"{"default_branch":"main","private":false,"size":204801,"clone_url":"https://github.com/acme/repo.git","permissions":{}}"#,
    )]);
    large.vault.put("github-oauth", "token").expect("token");
    assert_eq!(
        crate::git::git_clone::test_clone_choice(
            &large,
            "https://github.com/acme/repo.git",
            super::super::ShallowChoiceView::Auto,
        )
        .expect("large")
        .1,
        Some(1)
    );
    let small = runtime([response(
        r#"{"default_branch":"main","private":false,"size":204800,"clone_url":"https://github.com/acme/repo.git","permissions":{}}"#,
    )]);
    small.vault.put("github-oauth", "token").expect("token");
    assert_eq!(
        crate::git::git_clone::test_clone_choice(
            &small,
            "https://github.com/acme/repo.git",
            super::super::ShallowChoiceView::Auto,
        )
        .expect("small")
        .1,
        None
    );
}

#[test]
fn device_flow_maps_pending_slow_down_authorized_denied_and_expired() {
    let start = response(
        r#"{"device_code":"device","user_code":"ABCD","verification_uri":"https://github.com/login/device","expires_in":600,"interval":1}"#,
    );
    let pending = response(r#"{"error":"authorization_pending"}"#);
    let slow = response(r#"{"error":"slow_down"}"#);
    let token = response(r#"{"access_token":"oauth-token"}"#);
    let denied = response(r#"{"error":"access_denied"}"#);
    let expired = response(r#"{"error":"expired_token"}"#);
    let runtime = runtime([
        start.clone(),
        pending,
        slow,
        token,
        scoped_user(),
        start.clone(),
        denied,
        start,
        expired,
    ]);
    runtime.configure_github("client").expect("client");
    assert_eq!(
        runtime
            .github_sign_in_start()
            .expect("start")
            .interval_seconds,
        5
    );
    assert_eq!(
        runtime.github_sign_in_poll().expect("pending"),
        SignInStateView::Pending
    );
    assert_eq!(
        runtime.github_sign_in_poll().expect("slow"),
        SignInStateView::SlowDown
    );
    assert_eq!(
        runtime.github_sign_in_poll().expect("authorized"),
        SignInStateView::Authorized("blair".into())
    );
    runtime.github_sign_in_start().expect("second start");
    assert_eq!(
        runtime.github_sign_in_poll().expect("denied"),
        SignInStateView::Denied
    );
    runtime.github_sign_in_start().expect("third start");
    assert_eq!(
        runtime.github_sign_in_poll().expect("expired"),
        SignInStateView::Expired
    );
}

#[test]
fn file_clone_records_remote_and_reports_progress() {
    let (_dir, runtime, _source, workspace, database, url) = clone_fixture();
    let progress = Arc::new(Progress {
        phases: Mutex::new(Vec::new()),
        runtime: Mutex::new(Some(Arc::downgrade(&runtime))),
        cancel: false,
    });
    let id = runtime
        .clone_repository(
            &url,
            Some("cloned".into()),
            super::super::ShallowChoiceView::Full,
            progress.clone(),
        )
        .expect("clone");
    let store = Store::open(&database).expect("store");
    let project = store
        .load_project(&tether_types::ProjectId::new(id))
        .expect("project read")
        .expect("project exists");
    assert_eq!(
        store.project_remote(&project.id).expect("remote"),
        Some(tether_store::ProjectRemote {
            remote_url: Some(url),
            default_branch: Some("main".into()),
        })
    );
    assert!(workspace.join(project.path).join(".git").exists());
    assert!(
        progress
            .phases
            .lock()
            .expect("phase lock")
            .contains(&"Receiving".into())
    );
}

#[test]
fn failed_or_cancelled_clone_cleans_up_project_and_folder() {
    let (_dir, runtime, _source, workspace, database, url) = clone_fixture();
    let cancelling = Arc::new(Progress {
        phases: Mutex::new(Vec::new()),
        runtime: Mutex::new(Some(Arc::downgrade(&runtime))),
        cancel: true,
    });
    assert!(
        runtime
            .clone_repository(
                &url,
                Some("cancelled".into()),
                super::super::ShallowChoiceView::Full,
                cancelling,
            )
            .is_err()
    );
    assert_clone_cleanup(&database, &workspace);
    assert!(
        runtime
            .clone_repository(
                "file:///missing-tether-fixture",
                Some("failed".into()),
                super::super::ShallowChoiceView::Full,
                Arc::new(Progress {
                    phases: Mutex::new(Vec::new()),
                    runtime: Mutex::new(None),
                    cancel: false,
                }),
            )
            .is_err()
    );
    assert_clone_cleanup(&database, &workspace);
}

fn clone_fixture() -> (
    tempfile::TempDir,
    Arc<GitRuntime>,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    String,
) {
    let dir = tempfile::tempdir().expect("fixture directory");
    let source = dir.path().join("source");
    std::fs::create_dir_all(&source).expect("source");
    git(&source, &["init", "-q", "-b", "main"]);
    git(&source, &["config", "user.name", "T"]);
    git(&source, &["config", "user.email", "t@example.test"]);
    std::fs::write(source.join("file.txt"), "content\n").expect("file");
    git(&source, &["add", "file.txt"]);
    git(&source, &["commit", "-q", "-m", "base"]);
    let workspace = dir.path().join("workspaces");
    let database = dir.path().join("tether.db");
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = GitRuntime::new(
        database.to_str().expect("database"),
        workspace.to_str().expect("workspace"),
        vault,
    )
    .expect("runtime");
    let url = reqwest::Url::from_file_path(&source)
        .expect("file URL")
        .to_string();
    (dir, runtime, source, workspace, database, url)
}

fn assert_clone_cleanup(database: &Path, workspace: &Path) {
    assert_eq!(
        Store::open(database)
            .expect("store")
            .list_projects()
            .expect("projects")
            .len(),
        0
    );
    assert_eq!(
        std::fs::read_dir(workspace)
            .expect("workspace entries")
            .count(),
        0
    );
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("git");
    assert!(status.success(), "git {args:?}");
}
