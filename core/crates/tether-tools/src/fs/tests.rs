use std::fs;
use std::io::ErrorKind;

use tether_workspace::PathGuard;

use super::ReadTools;

#[test]
fn read_tools_use_workspace_fixture() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::create_dir(root.path().join(".git")).unwrap();
    fs::write(root.path().join("src/main.rs"), "first\nneedle\nlast\n").unwrap();
    fs::write(root.path().join(".git/config"), "needle\n").unwrap();
    fs::write(root.path().join("binary.bin"), [0, 1, 2]).unwrap();
    let tools = ReadTools::new(PathGuard::new(root.path()).unwrap());

    let whole = tools.read_file("src/main.rs", None).unwrap();
    assert_eq!(whole.text, "first\nneedle\nlast\n");
    let slice = tools.read_file("src/main.rs", Some((2, 3))).unwrap();
    assert_eq!(slice.start_line, 2);
    assert_eq!(slice.text, "needle\nlast\n");
    assert_eq!(tools.list_dir("").unwrap().len(), 2);
    assert_eq!(tools.list_dir("src").unwrap()[0].path.to_string_lossy(), "src/main.rs");
    assert_eq!(tools.glob("*.rs").unwrap().len(), 1);
    let hits = tools.grep("needle", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].line, 2);
    assert_eq!(hits[0].path.to_string_lossy(), "src/main.rs");
}

#[test]
fn read_tools_reject_escapes_and_binary_input() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("binary.bin"), [0, 1, 2]).unwrap();
    fs::write(root.path().join("text.txt"), "ok\n").unwrap();
    let tools = ReadTools::new(PathGuard::new(root.path()).unwrap());

    assert_eq!(tools.read_file("../outside", None).unwrap_err().kind(), ErrorKind::PermissionDenied);
    assert_eq!(tools.read_file("binary.bin", None).unwrap_err().kind(), ErrorKind::InvalidInput);
    assert_eq!(tools.read_file("text.txt", Some((0, 1))).unwrap_err().kind(), ErrorKind::InvalidInput);
    assert_eq!(tools.read_file("text.txt", Some((2, 1))).unwrap_err().kind(), ErrorKind::InvalidInput);
    assert_eq!(tools.list_dir(".git").unwrap_err().kind(), ErrorKind::InvalidInput);
}

#[test]
fn read_file_rejects_oversized_text() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("large.txt"), vec![b'x'; 1024 * 1024 + 1]).unwrap();
    let tools = ReadTools::new(PathGuard::new(root.path()).unwrap());
    assert_eq!(tools.read_file("large.txt", None).unwrap_err().kind(), ErrorKind::InvalidInput);
}

#[cfg(unix)]
#[test]
fn read_tools_do_not_follow_symlinks() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("private.txt"), "private").unwrap();
    symlink(outside.path().join("private.txt"), root.path().join("link.txt")).unwrap();
    let tools = ReadTools::new(PathGuard::new(root.path()).unwrap());
    assert_eq!(tools.read_file("link.txt", None).unwrap_err().kind(), ErrorKind::PermissionDenied);
    assert!(tools.list_dir("").unwrap().is_empty());
    assert!(tools.glob("*.txt").unwrap().is_empty());
}
