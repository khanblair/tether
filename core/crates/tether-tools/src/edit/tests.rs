use super::{EditError, EditSpec, EditTools};
use std::fs;
use tether_workspace::PathGuard;

fn tools(root: &std::path::Path) -> EditTools {
    EditTools::new(PathGuard::new(root).unwrap())
}

#[test]
fn failed_exact_match_never_changes_content() {
    let dir = tempfile::tempdir().unwrap();
    let tools = tools(dir.path());
    tools.write("sample.txt", "alpha\nbeta\nalpha\n").unwrap();
    let original = fs::read_to_string(dir.path().join("sample.txt")).unwrap();
    let ambiguous = EditSpec { old: "alpha".into(), new: "changed".into() };
    assert!(matches!(tools.edit("sample.txt", &ambiguous), Err(EditError::AmbiguousMatch)));
    let missing = EditSpec { old: "absent".into(), new: "changed".into() };
    assert!(matches!(tools.edit("sample.txt", &missing), Err(EditError::MissingMatch)));
    assert_eq!(fs::read_to_string(dir.path().join("sample.txt")).unwrap(), original);
}

#[test]
fn multi_edit_rolls_back_when_later_match_fails() {
    let dir = tempfile::tempdir().unwrap();
    let tools = tools(dir.path());
    tools.write("sample.txt", "red green").unwrap();
    let edits = [
        EditSpec { old: "red".into(), new: "blue".into() },
        EditSpec { old: "missing".into(), new: "white".into() },
    ];
    assert!(matches!(tools.multi_edit("sample.txt", &edits), Err(EditError::MissingMatch)));
    assert_eq!(fs::read_to_string(dir.path().join("sample.txt")).unwrap(), "red green");
}

#[test]
fn write_move_and_delete_stay_inside_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let tools = tools(dir.path());
    tools.write("nested/a.txt", "first").unwrap();
    tools.write("nested/b.txt", "keep").unwrap();
    assert!(matches!(tools.move_file("nested/a.txt", "nested/b.txt"),
        Err(EditError::DestinationExists)));
    assert_eq!(fs::read_to_string(dir.path().join("nested/a.txt")).unwrap(), "first");
    assert!(tools.write("../escape.txt", "no").is_err());
    tools.move_file("nested/a.txt", "nested/c.txt").unwrap();
    tools.delete("nested/c.txt").unwrap();
    assert!(!dir.path().join("nested/c.txt").exists());
}
