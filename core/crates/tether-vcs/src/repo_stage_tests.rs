use std::fs;
use std::os::unix::fs::symlink;

use super::RepoError;
use super::test_support::{TempDir, add_commit, git_ok, identity, init_repo, write};

fn committed_paths(dir: &std::path::Path) -> Vec<String> {
    String::from_utf8(git_ok(dir, &["ls-tree", "-r", "--name-only", "HEAD"]))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn stage_all_skips_ignored_files_and_keeps_tracked_changes_and_deletions() {
    let (dir, repo) = init_repo("stage-all");
    write(&dir.path().join(".gitignore"), b"*.log\nbuild/\n.env\n");
    write(&dir.path().join("keep.txt"), b"one\n");
    write(&dir.path().join("gone.txt"), b"bye\n");
    write(&dir.path().join("moved.txt"), b"move\n");
    add_commit(dir.path(), "base");

    write(&dir.path().join("keep.txt"), b"two\n");
    fs::remove_file(dir.path().join("gone.txt")).unwrap();
    fs::rename(dir.path().join("moved.txt"), dir.path().join("renamed.txt")).unwrap();
    write(&dir.path().join("new.txt"), b"new\n");
    write(&dir.path().join("debug.log"), b"noise\n");
    write(&dir.path().join(".env"), b"SECRET=1\n");
    write(&dir.path().join("build/out.bin"), b"bin\n");
    write(&dir.path().join("src/deep/inner.txt"), b"inner\n");

    repo.stage_all().expect("stage all");
    repo.commit("all", &identity()).expect("commit");
    assert_eq!(
        committed_paths(dir.path()),
        [
            ".gitignore",
            "keep.txt",
            "new.txt",
            "renamed.txt",
            "src/deep/inner.txt"
        ]
    );
    assert_eq!(
        git_ok(dir.path(), &["show", "HEAD:keep.txt"]),
        b"two\n".to_vec()
    );
}

#[test]
fn a_symlink_is_staged_as_its_link_text_never_as_the_file_it_points_to() {
    let (dir, repo) = init_repo("stage-link");
    let outside = TempDir::new("stage-link-outside");
    write(&outside.path().join("secret.txt"), b"TOP SECRET\n");
    write(&dir.path().join("a.txt"), b"a\n");
    add_commit(dir.path(), "base");
    let target = outside.path().join("secret.txt");
    symlink(&target, dir.path().join("link")).unwrap();

    repo.stage(&["link"]).expect("stage link");
    repo.commit("link", &identity()).expect("commit");
    let listing = String::from_utf8(git_ok(dir.path(), &["ls-tree", "HEAD", "link"])).unwrap();
    assert!(listing.starts_with("120000"), "{listing}");
    let blob = git_ok(dir.path(), &["cat-file", "-p", "HEAD:link"]);
    assert_eq!(blob, target.as_os_str().as_encoded_bytes());
    assert!(!String::from_utf8_lossy(&blob).contains("TOP SECRET"));
}

#[test]
fn a_symlinked_folder_is_not_walked_or_staged_through() {
    let (dir, repo) = init_repo("stage-dirlink");
    let outside = TempDir::new("stage-dirlink-outside");
    write(&outside.path().join("secret.txt"), b"TOP SECRET\n");
    write(&dir.path().join("a.txt"), b"a\n");
    add_commit(dir.path(), "base");
    symlink(outside.path(), dir.path().join("dirlink")).unwrap();

    assert!(matches!(
        repo.stage(&["dirlink/secret.txt"]),
        Err(RepoError::InvalidName)
    ));
    repo.stage_all().expect("stage all");
    repo.commit("links", &identity()).expect("commit");
    let paths = committed_paths(dir.path());
    assert!(paths.contains(&"dirlink".to_owned()));
    assert!(!paths.iter().any(|path| path.contains("secret.txt")));
}

#[test]
fn staging_inside_the_git_folder_is_refused() {
    let (dir, repo) = init_repo("stage-dotgit");
    write(&dir.path().join("a.txt"), b"a\n");
    add_commit(dir.path(), "base");
    assert!(matches!(
        repo.stage(&[".git/config"]),
        Err(RepoError::InvalidName)
    ));
    assert!(matches!(
        repo.stage(&["sub/.git/HEAD"]),
        Err(RepoError::InvalidName)
    ));
}

#[test]
fn switching_branches_refuses_to_drop_staged_changes() {
    let (dir, repo) = init_repo("switch-staged");
    write(&dir.path().join("f.txt"), b"base\n");
    add_commit(dir.path(), "base");
    repo.create_branch("other", "HEAD").expect("branch");
    write(&dir.path().join("f.txt"), b"staged edit\n");
    repo.stage(&["f.txt"]).expect("stage");
    // The worktree now matches the index, but the change is not in any commit yet.
    let error = repo
        .switch_branch("other")
        .expect_err("staged switch refused");
    assert!(matches!(error, RepoError::WouldOverwrite(path) if path == "f.txt"));
    assert_eq!(
        fs::read(dir.path().join("f.txt")).unwrap(),
        b"staged edit\n"
    );
    assert_eq!(repo.current_branch().unwrap().as_deref(), Some("main"));
}

#[test]
fn stage_all_follows_nested_ignore_files_the_info_exclude_and_tracked_files_in_ignored_folders() {
    let (dir, repo) = init_repo("stage-rules");
    write(&dir.path().join(".gitignore"), b"vendor/\n");
    write(&dir.path().join("sub/.gitignore"), b"*.tmp\n!keep.tmp\n");
    write(&dir.path().join("vendor/forced.txt"), b"one\n");
    add_commit(dir.path(), "base");
    git_ok(dir.path(), &["add", "-f", "vendor/forced.txt"]);
    git_ok(dir.path(), &["commit", "-q", "-m", "force add"]);
    write(&dir.path().join(".git/info/exclude"), b"private.txt\n");

    write(&dir.path().join("vendor/forced.txt"), b"two\n");
    write(&dir.path().join("vendor/other.txt"), b"untracked\n");
    write(&dir.path().join("sub/a.tmp"), b"ignored\n");
    write(&dir.path().join("sub/keep.tmp"), b"kept\n");
    write(&dir.path().join("sub/real.txt"), b"real\n");
    write(&dir.path().join("private.txt"), b"mine\n");
    write(&dir.path().join("top.tmp"), b"not ignored at the top\n");

    repo.stage_all().expect("stage all");
    repo.commit("rules", &identity()).expect("commit");
    assert_eq!(
        committed_paths(dir.path()),
        [
            ".gitignore",
            "sub/.gitignore",
            "sub/keep.tmp",
            "sub/real.txt",
            "top.tmp",
            "vendor/forced.txt"
        ]
    );
    assert_eq!(
        git_ok(dir.path(), &["show", "HEAD:vendor/forced.txt"]),
        b"two\n".to_vec()
    );
}

#[test]
fn stage_all_keeps_every_file_when_many_are_staged_in_one_call() {
    let (dir, repo) = init_repo("stage-many");
    write(&dir.path().join("seed.txt"), b"seed\n");
    add_commit(dir.path(), "base");
    let mut expected = vec!["seed.txt".to_owned()];
    for folder in 0..12 {
        for file in 0..25 {
            let path = format!("d{folder:02}/f{file:02}.txt");
            write(&dir.path().join(&path), format!("{path}\n").as_bytes());
            expected.push(path);
        }
    }
    expected.sort();
    repo.stage_all().expect("stage all");
    repo.commit("many", &identity()).expect("commit");
    assert_eq!(committed_paths(dir.path()), expected);
    assert_eq!(git_ok(dir.path(), &["fsck", "--strict"]), Vec::<u8>::new());
}
