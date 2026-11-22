use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::test_support::{TempDir, add_commit, git, git_ok, init_repo, write};
use super::{
    CancellationToken, PushError, PushPhase, PushProgress, PushRequest, PushRisk, RefUpdate,
    Transport, TransportError, classify_push, push,
};
use std::collections::BTreeMap;

use tether_secrets::Secret;

struct Progress;

impl PushProgress for Progress {
    fn update(&self, _phase: PushPhase, _current: u64, _total: Option<u64>) {}
}

struct GitBackedTransport {
    bare: PathBuf,
    posts: AtomicUsize,
}

impl GitBackedTransport {
    fn new(root: &Path) -> Self {
        git_ok(root, &["init", "--bare", "remote.git"]);
        Self {
            bare: root.join("remote.git"),
            posts: AtomicUsize::new(0),
        }
    }

    fn post_count(&self) -> usize {
        self.posts.load(Ordering::Relaxed)
    }
}

impl Transport for GitBackedTransport {
    fn get(
        &self,
        url: &str,
        _headers: BTreeMap<String, String>,
    ) -> Result<Vec<u8>, TransportError> {
        assert_eq!(
            url,
            "https://example.test/repo.git/info/refs?service=git-receive-pack"
        );
        let output = Command::new("git")
            .args(["receive-pack", "--stateless-rpc", "--advertise-refs"])
            .arg(&self.bare)
            .output()
            .map_err(|_| TransportError)?;
        output
            .status
            .success()
            .then_some(output.stdout)
            .ok_or(TransportError)
    }

    fn post(
        &self,
        url: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, TransportError> {
        assert_eq!(url, "https://example.test/repo.git/git-receive-pack");
        assert_eq!(content_type, "application/x-git-receive-pack-request");
        self.posts.fetch_add(1, Ordering::Relaxed);
        let mut child = Command::new("git")
            .args(["receive-pack", "--stateless-rpc"])
            .arg(&self.bare)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|_| TransportError)?;
        child
            .stdin
            .as_mut()
            .ok_or(TransportError)?
            .write_all(&body)
            .map_err(|_| TransportError)?;
        let output = child.wait_with_output().map_err(|_| TransportError)?;
        output
            .status
            .success()
            .then_some(output.stdout)
            .ok_or(TransportError)
    }
}

fn push_request(dst: &str, src: Option<&str>, force: bool) -> PushRequest {
    PushRequest {
        remote_url: String::from("https://example.test/repo.git"),
        updates: vec![RefUpdate {
            src: src.map(str::to_owned),
            dst: dst.to_owned(),
            force,
        }],
    }
}

fn push_one(
    repo: &super::Repo,
    transport: &GitBackedTransport,
    request: &PushRequest,
) -> super::PushReport {
    push(
        repo,
        request,
        transport,
        &Progress,
        &CancellationToken::new(),
    )
    .expect("push fixture")
}

#[test]
fn push_creates_updates_forces_and_deletes_refs() {
    let (local_dir, local) = init_repo("push-refs");
    write(&local_dir.path().join("file.txt"), b"one\n");
    add_commit(local_dir.path(), "one");
    let remote_root = TempDir::new("push-refs-remote");
    let transport = GitBackedTransport::new(remote_root.path());
    let main = push_one(
        &local,
        &transport,
        &push_request("refs/heads/main", Some("HEAD"), false),
    );
    assert_eq!(main.updates[0].dst, "refs/heads/main");
    let branch = push_one(
        &local,
        &transport,
        &push_request("refs/heads/feature", Some("HEAD"), false),
    );
    assert!(branch.updates[0].accepted);

    diverged_force_and_delete(&local_dir, &local, &transport);
}

fn diverged_force_and_delete(
    local_dir: &super::test_support::TempDir,
    local: &super::Repo,
    transport: &GitBackedTransport,
) {
    write(&local_dir.path().join("file.txt"), b"two\n");
    add_commit(local_dir.path(), "two");
    push_one(
        local,
        transport,
        &push_request("refs/heads/main", Some("HEAD"), false),
    );
    git_ok(local_dir.path(), &["reset", "--hard", "HEAD^"]);
    write(&local_dir.path().join("file.txt"), b"diverged\n");
    add_commit(local_dir.path(), "diverged");
    let before = transport.post_count();
    assert!(matches!(
        push(
            local,
            &push_request("refs/heads/main", Some("HEAD"), false),
            transport,
            &Progress,
            &CancellationToken::new()
        ),
        Err(PushError::NonFastForward)
    ));
    assert_eq!(transport.post_count(), before);
    push_one(
        local,
        transport,
        &push_request("refs/heads/main", Some("HEAD"), true),
    );
    push_one(
        local,
        transport,
        &push_request("refs/heads/feature", None, false),
    );
    assert!(
        !git(
            &transport.bare,
            &["show-ref", "--verify", "refs/heads/feature"]
        )
        .status
        .success()
    );
}

#[test]
fn push_transfers_nested_binary_empty_and_executable_files() {
    let (local_dir, local) = init_repo("push-content");
    write(&local_dir.path().join("nested/path/blob"), &[0, 1, 2, 3]);
    write(&local_dir.path().join("empty"), b"");
    write(&local_dir.path().join("run.sh"), b"#!/bin/sh\n");
    let mut permissions = fs::metadata(local_dir.path().join("run.sh"))
        .expect("metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(local_dir.path().join("run.sh"), permissions).expect("executable");
    let sha = add_commit(local_dir.path(), "content");
    let remote_root = TempDir::new("push-content-remote");
    let transport = GitBackedTransport::new(remote_root.path());
    push_one(
        &local,
        &transport,
        &push_request("refs/heads/content", Some("HEAD"), false),
    );
    assert_remote_content(&transport, &sha);
}

fn assert_remote_content(transport: &GitBackedTransport, sha: &str) {
    assert_eq!(
        git_ok(&transport.bare, &["fsck", "--strict"]),
        Vec::<u8>::new()
    );
    assert_eq!(
        String::from_utf8(git_ok(
            &transport.bare,
            &["rev-parse", "refs/heads/content"]
        ))
        .expect("remote sha")
        .trim(),
        sha
    );
    assert_eq!(
        git_ok(
            &transport.bare,
            &["cat-file", "-p", "refs/heads/content:nested/path/blob"]
        ),
        &[0, 1, 2, 3]
    );
    assert_eq!(
        git_ok(
            &transport.bare,
            &["cat-file", "-s", "refs/heads/content:empty"]
        ),
        b"0\n"
    );
    assert!(
        git_ok(
            &transport.bare,
            &["ls-tree", "refs/heads/content", "run.sh"]
        )
        .starts_with(b"100755")
    );
}

#[test]
fn rejected_push_returns_the_hook_reason_and_risks_are_exact() {
    let (local_dir, local) = init_repo("push-reject");
    write(&local_dir.path().join("file"), b"one");
    add_commit(local_dir.path(), "one");
    let remote_root = TempDir::new("push-reject-remote");
    let transport = GitBackedTransport::new(remote_root.path());
    let hook = transport.bare.join("hooks/pre-receive");
    write(&hook, b"#!/bin/sh\necho hook-reason >&2\nexit 1\n");
    let mut permissions = fs::metadata(&hook).expect("hook metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&hook, permissions).expect("hook executable");
    let error = push(
        &local,
        &push_request("refs/heads/main", Some("HEAD"), false),
        &transport,
        &Progress,
        &CancellationToken::new(),
    )
    .expect_err("hook rejection");
    assert!(
        matches!(error, PushError::Rejected { ref reason, .. } if reason.contains("hook-reason"))
    );

    let new = push_request("refs/heads/new", Some("HEAD"), false);
    assert_eq!(
        classify_push(&local, &new, "main"),
        vec![PushRisk::NewBranch]
    );
    let fast = push_request("refs/heads/main", Some("HEAD"), false);
    assert_eq!(
        classify_push(&local, &fast, "main"),
        vec![PushRisk::DefaultBranch, PushRisk::FastForward]
    );
    let force = push_request("refs/heads/main", Some("HEAD"), true);
    assert_eq!(
        classify_push(&local, &force, "main"),
        vec![
            PushRisk::DefaultBranch,
            PushRisk::Force,
            PushRisk::FastForward
        ]
    );
    let delete = push_request("refs/heads/main", None, false);
    assert_eq!(
        classify_push(&local, &delete, "main"),
        vec![PushRisk::DefaultBranch, PushRisk::DeleteRemoteRef]
    );
}

#[test]
fn push_transport_hides_token_from_errors_and_repository_config() {
    let (dir, _repo) = init_repo("push-token");
    let token = "recognizable-push-token";
    let transport = super::ReqwestTransport::new(Secret::from_text(token)).expect("transport");
    let error = TransportError;
    assert!(!error.to_string().contains(token));
    assert!(!format!("{error:?}").contains(token));
    assert!(!format!("{transport:?}").contains(token));
    let config = fs::read(dir.path().join(".git/config")).expect("config");
    assert!(!String::from_utf8_lossy(&config).contains(token));
}
