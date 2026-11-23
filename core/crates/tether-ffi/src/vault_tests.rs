use super::{VaultRuntime, VaultRuntimeError};
use std::collections::BTreeMap;
use std::fs;
use std::sync::{Arc, Mutex};

use super::{ImportMode, VaultFolderPlatform};
use tether_workspace::archive::{CreateOptions, create_zip};

struct FakeFolder {
    files: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl FakeFolder {
    fn new(files: BTreeMap<String, Vec<u8>>) -> Self {
        Self {
            files: Mutex::new(files),
        }
    }
}

impl VaultFolderPlatform for FakeFolder {
    fn list(&self) -> Result<Vec<String>, super::VaultFolderPlatformError> {
        Ok(self.files.lock().unwrap().keys().cloned().collect())
    }

    fn read(&self, path: String) -> Result<Option<Vec<u8>>, super::VaultFolderPlatformError> {
        Ok(self.files.lock().unwrap().get(&path).cloned())
    }

    fn write(&self, path: String, bytes: Vec<u8>) -> Result<(), super::VaultFolderPlatformError> {
        self.files.lock().unwrap().insert(path, bytes);
        Ok(())
    }
}

struct FailingFolder;

impl VaultFolderPlatform for FailingFolder {
    fn list(&self) -> Result<Vec<String>, super::VaultFolderPlatformError> {
        Err(super::VaultFolderPlatformError::Failed)
    }

    fn read(&self, _path: String) -> Result<Option<Vec<u8>>, super::VaultFolderPlatformError> {
        Err(super::VaultFolderPlatformError::Failed)
    }

    fn write(&self, _path: String, _bytes: Vec<u8>) -> Result<(), super::VaultFolderPlatformError> {
        Err(super::VaultFolderPlatformError::Failed)
    }
}

#[test]
fn vault_runtime_write_read_append_search_links_orphans_and_map() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("vault");
    let runtime = VaultRuntime::new(root.to_str().unwrap()).unwrap();
    assert!(root.is_dir());
    runtime
        .write(
            "Map.md",
            "---\ndescription: Entry map\ntype: moc\n---\n[[Note#First|open]]",
        )
        .unwrap();
    runtime
        .write(
            "Note.md",
            "---\ndescription: Useful note\ntags: [work]\n---\n# First",
        )
        .unwrap();
    runtime
        .write("Loose.md", "---\ndescription: Unlinked note\n---\nText")
        .unwrap();
    runtime.append("Note.md", "\n[[Map]]").unwrap();
    let note = runtime.read("Note.md").unwrap();
    assert_eq!(
        (
            note.path,
            note.description,
            note.tags,
            note.headings,
            note.source
        ),
        (
            "Note.md".into(),
            "Useful note".into(),
            vec!["work".into()],
            vec!["First".into()],
            "---\ndescription: Useful note\ntags: [work]\n---\n# First\n[[Map]]".into()
        )
    );
    assert_eq!(runtime.search("useful", 10).unwrap().len(), 1);
    let links = runtime.links("Note.md").unwrap();
    assert_eq!(links.outgoing[0].from, "Note.md");
    assert_eq!(links.incoming[0].target, "Note");
    assert_eq!(links.incoming[0].heading.as_deref(), Some("First"));
    assert_eq!(links.incoming[0].alias.as_deref(), Some("open"));
    assert!(links.incoming[0].resolved);
    assert_eq!(runtime.orphans().unwrap(), vec!["Loose.md"]);
    assert_eq!(
        runtime.map("Map.md", 1, 100).unwrap(),
        "Map.md: Entry map\nNote.md: Useful note\n"
    );
}

#[test]
fn vault_runtime_links_expose_resolved_paths() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = VaultRuntime::new(dir.path().to_str().unwrap()).unwrap();
    runtime
        .write(
            "folder/Map.md",
            "---\ndescription: Links\n---\n[[Plan]] [[Local]] [[Auth]] [[Shared]] [[Missing]]",
        )
        .unwrap();
    for path in [
        "Plan.md",
        "folder/Local.md",
        "notes/Auth.md",
        "one/Shared.md",
        "two/Shared.md",
    ] {
        runtime
            .write(path, "---\ndescription: Note\n---\nText")
            .unwrap();
    }
    let links = runtime.links("folder/Map.md").unwrap();
    let paths: Vec<_> = links
        .outgoing
        .iter()
        .map(|link| link.resolved_path.as_deref())
        .collect();
    assert_eq!(
        paths,
        [
            Some("Plan.md"),
            Some("folder/Local.md"),
            Some("notes/Auth.md"),
            None,
            None
        ]
    );
    assert!(
        links
            .outgoing
            .iter()
            .all(|link| link.resolved == link.resolved_path.is_some())
    );
    let incoming = runtime.links("notes/Auth.md").unwrap().incoming;
    assert_eq!(incoming[0].resolved_path.as_deref(), Some("notes/Auth.md"));
}

#[test]
fn vault_runtime_rejects_escape_and_redacts_invalid_note_path() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("vault");
    let runtime = VaultRuntime::new(root.to_str().unwrap()).unwrap();
    assert!(matches!(
        runtime.write("../escape.md", "---\ndescription: Safe\n---"),
        Err(VaultRuntimeError::InvalidPath)
    ));
    let error = runtime.write("Bad.md", "---\ntitle: Bad\n---").unwrap_err();
    assert!(matches!(error, VaultRuntimeError::InvalidNote));
    assert_eq!(error.to_string(), "invalid vault note");
    assert!(!error.to_string().contains(dir.path().to_str().unwrap()));
    fs::write(root.join("Broken.md"), "invalid").unwrap();
    let error = runtime.search("bad", 10).unwrap_err();
    assert_eq!(error.to_string(), "invalid vault note");
}

#[cfg(unix)]
#[test]
fn vault_runtime_rejects_symlinked_root() {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let root = dir.path().join("vault");
    std::os::unix::fs::symlink(outside, &root).unwrap();
    assert!(matches!(
        VaultRuntime::new(root.to_str().unwrap()),
        Err(VaultRuntimeError::InvalidPath)
    ));
}

#[test]
fn vault_export_import_skips_obsidian_and_preserves_attachments() {
    let source_dir = tempfile::tempdir().unwrap();
    let runtime = VaultRuntime::new(source_dir.path().to_str().unwrap()).unwrap();
    runtime
        .write("Note.md", "---\ndescription: Note\n---\nText")
        .unwrap();
    fs::write(source_dir.path().join("image.png"), b"png").unwrap();
    fs::create_dir_all(source_dir.path().join(".obsidian")).unwrap();
    fs::write(source_dir.path().join(".obsidian/app.json"), b"{}").unwrap();
    let archive = source_dir.path().join("vault.zip");
    runtime
        .export_zip(archive.to_string_lossy().into_owned())
        .unwrap();

    let destination = tempfile::tempdir().unwrap();
    let imported = VaultRuntime::new(destination.path().to_str().unwrap()).unwrap();
    let report = imported
        .import_zip(
            archive.to_string_lossy().into_owned(),
            ImportMode::ReplaceEmpty,
        )
        .unwrap();
    assert_eq!(report.imported, 2);
    assert!(destination.path().join("image.png").exists());
    assert!(!destination.path().join(".obsidian/app.json").exists());
}

#[test]
fn vault_merge_conflict_gets_import_date_and_counts_it() {
    let source_dir = tempfile::tempdir().unwrap();
    let source = VaultRuntime::new(source_dir.path().to_str().unwrap()).unwrap();
    source
        .write("Note.md", "---\ndescription: Incoming\n---\nNew")
        .unwrap();
    let archive = source_dir.path().join("vault.zip");
    source
        .export_zip(archive.to_string_lossy().into_owned())
        .unwrap();
    let destination = tempfile::tempdir().unwrap();
    let runtime = VaultRuntime::new(destination.path().to_str().unwrap()).unwrap();
    runtime
        .write("Note.md", "---\ndescription: Existing\n---\nOld")
        .unwrap();
    let report = runtime
        .import_zip_at_date(
            archive.to_string_lossy().into_owned(),
            ImportMode::Merge,
            "2026-11-23",
        )
        .unwrap();
    assert_eq!(report.conflicts, 1);
    assert!(
        destination
            .path()
            .join("Note (imported 2026-11-23).md")
            .exists()
    );
}

#[test]
fn vault_import_skips_unknown_files_and_refuses_reserved_folders() {
    let source_dir = tempfile::tempdir().unwrap();
    fs::write(source_dir.path().join("unknown.txt"), b"skip").unwrap();
    fs::write(
        source_dir.path().join("Note.md"),
        b"---\ndescription: Note\n---\nText",
    )
    .unwrap();
    let archive_dir = tempfile::tempdir().unwrap();
    let archive = archive_dir.path().join("unknown.zip");
    create_zip(
        source_dir.path(),
        &archive,
        &CreateOptions {
            include_ignored: true,
            ..CreateOptions::default()
        },
        &|| false,
    )
    .unwrap();
    let destination = tempfile::tempdir().unwrap();
    let runtime = VaultRuntime::new(destination.path().to_str().unwrap()).unwrap();
    let report = runtime
        .import_zip(
            archive.to_string_lossy().into_owned(),
            ImportMode::ReplaceEmpty,
        )
        .unwrap();
    assert_eq!((report.imported, report.skipped), (1, 1));

    let reserved = tempfile::tempdir().unwrap();
    fs::create_dir_all(reserved.path().join(".obsidian")).unwrap();
    fs::write(reserved.path().join(".obsidian/config"), b"{}").unwrap();
    let reserved_archive_dir = tempfile::tempdir().unwrap();
    let reserved_zip = reserved_archive_dir.path().join("reserved.zip");
    create_zip(
        reserved.path(),
        &reserved_zip,
        &CreateOptions::default(),
        &|| false,
    )
    .unwrap();
    let empty = tempfile::tempdir().unwrap();
    let runtime = VaultRuntime::new(empty.path().to_str().unwrap()).unwrap();
    assert!(matches!(
        runtime.import_zip(
            reserved_zip.to_string_lossy().into_owned(),
            ImportMode::Merge
        ),
        Err(VaultRuntimeError::InvalidPath)
    ));
}

#[test]
fn vault_sync_copies_both_ways_and_persists_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = VaultRuntime::new(dir.path().to_str().unwrap()).unwrap();
    runtime
        .write("Note.md", "---\ndescription: Private\n---\nOne")
        .unwrap();
    let fake = Arc::new(FakeFolder::new(BTreeMap::from([(
        "Other.md".into(),
        b"---\ndescription: Shared\n---\nTwo".to_vec(),
    )])));
    let shared: Arc<dyn VaultFolderPlatform> = fake.clone();
    let first = runtime.sync_with_platform(Arc::clone(&shared)).unwrap();
    assert_eq!(first.copied_to_shared, vec!["Note.md"]);
    assert_eq!(first.copied_to_private, vec!["Other.md"]);
    drop(runtime);
    let reopened = VaultRuntime::new(dir.path().to_str().unwrap()).unwrap();
    fake.files.lock().unwrap().insert(
        "Note.md".into(),
        b"---\ndescription: Shared edit\n---\nThree".to_vec(),
    );
    let second = reopened.sync_with_platform(shared).unwrap();
    assert_eq!(second.copied_to_private, vec!["Note.md"]);
    assert_eq!(
        reopened.read("Note.md").unwrap().source,
        "---\ndescription: Shared edit\n---\nThree"
    );
}

#[test]
fn vault_sync_keeps_two_sided_edits_and_reports_platform_failure() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = VaultRuntime::new(dir.path().to_str().unwrap()).unwrap();
    let source = "---\ndescription: Base\n---\nBase";
    runtime.write("Note.md", source).unwrap();
    let fake = Arc::new(FakeFolder::new(BTreeMap::from([(
        "Note.md".into(),
        source.as_bytes().to_vec(),
    )])));
    let shared: Arc<dyn VaultFolderPlatform> = fake.clone();
    runtime.sync_with_platform(Arc::clone(&shared)).unwrap();
    runtime
        .write("Note.md", "---\ndescription: Private\n---\nPrivate")
        .unwrap();
    fake.files.lock().unwrap().insert(
        "Note.md".into(),
        b"---\ndescription: Shared\n---\nShared".to_vec(),
    );
    let report = runtime.sync_with_platform(shared).unwrap();
    assert_eq!(report.conflicts, vec!["Note.md"]);
    assert!(runtime.read("Note.conflict-shared.md").is_ok());
    assert!(runtime.sync_with_platform(Arc::new(FailingFolder)).is_err());
}
