use std::fs;

use tether_store::Store;

use super::*;

#[test]
fn create_project_makes_distinct_directories_under_root() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("tether.db");
    let root = temp.path().join("workspaces");
    let runtime = ProjectRuntime::new(db.to_str().unwrap(), root.to_str().unwrap()).unwrap();
    let first = runtime.create_project("My App".into()).unwrap();
    let second = runtime.create_project("My App".into()).unwrap();
    assert_ne!(first.path, second.path);
    assert!(root.join(&first.path).is_dir());
    assert!(root.join(&second.path).is_dir());
    assert_eq!(runtime.load_project(first.id.clone()).unwrap(), first);
}

#[test]
fn traversal_name_cannot_create_a_directory_outside_root() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("tether.db");
    let root = temp.path().join("workspaces");
    let runtime = ProjectRuntime::new(db.to_str().unwrap(), root.to_str().unwrap()).unwrap();
    let project = runtime.create_project("../../etc/passwd".into()).unwrap();
    assert!(!project.path.contains('/'));
    assert!(!project.path.contains(".."));
    assert!(root.join(&project.path).is_dir());
}

#[test]
fn failed_directory_creation_removes_the_project_row() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("tether.db");
    let root = temp.path().join("root-file");
    fs::write(&root, b"file").unwrap();
    let runtime = ProjectRuntime::new(db.to_str().unwrap(), root.to_str().unwrap()).unwrap();
    assert!(matches!(
        runtime.create_project("Fails".into()),
        Err(ProjectRuntimeError::Unavailable)
    ));
    assert_eq!(Store::open(&db).unwrap().list_projects().unwrap(), []);
}

#[test]
fn project_list_survives_reopening_and_unknown_id_is_not_found() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("tether.db");
    let root = temp.path().join("workspaces");
    let runtime = ProjectRuntime::new(db.to_str().unwrap(), root.to_str().unwrap()).unwrap();
    let project = runtime.create_project("Saved".into()).unwrap();
    drop(runtime);
    let reopened = ProjectRuntime::new(db.to_str().unwrap(), root.to_str().unwrap()).unwrap();
    assert_eq!(reopened.list_projects().unwrap(), vec![project]);
    assert!(matches!(
        reopened.load_project("missing".into()),
        Err(ProjectRuntimeError::NotFound)
    ));
    assert!(matches!(
        reopened.create_project("\n".into()),
        Err(ProjectRuntimeError::InvalidName)
    ));
}
