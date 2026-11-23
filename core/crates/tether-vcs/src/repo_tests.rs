use std::fs;
use std::os::unix::fs::PermissionsExt;

use super::test_support::{TempDir, add_commit, git_ok, identity, init_repo, write};
use super::{DiffSpec, FileState, RepoError};

#[test]
fn status_reports_expected_entries_in_path_order() {
    let (dir, repo) = init_repo("status");
    write(&dir.path().join("deleted.txt"), b"gone\n");
    write(&dir.path().join("modified.txt"), b"before\n");
    write(&dir.path().join("old-name.txt"), b"move me\n");
    write(&dir.path().join(".gitignore"), b"ignored.txt\n");
    add_commit(dir.path(), "base");

    write(&dir.path().join("staged-add.txt"), b"new\n");
    repo.stage(&["staged-add.txt"]).expect("stage add");
    write(&dir.path().join("modified.txt"), b"after\n");
    fs::remove_file(dir.path().join("deleted.txt")).expect("remove deleted file");
    repo.stage(&["deleted.txt"]).expect("stage deletion");
    git_ok(dir.path(), &["mv", "old-name.txt", "renamed.txt"]);
    write(&dir.path().join("untracked.txt"), b"untracked\n");
    write(&dir.path().join("ignored.txt"), b"ignored\n");

    let entries = repo.status().expect("status");
    let summary: Vec<_> = entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry.index_state, entry.worktree_state))
        .collect();
    assert_eq!(
        summary,
        vec![
            ("deleted.txt", Some(FileState::Deleted), None),
            ("modified.txt", None, Some(FileState::Modified)),
            ("renamed.txt", Some(FileState::Renamed), None),
            ("staged-add.txt", Some(FileState::Added), None),
            ("untracked.txt", None, Some(FileState::Untracked)),
        ]
    );
}

#[test]
fn diff_matches_git_for_worktree_staged_head_and_revisions() {
    let (dir, repo) = init_repo("diff");
    write(&dir.path().join("added.txt"), b"added\n");
    write(&dir.path().join("deleted.txt"), b"deleted\n");
    write(
        &dir.path().join("edited.txt"),
        b"one\ntwo\nthree\nfour\nfive\n",
    );
    write(&dir.path().join("binary.bin"), &[0, 1, 2, 3]);
    write(&dir.path().join("mode.sh"), b"echo old\n");
    write(&dir.path().join("no-newline.txt"), b"old");
    let first = add_commit(dir.path(), "base");

    write(&dir.path().join("added.txt"), b"added changed\n");
    fs::remove_file(dir.path().join("deleted.txt")).expect("remove file");
    write(
        &dir.path().join("edited.txt"),
        b"one changed\ntwo\nthree changed\nfour\nfive changed\n",
    );
    write(&dir.path().join("binary.bin"), &[0, 1, 9, 3]);
    fs::set_permissions(
        dir.path().join("mode.sh"),
        fs::Permissions::from_mode(0o755),
    )
    .expect("set executable bit");
    write(&dir.path().join("no-newline.txt"), b"new");

    let expected_worktree = git_ok(dir.path(), &["diff"]);
    let actual_worktree = repo.diff(DiffSpec::Worktree).expect("working tree diff");
    assert_eq!(actual_worktree.as_bytes(), expected_worktree.as_slice());

    git_ok(dir.path(), &["add", "--all"]);
    let expected_staged = git_ok(dir.path(), &["diff", "--cached"]);
    assert_eq!(
        repo.diff(DiffSpec::Staged).expect("staged diff").as_bytes(),
        expected_staged.as_slice()
    );
    let expected_head = git_ok(dir.path(), &["diff", "HEAD"]);
    assert_eq!(
        repo.diff(DiffSpec::Head).expect("head diff").as_bytes(),
        expected_head.as_slice()
    );

    let second = add_commit(dir.path(), "changed");
    let expected_revisions = git_ok(dir.path(), &["diff", &first, &second]);
    assert_eq!(
        repo.diff(DiffSpec::Revisions {
            old: first,
            new: second,
        })
        .expect("revision diff")
        .as_bytes(),
        expected_revisions.as_slice()
    );
}

#[test]
fn diff_adds_truncation_marker_after_one_mib() {
    let (dir, repo) = init_repo("diff-cap");
    write(&dir.path().join("large.txt"), b"old\n");
    add_commit(dir.path(), "base");
    write(&dir.path().join("large.txt"), &vec![b'x'; 1_100_000]);
    let diff = repo.diff(DiffSpec::Worktree).expect("large diff");
    assert!(diff.contains("[diff truncated at 1 MiB]"));
    assert!(diff.len() > 1024 * 1024);
}

#[test]
fn log_matches_git_order_parents_message_path_filter_and_limit() {
    let (dir, repo) = init_repo("log");
    write(&dir.path().join("common.txt"), b"base\n");
    let root = add_commit(dir.path(), "root");
    git_ok(dir.path(), &["checkout", "-b", "side"]);
    write(&dir.path().join("side.txt"), b"side\n");
    let side = add_commit(dir.path(), "side commit\n\nbody");
    git_ok(dir.path(), &["checkout", "main"]);
    write(&dir.path().join("main.txt"), b"main\n");
    let main = add_commit(dir.path(), "main commit");
    git_ok(
        dir.path(),
        &["merge", "--no-ff", "side", "-m", "merge commit"],
    );
    let merge = String::from_utf8(git_ok(dir.path(), &["rev-parse", "HEAD"]))
        .expect("sha is utf8")
        .trim()
        .to_owned();

    let log = repo.log(10, None).expect("log");
    let expected = git_ok(dir.path(), &["log", "-10", "--format=%H"]);
    let expected_ids: Vec<_> = String::from_utf8(expected)
        .expect("log is utf8")
        .lines()
        .map(str::to_owned)
        .collect();
    let actual_ids: Vec<_> = log.iter().map(|item| item.id.clone()).collect();
    assert_eq!(actual_ids, expected_ids);
    assert_eq!(log[0].id, merge);
    assert_eq!(log[0].parents, vec![main.clone(), side.clone()]);
    assert_eq!(log[0].subject, "merge commit");
    assert!(log.iter().any(|item| item.id == side));
    assert!(log.iter().any(|item| item.id == main));
    assert!(log.iter().any(|item| item.id == root));
    assert_eq!(log.len(), 4);

    let filtered = repo.log(10, Some("side.txt")).expect("filtered log");
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].id, log[0].id);
    assert_eq!(filtered[1].id, side);
    assert_eq!(repo.log(2, None).expect("limited log").len(), 2);
}

#[test]
fn branches_switch_commit_and_delete_preserve_git_invariants() {
    let (dir, repo) = init_repo("branches");
    write(&dir.path().join("file.txt"), b"base\n");
    add_commit(dir.path(), "base");
    repo.create_branch("feature/task", "HEAD")
        .expect("create branch");
    assert_eq!(
        super::unique_branch_name(&repo, "feature/task").expect("unique branch"),
        "feature/task-2"
    );
    assert_eq!(
        super::unique_branch_name(&repo, "new/task").expect("unused branch"),
        "new/task"
    );
    repo.switch_branch("feature/task").expect("switch branch");
    assert_eq!(
        fs::read(dir.path().join("file.txt")).expect("read file"),
        b"base\n"
    );
    write(&dir.path().join("file.txt"), b"feature\n");
    repo.stage(&["file.txt"]).expect("stage feature");
    let id = repo.commit("feature commit", &identity()).expect("commit");
    let format = String::from_utf8(git_ok(
        dir.path(),
        &["log", "-1", "--format=%an,%ae,%cn,%ce", &id],
    ))
    .expect("log is utf8");
    assert_eq!(
        format.trim(),
        "Given Author,given@example.test,Given Author,given@example.test"
    );
    assert_eq!(git_ok(dir.path(), &["fsck", "--strict"]), Vec::<u8>::new());
    repo.switch_branch("main").expect("switch main");
    repo.delete_branch("feature/task").expect("delete feature");
    assert!(repo.delete_branch("main").is_err());
}

#[test]
fn switching_dirty_conflict_names_the_path_and_empty_commits_are_rejected() {
    let (dir, repo) = init_repo("switch-conflict");
    write(&dir.path().join("same.txt"), b"base\n");
    add_commit(dir.path(), "base");
    repo.create_branch("other", "HEAD").expect("branch");
    repo.switch_branch("other").expect("switch");
    write(&dir.path().join("same.txt"), b"other\n");
    repo.stage(&["same.txt"]).expect("stage");
    repo.commit("other", &identity()).expect("commit other");
    repo.switch_branch("main").expect("switch main");
    write(&dir.path().join("same.txt"), b"dirty\n");
    let error = repo
        .switch_branch("other")
        .expect_err("dirty switch refused");
    assert!(matches!(error, RepoError::WouldOverwrite(path) if path == "same.txt"));
    assert!(matches!(
        repo.commit(" ", &identity()),
        Err(RepoError::EmptyMessage)
    ));
    assert!(matches!(
        repo.commit("empty", &identity()),
        Err(RepoError::EmptyCommit)
    ));
}

#[test]
fn create_branch_works_when_no_committer_is_configured() {
    // A phone has no global Git configuration, and Tether keeps the identity in its own
    // database, so the repository has no committer. The reflog entry still needs a signature.
    let dir = TempDir::new("no-committer");
    git_ok(dir.path(), &["init", "-b", "main"]);
    git_ok(
        dir.path(),
        &[
            "-c",
            "user.name=T",
            "-c",
            "user.email=t@example.test",
            "commit",
            "--allow-empty",
            "-m",
            "start",
        ],
    );
    let isolated = gix::open_opts(dir.path(), gix::open::Options::isolated()).expect("open");
    assert!(
        isolated.committer().is_none(),
        "the fixture has a committer"
    );
    super::repo::create_branch_in(isolated, "feature/no-identity", "HEAD").expect("create");
    let repo = super::Repo::open(dir.path()).expect("reopen");
    let names: Vec<String> = repo
        .branches()
        .expect("branches")
        .into_iter()
        .map(|branch| branch.name)
        .collect();
    assert!(
        names.contains(&"feature/no-identity".to_owned()),
        "{names:?}"
    );
}
