use super::{VaultRuntime, VaultRuntimeError};
use std::fs;

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
