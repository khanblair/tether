use std::sync::Arc;
use std::{fs, path::PathBuf};

use tether_store::{BlobStore, SnapshotInput, Store};
use tether_types::{ModelRef, PermissionMode, Provider};

use crate::{
    CoreEventView, EventSink, FilesRuntime, GitRuntime, RewindScopeView, SecretVault,
    SecretsPlatform, SecretsPlatformFailure, SessionRuntime,
};

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
fn custom_root_is_used_by_files_git_checkpoints_sessions_and_export() {
    let fixture = custom_root_fixture();
    assert_custom_root_location(&fixture);
    assert_git_and_files_use_custom_root(&fixture);
    assert_checkpoint_rewind_uses_custom_root(&fixture);
    assert_export_uses_custom_root(&fixture);
}

struct CustomRootFixture {
    temp: tempfile::TempDir,
    db: PathBuf,
    private: PathBuf,
    canonical_custom: PathBuf,
    runtime: Arc<ProjectRuntime>,
    project: ProjectView,
    project_root: PathBuf,
    vault: Arc<SecretVault>,
}

fn custom_root_fixture() -> CustomRootFixture {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("tether.db");
    let private = temp.path().join("workspaces");
    let custom = temp.path().join("shared");
    let runtime = ProjectRuntime::new(db.to_str().unwrap(), private.to_str().unwrap()).unwrap();
    runtime
        .set_new_project_root(Some(custom.to_string_lossy().into_owned()))
        .unwrap();
    let project = runtime.create_project("Shared".into()).unwrap();
    let project_root = custom.join(&project.path);
    let canonical_custom = fs::canonicalize(&custom).unwrap();
    let vault = SecretVault::new(
        temp.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    CustomRootFixture {
        temp,
        db,
        private,
        canonical_custom,
        runtime,
        project,
        project_root,
        vault,
    }
}

fn assert_custom_root_location(fixture: &CustomRootFixture) {
    assert_eq!(
        fixture.project.root,
        Some(fixture.canonical_custom.to_string_lossy().into_owned())
    );
    let location = fixture.runtime.workspace_location().unwrap();
    assert_eq!(location.default_root, fixture.private.to_string_lossy());
    assert_eq!(location.new_project_root, fixture.project.root);
    assert_eq!(
        location.effective_root,
        fixture.canonical_custom.to_string_lossy()
    );
    assert!(fixture.project_root.is_dir());
}

fn assert_git_and_files_use_custom_root(fixture: &CustomRootFixture) {
    let git = GitRuntime::new(
        &fixture.db.to_string_lossy(),
        &fixture.private.to_string_lossy(),
        Arc::clone(&fixture.vault),
    )
    .unwrap();
    git.git_init(&fixture.project.id, "main").unwrap();
    assert!(fixture.project_root.join(".git").is_dir());

    fs::write(fixture.project_root.join("note.txt"), b"after").unwrap();
    let files = FilesRuntime::new(
        &fixture.db.to_string_lossy(),
        &fixture.private.to_string_lossy(),
    )
    .unwrap();
    assert!(
        files
            .list_dir(&fixture.project.id, "", false)
            .unwrap()
            .entries
            .iter()
            .any(|entry| entry.name == "note.txt")
    );
}

fn assert_checkpoint_rewind_uses_custom_root(fixture: &CustomRootFixture) {
    let mut store = Store::open(&fixture.db).unwrap();
    let session = store
        .create_session(
            &tether_types::ProjectId::new(fixture.project.id.clone()),
            &ModelRef {
                provider: Provider::Anthropic,
                model_id: "model".into(),
            },
            PermissionMode::Ask,
            1,
        )
        .unwrap();
    let checkpoint = store
        .save_checkpoint(
            &session.id,
            None,
            &BlobStore::open(fixture.temp.path().join("blobs")).unwrap(),
            &[SnapshotInput {
                path: "note.txt".into(),
                content: Some(b"before".to_vec()),
            }],
            2,
        )
        .unwrap();
    fixture.vault.put("provider-anthropic", "test-key").unwrap();
    let sessions = SessionRuntime::new(
        &fixture.db.to_string_lossy(),
        Arc::clone(&fixture.vault),
        Arc::new(NoopSink),
    )
    .unwrap();
    let runner = sessions
        .open_session_with_workspace(&session.id.to_string(), &fixture.private.to_string_lossy())
        .unwrap();
    runner
        .rewind(checkpoint.id, RewindScopeView::Files)
        .unwrap();
    assert_eq!(
        fs::read(fixture.project_root.join("note.txt")).unwrap(),
        b"before"
    );
}

fn assert_export_uses_custom_root(fixture: &CustomRootFixture) {
    let archive = fixture.temp.path().join("shared.zip");
    fixture
        .runtime
        .export_project_zip(
            fixture.project.id.clone(),
            archive.to_string_lossy().into_owned(),
            false,
        )
        .unwrap();
    assert!(archive.is_file());
}

#[test]
fn changing_the_saved_root_does_not_move_existing_projects() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("tether.db");
    let private = temp.path().join("workspaces");
    let custom = temp.path().join("shared");
    let runtime = ProjectRuntime::new(&db.to_string_lossy(), &private.to_string_lossy()).unwrap();
    let old = runtime.create_project("Old".into()).unwrap();
    let old_path = private.join(&old.path);
    runtime
        .set_new_project_root(Some(custom.to_string_lossy().into_owned()))
        .unwrap();
    let new = runtime.create_project("New".into()).unwrap();
    assert!(old_path.is_dir());
    assert!(custom.join(&new.path).is_dir());
    assert_eq!(runtime.load_project(old.id).unwrap().root, None);
}

#[test]
fn same_name_in_different_roots_has_two_workspace_directories() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("tether.db");
    let private = temp.path().join("workspaces");
    let shared = temp.path().join("shared");
    let runtime = ProjectRuntime::new(&db.to_string_lossy(), &private.to_string_lossy()).unwrap();
    runtime
        .set_new_project_root(Some(shared.to_string_lossy().into_owned()))
        .unwrap();
    let shared_project = runtime.create_project("Same".into()).unwrap();
    runtime.set_new_project_root(None).unwrap();
    let private_project = runtime.create_project("Same".into()).unwrap();
    assert_ne!(shared_project.path, private_project.path);
    assert!(shared.join(shared_project.path).is_dir());
    assert!(private.join(private_project.path).is_dir());
}

#[test]
fn root_validation_reports_each_invalid_case() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("tether.db");
    let private = temp.path().join("workspaces");
    let runtime = ProjectRuntime::new(&db.to_string_lossy(), &private.to_string_lossy()).unwrap();
    assert!(matches!(
        runtime.set_new_project_root(Some("relative".into())),
        Err(ProjectRuntimeError::RootNotAbsolute)
    ));
    assert!(matches!(
        runtime.set_new_project_root(Some(
            temp.path()
                .join("missing-parent")
                .join("root")
                .to_string_lossy()
                .into_owned()
        )),
        Err(ProjectRuntimeError::RootCannotBeCreated)
    ));
    let file = temp.path().join("file");
    fs::write(&file, b"file").unwrap();
    assert!(matches!(
        runtime.set_new_project_root(Some(file.to_string_lossy().into_owned())),
        Err(ProjectRuntimeError::RootNotDirectory)
    ));
    assert!(matches!(
        runtime.set_new_project_root(Some(private.join("nested").to_string_lossy().into_owned())),
        Err(ProjectRuntimeError::RootInsidePrivateRoot)
    ));
    assert!(matches!(
        runtime.set_new_project_root(Some("/".into())),
        Err(ProjectRuntimeError::RootIsFilesystemRoot)
    ));

    #[cfg(unix)]
    assert_unix_root_validation(&runtime, &temp);
}

#[cfg(unix)]
fn assert_unix_root_validation(runtime: &ProjectRuntime, temp: &tempfile::TempDir) {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let target = temp.path().join("target");
    fs::create_dir(&target).unwrap();
    let link = temp.path().join("link");
    symlink(&target, &link).unwrap();
    assert!(matches!(
        runtime.set_new_project_root(Some(link.to_string_lossy().into_owned())),
        Err(ProjectRuntimeError::RootIsSymlink)
    ));
    let readonly = temp.path().join("readonly");
    fs::create_dir(&readonly).unwrap();
    fs::set_permissions(&readonly, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(matches!(
        runtime.set_new_project_root(Some(readonly.to_string_lossy().into_owned())),
        Err(ProjectRuntimeError::RootNotWritable)
    ));
    fs::set_permissions(&readonly, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn nested_project_root_is_rejected_and_none_restores_private_default() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("tether.db");
    let private = temp.path().join("workspaces");
    let shared = temp.path().join("shared");
    let runtime = ProjectRuntime::new(&db.to_string_lossy(), &private.to_string_lossy()).unwrap();
    runtime
        .set_new_project_root(Some(shared.to_string_lossy().into_owned()))
        .unwrap();
    let project = runtime.create_project("Outer".into()).unwrap();
    let nested = shared.join(&project.path).join("nested");
    assert!(matches!(
        runtime.set_new_project_root(Some(nested.to_string_lossy().into_owned())),
        Err(ProjectRuntimeError::RootInsideProject)
    ));
    runtime.set_new_project_root(None).unwrap();
    let location = runtime.workspace_location().unwrap();
    assert_eq!(location.new_project_root, None);
    assert_eq!(location.effective_root, private.to_string_lossy());
    let private_project = runtime.create_project("Private".into()).unwrap();
    assert_eq!(private_project.root, None);
    assert!(private.join(private_project.path).is_dir());
}

struct PlainPlatform;

impl SecretsPlatform for PlainPlatform {
    fn wrap(
        &self,
        plaintext: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, SecretsPlatformFailure> {
        Ok(plaintext)
    }

    fn unwrap(
        &self,
        wrapped: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, SecretsPlatformFailure> {
        Ok(wrapped)
    }
}

struct NoopSink;

impl EventSink for NoopSink {
    fn on_event(&self, _event: CoreEventView) {}
}
