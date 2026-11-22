use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll, Waker};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Identity, Repo};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn new(label: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tether-vcs-{label}-{}-{stamp}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test directory");
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.path));
    }
}

pub(crate) fn git(path: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .expect("run system git")
}

pub(crate) fn git_ok(path: &Path, args: &[&str]) -> Vec<u8> {
    let output = git(path, args);
    assert!(
        output.status.success(),
        "git {:?}: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

pub(crate) fn init_repo(label: &str) -> (TempDir, Repo) {
    let dir = TempDir::new(label);
    git_ok(dir.path(), &["init", "-b", "main"]);
    git_ok(dir.path(), &["config", "user.name", "Fixture User"]);
    git_ok(
        dir.path(),
        &["config", "user.email", "fixture@example.test"],
    );
    let repo = Repo::open(dir.path()).expect("open fixture repository");
    (dir, repo)
}

pub(crate) fn write(path: &Path, content: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, content).expect("write fixture file");
}

pub(crate) fn add_commit(path: &Path, message: &str) -> String {
    git_ok(path, &["add", "--all"]);
    git_ok(path, &["commit", "--no-gpg-sign", "-m", message]);
    String::from_utf8(git_ok(path, &["rev-parse", "HEAD"]))
        .expect("git sha is utf8")
        .trim()
        .to_owned()
}

pub(crate) fn identity() -> Identity {
    Identity {
        name: String::from("Given Author"),
        email: String::from("given@example.test"),
    }
}

pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
