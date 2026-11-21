use std::fs;
use std::path::PathBuf;

use crate::ProjectRuntime;

use super::*;

struct Fixture {
    temp: tempfile::TempDir,
    runtime: Arc<FilesRuntime>,
    project_id: String,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("tether.db");
        let workspaces = temp.path().join("workspaces");
        let project_runtime =
            ProjectRuntime::new(database.to_str().unwrap(), workspaces.to_str().unwrap()).unwrap();
        let project = project_runtime.create_project("Test".into()).unwrap();
        let runtime =
            FilesRuntime::new(database.to_str().unwrap(), workspaces.to_str().unwrap()).unwrap();
        let root = workspaces.join(project.path);
        Self {
            temp,
            runtime,
            project_id: project.id,
            root,
        }
    }

    fn write(&self, path: &str, data: impl AsRef<[u8]>) {
        let target = self.root.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, data).unwrap();
    }

    fn list(&self, dir: &str, show_ignored: bool) -> DirListingView {
        self.runtime
            .list_dir(&self.project_id, dir, show_ignored)
            .unwrap()
    }
}

#[test]
fn list_dir_sorts_folders_then_files_case_insensitively() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.root.join("zeta")).unwrap();
    fs::create_dir(fixture.root.join("Alpha")).unwrap();
    fixture.write("beta.txt", "123");
    fixture.write("Zoo.txt", "x");
    let listing = fixture.list("", false);
    assert!(!listing.truncated);
    assert_eq!(
        listing.entries,
        vec![
            FileEntryView {
                name: "Alpha".into(),
                path: "Alpha".into(),
                is_dir: true,
                size_bytes: 0,
                ignored: false
            },
            FileEntryView {
                name: "zeta".into(),
                path: "zeta".into(),
                is_dir: true,
                size_bytes: 0,
                ignored: false
            },
            FileEntryView {
                name: "beta.txt".into(),
                path: "beta.txt".into(),
                is_dir: false,
                size_bytes: 3,
                ignored: false
            },
            FileEntryView {
                name: "Zoo.txt".into(),
                path: "Zoo.txt".into(),
                is_dir: false,
                size_bytes: 1,
                ignored: false
            },
        ]
    );
}

#[test]
fn list_dir_hides_git_symlinks_and_ignored_entries_until_toggled() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.root.join(".git")).unwrap();
    fs::create_dir(fixture.root.join("cache")).unwrap();
    fixture.write(".git/config", "x");
    fixture.write(".gitignore", "*.log\ncache\n");
    fixture.write("debug.log", "x");
    fixture.write("cache/data.txt", "x");
    fixture.write("keep.txt", "x");
    #[cfg(unix)]
    std::os::unix::fs::symlink(fixture.root.join("keep.txt"), fixture.root.join("link")).unwrap();
    let hidden = fixture.list("", false);
    assert_eq!(
        hidden
            .entries
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>(),
        [".gitignore", "keep.txt"]
    );
    let shown = fixture.list("", true);
    assert_eq!(
        shown
            .entries
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>(),
        ["cache", ".gitignore", "debug.log", "keep.txt"]
    );
    assert_eq!(
        shown
            .entries
            .iter()
            .map(|entry| entry.ignored)
            .collect::<Vec<_>>(),
        [true, false, true, false]
    );
    assert!(fixture.list("cache", true).entries[0].ignored);
}

#[test]
fn list_dir_caps_at_two_thousand_and_rejects_file_as_dir() {
    let fixture = Fixture::new();
    for number in 0..2001 {
        fixture.write(&format!("file-{number:04}.txt"), "x");
    }
    let listing = fixture.list("", false);
    assert_eq!(listing.entries.len(), 2000);
    assert!(listing.truncated);
    assert_eq!(listing.entries[0].name, "file-0000.txt");
    assert_eq!(listing.entries[1999].name, "file-1999.txt");
    assert!(matches!(
        fixture
            .runtime
            .list_dir(&fixture.project_id, "file-0000.txt", false),
        Err(FilesRuntimeError::InvalidPath)
    ));
}

#[test]
fn read_file_returns_text_and_truncates_on_utf8_boundary() {
    let fixture = Fixture::new();
    fixture.write("small.txt", "Hello\n");
    let small = fixture
        .runtime
        .read_file(&fixture.project_id, "small.txt")
        .unwrap();
    assert_eq!(
        small,
        FilePreviewView {
            path: "small.txt".into(),
            size_bytes: 6,
            text: Some("Hello\n".into()),
            truncated: false,
            binary: false
        }
    );
    let big = format!("{}érest", "a".repeat(256 * 1024 - 1));
    fixture.write("big.txt", &big);
    let preview = fixture
        .runtime
        .read_file(&fixture.project_id, "big.txt")
        .unwrap();
    assert_eq!(preview.path, "big.txt");
    assert_eq!(preview.size_bytes, big.len() as u64);
    assert_eq!(
        preview.text.as_deref(),
        Some("a".repeat(256 * 1024 - 1).as_str())
    );
    assert!(preview.truncated);
    assert!(!preview.binary);
}

#[test]
fn read_file_detects_invalid_utf8_and_nul() {
    let fixture = Fixture::new();
    fixture.write("invalid.bin", [0xff, 0xfe]);
    fixture.write("nul.bin", b"hello\0world");
    for path in ["invalid.bin", "nul.bin"] {
        let preview = fixture
            .runtime
            .read_file(&fixture.project_id, path)
            .unwrap();
        assert_eq!(preview.text, None);
        assert!(preview.binary);
        assert!(!preview.truncated);
    }
}

#[test]
fn read_file_rejects_symlink_parent_and_traversal() {
    let fixture = Fixture::new();
    fixture.write("real.txt", "safe");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(fixture.root.join("real.txt"), fixture.root.join("link"))
            .unwrap();
        assert!(matches!(
            fixture.runtime.read_file(&fixture.project_id, "link"),
            Err(FilesRuntimeError::InvalidPath)
        ));
    }
    assert!(matches!(
        fixture.runtime.read_file(&fixture.project_id, ".."),
        Err(FilesRuntimeError::InvalidPath)
    ));
    assert!(matches!(
        fixture.runtime.read_file(&fixture.project_id, "missing"),
        Err(FilesRuntimeError::NotFound)
    ));
    assert!(matches!(
        fixture.runtime.read_file(&fixture.project_id, ""),
        Err(FilesRuntimeError::InvalidPath)
    ));
}

#[test]
fn search_file_names_matches_glob_and_substring_excluding_ignored() {
    let fixture = Fixture::new();
    fixture.write(".gitignore", "*.log\n");
    fixture.write("src/Main.rs", "x");
    fixture.write("src/helper.rs", "x");
    fixture.write("debug.log", "x");
    assert_eq!(
        fixture
            .runtime
            .search_file_names(&fixture.project_id, "*.rs", 10)
            .unwrap(),
        ["src/Main.rs", "src/helper.rs"]
    );
    assert_eq!(
        fixture
            .runtime
            .search_file_names(&fixture.project_id, "MAIN", 10)
            .unwrap(),
        ["src/Main.rs"]
    );
    assert_eq!(
        fixture
            .runtime
            .search_file_names(&fixture.project_id, "SRC/", 10)
            .unwrap(),
        ["src/Main.rs", "src/helper.rs"]
    );
    assert_eq!(
        fixture
            .runtime
            .search_file_names(&fixture.project_id, "*.log", 10)
            .unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn search_file_names_caps_limit_at_one_hundred() {
    let fixture = Fixture::new();
    for number in 0..101 {
        fixture.write(&format!("match-{number:03}.txt"), "x");
    }
    let paths = fixture
        .runtime
        .search_file_names(&fixture.project_id, "match", 1000)
        .unwrap();
    assert_eq!(paths.len(), 100);
    assert_eq!(paths[0], "match-000.txt");
    assert_eq!(paths[99], "match-099.txt");
}

#[test]
fn search_file_text_returns_numbered_line_and_skips_binary() {
    let fixture = Fixture::new();
    fixture.write("a.bin", [0xff, b'\n', b'n']);
    fixture.write("b.txt", "first\nneedle here\n");
    assert_eq!(
        fixture
            .runtime
            .search_file_text(&fixture.project_id, "needle", 10)
            .unwrap(),
        [FileTextHitView {
            path: "b.txt".into(),
            line: 2,
            text: "needle here".into()
        }]
    );
}

#[test]
fn unknown_project_and_missing_directory_have_safe_errors() {
    let fixture = Fixture::new();
    assert!(matches!(
        fixture.runtime.list_dir("missing", "", false),
        Err(FilesRuntimeError::NotFound)
    ));
    assert!(matches!(
        fixture.runtime.read_file(&fixture.project_id, "none"),
        Err(FilesRuntimeError::NotFound)
    ));
    assert!(matches!(
        fixture.runtime.list_dir(&fixture.project_id, "none", false),
        Err(FilesRuntimeError::NotFound)
    ));
    fs::remove_dir_all(&fixture.root).unwrap();
    let error = fixture
        .runtime
        .list_dir(&fixture.project_id, "", false)
        .unwrap_err();
    assert!(matches!(error, FilesRuntimeError::Unavailable));
    for error in [
        FilesRuntimeError::InvalidPath,
        FilesRuntimeError::NotFound,
        error,
    ] {
        assert!(
            !error
                .to_string()
                .contains(fixture.temp.path().to_str().unwrap())
        );
    }
}
