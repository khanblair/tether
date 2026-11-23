use std::fs;

use tether_store::Store;
use tether_workspace::archive::{CreateOptions, ExtractLimits, create_zip};

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

#[test]
fn project_defaults_are_visible_after_reopening() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("tether.db");
    let root = temp.path().join("workspaces");
    let runtime = ProjectRuntime::new(db.to_str().unwrap(), root.to_str().unwrap()).unwrap();
    let project = runtime.create_project("Saved".into()).unwrap();
    runtime
        .set_project_defaults(
            project.id.clone(),
            Some(ProviderView::Anthropic),
            Some("saved-model".into()),
            Some(PermissionModeView::AcceptEdits),
        )
        .unwrap();
    let loaded = runtime.load_project(project.id.clone()).unwrap();
    assert_eq!(loaded.default_provider, Some(ProviderView::Anthropic));
    assert_eq!(loaded.default_model_id.as_deref(), Some("saved-model"));
    assert_eq!(loaded.default_mode, Some(PermissionModeView::AcceptEdits));
    drop(runtime);
    let reopened = ProjectRuntime::new(db.to_str().unwrap(), root.to_str().unwrap()).unwrap();
    assert_eq!(reopened.load_project(project.id.clone()).unwrap(), loaded);
    assert_eq!(reopened.list_projects().unwrap(), vec![loaded]);
}

#[test]
fn project_defaults_reject_incomplete_model_pairs() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("tether.db");
    let root = temp.path().join("workspaces");
    let runtime = ProjectRuntime::new(db.to_str().unwrap(), root.to_str().unwrap()).unwrap();
    let project = runtime.create_project("Saved".into()).unwrap();
    for (provider, model_id) in [
        (Some(ProviderView::Anthropic), None),
        (None, Some("saved-model".into())),
    ] {
        assert!(matches!(
            runtime.set_project_defaults(project.id.clone(), provider, model_id, None),
            Err(ProjectRuntimeError::InvalidDefaults)
        ));
    }
    assert_eq!(
        runtime.load_project(project.id).unwrap().default_provider,
        None
    );
}

#[test]
fn import_project_zip_creates_row_and_uses_archive_name() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::create_dir_all(source.join("src")).unwrap();
    fs::write(source.join("src/main.rs"), b"fn main() {}\n").unwrap();
    let archive = temp.path().join("demo.zip");
    create_zip(&source, &archive, &CreateOptions::default(), &|| false).unwrap();
    let runtime = ProjectRuntime::new(
        temp.path().join("tether.db").to_str().unwrap(),
        temp.path().join("workspaces").to_str().unwrap(),
    )
    .unwrap();

    let id = runtime
        .import_project_zip(archive.to_string_lossy().into_owned(), None)
        .unwrap();
    let project = runtime.load_project(id).unwrap();
    assert_eq!(project.name, "demo");
    assert_eq!(
        fs::read(
            temp.path()
                .join("workspaces")
                .join(project.path)
                .join("main.rs")
        )
        .unwrap(),
        b"fn main() {}\n"
    );
}

#[test]
fn project_zip_export_round_trips_and_can_include_ignored_files() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = ProjectRuntime::new(
        temp.path().join("tether.db").to_str().unwrap(),
        temp.path().join("workspaces").to_str().unwrap(),
    )
    .unwrap();
    let project = runtime.create_project("Demo".into()).unwrap();
    let root = temp.path().join("workspaces").join(&project.path);
    fs::write(root.join(".gitignore"), "ignored.txt\n").unwrap();
    fs::write(root.join("kept.txt"), b"kept").unwrap();
    fs::write(root.join("ignored.txt"), b"ignored").unwrap();
    let excluded = temp.path().join("excluded.zip");
    runtime
        .export_project_zip(
            project.id.clone(),
            excluded.to_string_lossy().into_owned(),
            false,
        )
        .unwrap();
    let included = temp.path().join("included.zip");
    runtime
        .export_project_zip(
            project.id.clone(),
            included.to_string_lossy().into_owned(),
            true,
        )
        .unwrap();
    let destination = temp.path().join("round-trip");
    fs::create_dir_all(&destination).unwrap();
    tether_workspace::archive::extract_zip(
        &excluded,
        &destination,
        &ExtractLimits::default(),
        &|| false,
        false,
    )
    .unwrap();
    assert!(destination.join("kept.txt").exists());
    assert!(!destination.join("ignored.txt").exists());
    let included_destination = temp.path().join("included-round-trip");
    fs::create_dir_all(&included_destination).unwrap();
    tether_workspace::archive::extract_zip(
        &included,
        &included_destination,
        &ExtractLimits::default(),
        &|| false,
        false,
    )
    .unwrap();
    assert!(included_destination.join("ignored.txt").exists());
}

#[test]
fn failed_project_import_removes_row_and_folder() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = ProjectRuntime::new(
        temp.path().join("tether.db").to_str().unwrap(),
        temp.path().join("workspaces").to_str().unwrap(),
    )
    .unwrap();
    let result = runtime.import_project_zip(
        temp.path()
            .join("malicious.zip")
            .to_string_lossy()
            .into_owned(),
        Some("Bad".into()),
    );
    assert!(matches!(result, Err(ProjectRuntimeError::Unavailable)));
    assert_eq!(runtime.list_projects().unwrap().len(), 0);
    assert!(
        !temp.path().join("workspaces").exists()
            || fs::read_dir(temp.path().join("workspaces"))
                .unwrap()
                .next()
                .is_none()
    );
}

#[test]
fn malicious_project_import_removes_row_and_folder() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("malicious.zip");
    fs::write(&archive, minimal_stored_zip(b"../evil")).unwrap();
    let runtime = ProjectRuntime::new(
        temp.path().join("tether.db").to_str().unwrap(),
        temp.path().join("workspaces").to_str().unwrap(),
    )
    .unwrap();
    assert!(matches!(
        runtime.import_project_zip(archive.to_string_lossy().into_owned(), Some("Bad".into())),
        Err(ProjectRuntimeError::Archive { .. })
    ));
    assert_eq!(runtime.list_projects().unwrap().len(), 0);
    assert!(
        !temp.path().join("workspaces").exists()
            || fs::read_dir(temp.path().join("workspaces"))
                .unwrap()
                .next()
                .is_none()
    );
}

#[test]
fn cancelled_project_import_is_cleaned_up() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::create_dir_all(&source).unwrap();
    for index in 0..20 {
        fs::write(source.join(format!("{index}.txt")), vec![b'x'; 1024]).unwrap();
    }
    let archive = temp.path().join("many.zip");
    create_zip(&source, &archive, &CreateOptions::default(), &|| false).unwrap();
    let runtime = ProjectRuntime::new(
        temp.path().join("tether.db").to_str().unwrap(),
        temp.path().join("workspaces").to_str().unwrap(),
    )
    .unwrap();
    // Cancel on the sixth check, so the import is stopped part way through, every time.
    let checks = std::cell::Cell::new(0_u32);
    let cancel_after_five = || {
        checks.set(checks.get() + 1);
        checks.get() > 5
    };
    let result = runtime.import_with_cancel(
        archive.to_str().unwrap(),
        Some("Cancelled".into()),
        &cancel_after_five,
    );
    assert!(matches!(result, Err(ProjectRuntimeError::Cancelled)));
    assert_eq!(runtime.list_projects().unwrap().len(), 0);
    let left = fs::read_dir(temp.path().join("workspaces")).map_or(0, Iterator::count);
    assert_eq!(left, 0, "the workspace folder must be removed");
}

fn minimal_stored_zip(name: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let offset = 0_u32;
    let central_size = 46_u32 + u32::try_from(name.len()).unwrap();
    put_u32(&mut bytes, 0x0403_4b50);
    put_u16(&mut bytes, 20);
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, 0);
    put_u32(&mut bytes, 0);
    put_u32(&mut bytes, 0);
    put_u32(&mut bytes, 0);
    put_u16(&mut bytes, u16::try_from(name.len()).unwrap());
    put_u16(&mut bytes, 0);
    bytes.extend_from_slice(name);
    put_u32(&mut bytes, 0x0201_4b50);
    put_u16(&mut bytes, 20);
    put_u16(&mut bytes, 20);
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, 0);
    put_u32(&mut bytes, 0);
    put_u32(&mut bytes, 0);
    put_u32(&mut bytes, 0);
    put_u16(&mut bytes, u16::try_from(name.len()).unwrap());
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, 0);
    put_u32(&mut bytes, 0);
    put_u32(&mut bytes, offset);
    bytes.extend_from_slice(name);
    put_u32(&mut bytes, 0x0605_4b50);
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, 1);
    put_u16(&mut bytes, 1);
    put_u32(&mut bytes, central_size);
    put_u32(&mut bytes, 30 + u32::try_from(name.len()).unwrap());
    put_u16(&mut bytes, 0);
    bytes
}

fn put_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
