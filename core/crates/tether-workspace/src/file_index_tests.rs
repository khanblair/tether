use std::fs;

use crate::{FileIndex, PathGuard};

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
