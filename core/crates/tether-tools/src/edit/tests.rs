use super::{EditError, EditSpec, EditTools};
use std::fs;
use tether_workspace::PathGuard;

fn assert_path_denied(result: Result<(), EditError>) {
    assert!(
        matches!(result, Err(EditError::Io(error)) if error.kind() == std::io::ErrorKind::PermissionDenied)
    );
}

fn tools(root: &std::path::Path) -> EditTools {
    EditTools::new(PathGuard::new(root).unwrap())
}

#[test]
fn failed_exact_match_never_changes_content() {
    let dir = tempfile::tempdir().unwrap();
    let tools = tools(dir.path());
    tools.write("sample.txt", "alpha\nbeta\nalpha\n").unwrap();
    let original = fs::read_to_string(dir.path().join("sample.txt")).unwrap();
    let ambiguous = EditSpec {
        old: "alpha".into(),
        new: "changed".into(),
    };
    assert!(matches!(
        tools.edit("sample.txt", &ambiguous),
        Err(EditError::AmbiguousMatch)
    ));
    let missing = EditSpec {
        old: "absent".into(),
        new: "changed".into(),
    };
    assert!(matches!(
        tools.edit("sample.txt", &missing),
        Err(EditError::MissingMatch)
    ));
    assert_eq!(
        fs::read_to_string(dir.path().join("sample.txt")).unwrap(),
        original
    );
}

#[test]
fn multi_edit_rolls_back_when_later_match_fails() {
    let dir = tempfile::tempdir().unwrap();
    let tools = tools(dir.path());
    tools.write("sample.txt", "red green").unwrap();
    let edits = [
        EditSpec {
            old: "red".into(),
            new: "blue".into(),
        },
        EditSpec {
            old: "missing".into(),
            new: "white".into(),
        },
    ];
    assert!(matches!(
        tools.multi_edit("sample.txt", &edits),
        Err(EditError::MissingMatch)
    ));
    assert_eq!(
        fs::read_to_string(dir.path().join("sample.txt")).unwrap(),
        "red green"
    );
}

#[test]
fn write_move_and_delete_stay_inside_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let tools = tools(dir.path());
    tools.write("nested/a.txt", "first").unwrap();
    tools.write("nested/b.txt", "keep").unwrap();
    assert!(matches!(
        tools.move_file("nested/a.txt", "nested/b.txt"),
        Err(EditError::DestinationExists)
    ));
    assert_eq!(
        fs::read_to_string(dir.path().join("nested/a.txt")).unwrap(),
        "first"
    );
    assert!(tools.write("../escape.txt", "no").is_err());
    tools.move_file("nested/a.txt", "nested/c.txt").unwrap();
    tools.delete("nested/c.txt").unwrap();
    assert!(!dir.path().join("nested/c.txt").exists());
}

#[test]
fn git_metadata_mutations_are_denied_without_changes() {
    let dir = tempfile::tempdir().unwrap();
    let tools = tools(dir.path());
    for path in [".git/config", "sub/.git/HEAD"] {
        let absolute = dir.path().join(path);
        fs::create_dir_all(absolute.parent().unwrap()).unwrap();
        fs::write(&absolute, "original").unwrap();
        assert_path_denied(tools.write(path, "changed"));
        let spec = EditSpec {
            old: "original".into(),
            new: "changed".into(),
        };
        assert_path_denied(tools.edit(path, &spec));
        assert_path_denied(tools.multi_edit(path, &[spec]));
        assert_path_denied(tools.move_file(path, "moved.txt"));
        assert_path_denied(tools.delete(path));
        assert_eq!(fs::read_to_string(absolute).unwrap(), "original");
    }
    tools.write("source.txt", "source").unwrap();
    assert_path_denied(tools.move_file("source.txt", ".git/moved"));
    assert_path_denied(tools.move_file("source.txt", "sub/.git/moved"));
    assert_eq!(
        fs::read_to_string(dir.path().join("source.txt")).unwrap(),
        "source"
    );
}

#[test]
fn git_named_neighbors_remain_writable() {
    let dir = tempfile::tempdir().unwrap();
    let tools = tools(dir.path());
    tools.write(".gitignore", "keep").unwrap();
    tools.write(".github/config", "keep").unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
        "keep"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join(".github/config")).unwrap(),
        "keep"
    );
}

#[test]
fn overlapping_matches_are_ambiguous() {
    let dir = tempfile::tempdir().unwrap();
    let tools = tools(dir.path());
    for (source, old) in [("ababa", "aba"), ("ééé", "éé")] {
        tools.write("sample.txt", source).unwrap();
        let spec = EditSpec {
            old: old.into(),
            new: "X".into(),
        };
        assert!(matches!(
            tools.edit("sample.txt", &spec),
            Err(EditError::AmbiguousMatch)
        ));
        assert_eq!(
            fs::read_to_string(dir.path().join("sample.txt")).unwrap(),
            source
        );
    }
    tools.write("sample.txt", "before aba after").unwrap();
    let spec = EditSpec {
        old: "aba".into(),
        new: "X".into(),
    };
    tools.edit("sample.txt", &spec).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("sample.txt")).unwrap(),
        "before X after"
    );
}

#[cfg(unix)]
#[test]
fn atomic_replacement_keeps_existing_mode_and_new_files_are_private() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let tools = tools(dir.path());
    let script = dir.path().join("script.sh");
    fs::write(&script, "old").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let spec = EditSpec {
        old: "old".into(),
        new: "new".into(),
    };
    tools.edit("script.sh", &spec).unwrap();
    assert_eq!(
        fs::metadata(&script).unwrap().permissions().mode() & 0o777,
        0o755
    );
    tools.write("new.txt", "new").unwrap();
    assert_eq!(
        fs::metadata(dir.path().join("new.txt"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}
