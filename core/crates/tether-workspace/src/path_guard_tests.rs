use std::fs;
use std::path::Path;

use crate::PathGuard;

fn fixture() -> (std::path::PathBuf, PathGuard) {
    let root = std::env::temp_dir().join(format!("tether-guard-{}", ulid::Ulid::new()));
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("nested/file.txt"), b"text").unwrap();
    let guard = PathGuard::new(&root).unwrap();
    (guard.root().to_path_buf(), guard)
}

#[test]
fn existing_and_new_paths_stay_under_root() {
    let (root, guard) = fixture();
    assert_eq!(
        guard.resolve("nested/file.txt").unwrap(),
        root.join("nested/file.txt")
    );
    assert_eq!(
        guard.resolve("nested/new.txt").unwrap(),
        root.join("nested/new.txt")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn twenty_escape_inputs_are_rejected() {
    let (root, guard) = fixture();
    for depth in 1..=10 {
        let up = "../".repeat(depth);
        for input in [up.clone(), format!("nested/{up}outside")] {
            assert!(guard.resolve(&input).is_err(), "accepted {input}");
        }
    }
    for input in ["", "/etc/passwd", "./nested/file.txt", "nested/./file.txt"] {
        assert!(guard.resolve(input).is_err(), "accepted {input}");
    }
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn symlink_to_outside_is_rejected() {
    use std::os::unix::fs::symlink;

    let (root, guard) = fixture();
    symlink(Path::new("/tmp"), root.join("link")).unwrap();
    assert!(guard.resolve("link/other").is_err());
    fs::remove_dir_all(root).unwrap();
}
