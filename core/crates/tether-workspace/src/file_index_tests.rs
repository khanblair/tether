use std::fs;

use crate::{FileIndex, PathGuard, SearchHit};

#[test]
fn scan_glob_and_grep_respect_root_ignore() {
    let root = std::env::temp_dir().join(format!("tether-index-{}", ulid::Ulid::new()));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join(".gitignore"), "*.log\n").unwrap();
    fs::write(root.join("src/main.rs"), "first\nneedle\n").unwrap();
    fs::write(root.join("debug.log"), "needle\n").unwrap();
    fs::write(root.join(".git/config"), "needle\n").unwrap();
    let index = FileIndex::scan(PathGuard::new(&root).unwrap()).unwrap();
    assert_eq!(index.glob("*.rs").len(), 1);
    let hits = index.grep("needle", 1).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path.to_string_lossy(), "src/main.rs");
    assert_eq!(hits[0].line, 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scan_excludes_git_metadata_inside_nested_checkouts() {
    let root = std::env::temp_dir().join(format!("tether-index-{}", ulid::Ulid::new()));
    fs::create_dir_all(root.join("vendor/repo/.git")).unwrap();
    fs::write(root.join("vendor/repo/.git/config"), "needle\n").unwrap();
    fs::write(root.join("vendor/repo/source.rs"), "needle\n").unwrap();
    let index = FileIndex::scan(PathGuard::new(&root).unwrap()).unwrap();
    assert_eq!(index.glob("*").len(), 1);
    assert_eq!(index.grep("needle", 10).unwrap().len(), 1);
    assert_eq!(
        index.files()[0].path.to_string_lossy(),
        "vendor/repo/source.rs"
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn scan_ignores_external_gitignore_symlink() {
    use std::os::unix::fs::symlink;

    let fixture = std::env::temp_dir().join(format!("tether-index-{}", ulid::Ulid::new()));
    let root = fixture.join("workspace");
    fs::create_dir(&fixture).unwrap();
    fs::create_dir(&root).unwrap();
    fs::write(root.join("secret.txt"), "visible").unwrap();
    fs::write(fixture.join("outside-ignore"), "secret.txt\n").unwrap();
    symlink(fixture.join("outside-ignore"), root.join(".gitignore")).unwrap();
    let index = FileIndex::scan(PathGuard::new(&root).unwrap()).unwrap();
    assert_eq!(
        index
            .files()
            .iter()
            .map(|file| file.path.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ["secret.txt"]
    );
    fs::remove_file(root.join(".gitignore")).unwrap();
    fs::write(root.join(".gitignore"), "secret.txt\n").unwrap();
    let index = FileIndex::scan(PathGuard::new(&root).unwrap()).unwrap();
    assert_eq!(
        index
            .files()
            .iter()
            .map(|file| file.path.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        [".gitignore"]
    );
    fs::remove_dir_all(fixture).unwrap();
}

#[test]
fn grep_skips_binary_and_keeps_later_text_hits() {
    let root = std::env::temp_dir().join(format!("tether-index-{}", ulid::Ulid::new()));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("a.bin"), [0xff, 0xfe, 0x00]).unwrap();
    fs::write(root.join("b.txt"), "first\nneedle\n").unwrap();
    let index = FileIndex::scan(PathGuard::new(&root).unwrap()).unwrap();
    assert_eq!(
        index.grep("needle", 10).unwrap(),
        [SearchHit {
            path: "b.txt".into(),
            line: 2,
            text: "needle".into()
        }]
    );
    fs::remove_dir_all(root).unwrap();
}
