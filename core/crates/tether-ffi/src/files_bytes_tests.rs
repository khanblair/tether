use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::ProjectRuntime;

use super::*;

struct Fixture {
    _temp: tempfile::TempDir,
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
            _temp: temp,
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

    fn read(&self, path: &str, limit_bytes: u64) -> Result<FileBytesView, FilesRuntimeError> {
        self.runtime
            .read_file_bytes(&self.project_id, path, limit_bytes)
    }
}

#[test]
fn binary_bytes_come_back_unchanged() {
    let fixture = Fixture::new();
    let data = vec![0u8, 0xff, b'a', 0, 0x80, b'\n', 0xfe];
    fixture.write("image.bin", &data);
    let view = fixture.read("image.bin", 0).unwrap();
    assert_eq!(view.bytes, data);
    assert_eq!(view.path, "image.bin");
    assert_eq!(view.size_bytes, 7);
    assert!(!view.truncated);
}

#[test]
fn file_longer_than_limit_is_cut_and_marked_truncated() {
    let fixture = Fixture::new();
    let data: Vec<u8> = (0..100).collect();
    fixture.write("long.bin", &data);
    let view = fixture.read("long.bin", 10).unwrap();
    assert_eq!(view.bytes, data[..10]);
    assert_eq!(view.size_bytes, 100);
    assert!(view.truncated);
}

#[test]
fn limit_is_clamped_to_ten_mebibytes_and_zero_means_the_cap() {
    assert_eq!(byte_cap(0), BYTES_CAP);
    assert_eq!(byte_cap(BYTES_CAP + 1), BYTES_CAP);
    assert_eq!(byte_cap(u64::MAX), BYTES_CAP);
    assert_eq!(byte_cap(BYTES_CAP), BYTES_CAP);
    assert_eq!(byte_cap(42), 42);
}

#[test]
fn folder_is_invalid_path() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.root.join("sub")).unwrap();
    assert!(matches!(
        fixture.read("sub", 0),
        Err(FilesRuntimeError::InvalidPath)
    ));
}

#[test]
fn traversal_absolute_and_empty_paths_are_invalid() {
    let fixture = Fixture::new();
    fixture.write("a.txt", "x");
    for path in ["..", "../a.txt", "a/../a.txt", "/etc/passwd", ""] {
        assert!(
            matches!(fixture.read(path, 0), Err(FilesRuntimeError::InvalidPath)),
            "path {path:?} should be invalid"
        );
    }
}

#[test]
fn git_paths_are_invalid_even_when_the_file_exists() {
    let fixture = Fixture::new();
    fixture.write(".git/config", "url = https://example.invalid/repo");
    fixture.write("a/.git/x", "secret");
    for path in [".git/config", "a/.git/x"] {
        assert!(
            matches!(fixture.read(path, 0), Err(FilesRuntimeError::InvalidPath)),
            "path {path:?} should be invalid"
        );
    }
}

#[cfg(unix)]
#[test]
fn symlink_inside_project_is_refused() {
    let fixture = Fixture::new();
    fixture.write("real.txt", "safe");
    std::os::unix::fs::symlink(fixture.root.join("real.txt"), fixture.root.join("link")).unwrap();
    assert!(matches!(
        fixture.read("link", 0),
        Err(FilesRuntimeError::InvalidPath)
    ));
}

#[test]
fn missing_file_is_not_found() {
    let fixture = Fixture::new();
    assert!(matches!(
        fixture.read("none.bin", 0),
        Err(FilesRuntimeError::NotFound)
    ));
}

#[test]
fn unknown_project_is_not_found() {
    let fixture = Fixture::new();
    fixture.write("a.txt", "x");
    assert!(matches!(
        fixture.runtime.read_file_bytes("missing", "a.txt", 0),
        Err(FilesRuntimeError::NotFound)
    ));
}
