use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use tether_secrets::Secret;

use super::test_support::{TempDir, add_commit, git_ok, init_repo, write};
use super::{CancellationToken, CloneError, ClonePhase, CloneProgress, CloneRequest, clone};

struct Progress {
    updates: AtomicUsize,
    cancel_on_receiving: Option<CancellationToken>,
}

impl CloneProgress for Progress {
    fn update(&self, phase: ClonePhase, _received: u64, _total: Option<u64>) {
        self.updates.fetch_add(1, Ordering::Relaxed);
        if phase == ClonePhase::Receiving
            && let Some(cancel) = &self.cancel_on_receiving
        {
            cancel.cancel();
        }
    }
}

fn file_url(path: &std::path::Path) -> String {
    reqwest::Url::from_file_path(path)
        .expect("fixture path is a file URL")
        .to_string()
}

fn request(url: String, dest: &std::path::Path) -> CloneRequest {
    CloneRequest {
        url,
        dest: dest.to_owned(),
        depth: None,
        branch: None,
        credentials: None,
    }
}

#[test]
fn file_clone_is_complete_and_shallow_clone_has_one_commit() {
    let (source_dir, _) = init_repo("clone-source");
    write(&source_dir.path().join("nested/file.txt"), b"source\n");
    add_commit(source_dir.path(), "one");
    write(&source_dir.path().join("nested/file.txt"), b"newer\n");
    add_commit(source_dir.path(), "two");
    let parent = TempDir::new("clone-dest");
    let full_path = parent.path().join("full");
    let progress = Progress {
        updates: AtomicUsize::new(0),
        cancel_on_receiving: None,
    };
    let full = clone(
        CloneRequest {
            credentials: Some(Secret::from_text("fake-token")),
            ..request(file_url(source_dir.path()), &full_path)
        },
        &progress,
        &CancellationToken::new(),
    )
    .expect("full clone");
    assert_eq!(
        git_ok(full.path(), &["rev-parse", "HEAD"]),
        git_ok(source_dir.path(), &["rev-parse", "HEAD"])
    );
    assert_eq!(
        fs::read(full.path().join("nested/file.txt")).expect("cloned file"),
        b"newer\n"
    );
    let config = fs::read(full.path().join(".git/config")).expect("clone config");
    assert!(!String::from_utf8_lossy(&config).contains("fake-token"));

    let shallow_path = parent.path().join("shallow");
    let shallow = clone(
        CloneRequest {
            depth: Some(1),
            ..request(file_url(source_dir.path()), &shallow_path)
        },
        &progress,
        &CancellationToken::new(),
    )
    .expect("shallow clone");
    assert_eq!(
        git_ok(shallow.path(), &["rev-list", "--count", "HEAD"]),
        b"1\n"
    );
}

#[test]
fn clone_branch_cancel_destination_and_url_guards_work() {
    let (source_dir, _) = init_repo("clone-guards-source");
    write(&source_dir.path().join("file.txt"), b"main\n");
    add_commit(source_dir.path(), "main");
    git_ok(source_dir.path(), &["checkout", "-b", "feature"]);
    write(&source_dir.path().join("file.txt"), b"feature\n");
    add_commit(source_dir.path(), "feature");
    let parent = TempDir::new("clone-guards-dest");
    branch_clone(source_dir.path(), parent.path());
    cancellation_leaves_no_destination(source_dir.path(), parent.path());
    destination_and_url_guards(source_dir.path(), parent.path());
}

fn branch_clone(source: &std::path::Path, parent: &std::path::Path) {
    let branch_path = parent.join("branch");
    let progress = Progress {
        updates: AtomicUsize::new(0),
        cancel_on_receiving: None,
    };
    let branch = clone(
        CloneRequest {
            branch: Some(String::from("feature")),
            ..request(file_url(source), &branch_path)
        },
        &progress,
        &CancellationToken::new(),
    )
    .expect("branch clone");
    assert_eq!(
        fs::read(branch.path().join("file.txt")).expect("branch file"),
        b"feature\n"
    );
}

fn cancellation_leaves_no_destination(source: &std::path::Path, parent: &std::path::Path) {
    let cancelled_path = parent.join("cancelled");
    let cancel = CancellationToken::new();
    let progress = Progress {
        updates: AtomicUsize::new(0),
        cancel_on_receiving: Some(cancel.clone()),
    };
    assert!(matches!(
        clone(
            request(file_url(source), &cancelled_path),
            &progress,
            &cancel
        ),
        Err(CloneError::Cancelled)
    ));
    assert!(!cancelled_path.exists());
}

fn destination_and_url_guards(source: &std::path::Path, parent: &std::path::Path) {
    let progress = Progress {
        updates: AtomicUsize::new(0),
        cancel_on_receiving: None,
    };
    let non_empty = parent.join("non-empty");
    fs::create_dir_all(&non_empty).expect("create destination");
    fs::write(non_empty.join("keep"), b"keep").expect("write destination");
    assert!(matches!(
        clone(
            request(file_url(source), &non_empty),
            &progress,
            &CancellationToken::new()
        ),
        Err(CloneError::DestinationNotEmpty)
    ));

    for url in [
        "http://host/repo",
        "ssh://host/repo",
        "git://host/repo",
        "https://user:pw@host/repo",
    ] {
        let error = clone(
            CloneRequest {
                credentials: Some(Secret::from_text("fake-token")),
                ..request(url.to_owned(), &parent.join("invalid"))
            },
            &progress,
            &CancellationToken::new(),
        )
        .expect_err("URL must be rejected");
        assert!(matches!(error, CloneError::InvalidUrl));
        assert!(!format!("{error:?}").contains("fake-token"));
    }
}
