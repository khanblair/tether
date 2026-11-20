use std::fs;
use std::path::PathBuf;

use tether_types::SessionId;
use super::{BlobStore, SnapshotInput, Store};

fn seeded_store() -> Store {
    let store = Store::open_in_memory().unwrap();
    store.conn.execute(
        "INSERT INTO projects (id, name, path, created_at) VALUES ('project', 'fixture', 'fixture', 1)",
        [],
    ).unwrap();
    store.conn.execute(
        "INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
         VALUES ('session', 'project', 'openai', 'fixture', 'ask', 'idle', 1, 1)",
        [],
    ).unwrap();
    store
}

#[test]
fn checkpoint_stores_prior_content_and_deduplicates_blobs() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    fs::write(workspace.join("existing.txt"), b"before\n").unwrap();
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let mut store = seeded_store();
    let paths = [
        SnapshotInput { path: PathBuf::from("existing.txt"), content: Some(fs::read(workspace.join("existing.txt")).unwrap()) },
        SnapshotInput { path: PathBuf::from("new.txt"), content: None },
    ];

    let first = store.save_checkpoint(&SessionId::new("session"), None, &blobs, &paths, 10).unwrap();
    fs::write(workspace.join("existing.txt"), b"after\n").unwrap();
    fs::write(workspace.join("new.txt"), b"created\n").unwrap();
    let loaded = store.load_checkpoint(&first.id).unwrap().unwrap();
    assert_eq!(loaded, first);
    assert!(loaded.files[0].existed);
    assert!(!loaded.files[1].existed);
    assert!(loaded.files[1].blob_hash.is_none());
    assert_eq!(blobs.get(loaded.files[0].blob_hash.as_deref().unwrap()).unwrap(), b"before\n");

    fs::write(workspace.join("existing.txt"), b"before\n").unwrap();
    let second = store.save_checkpoint(&SessionId::new("session"), None, &blobs, &paths[..1], 20).unwrap();
    assert_eq!(first.files[0].blob_hash, second.files[0].blob_hash);
    let fanout = fs::read_dir(root.path().join("blobs")).unwrap().next().unwrap().unwrap().path();
    assert_eq!(fs::read_dir(fanout).unwrap().count(), 1);
}

#[test]
fn checkpoint_rejects_escapes_and_duplicates() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let mut store = seeded_store();
    let session = SessionId::new("session");

    let input = |path: &str| SnapshotInput { path: path.into(), content: None };
    assert!(store.save_checkpoint(&session, None, &blobs, &[input("../outside")], 1).is_err());
    assert!(store.save_checkpoint(&session, None, &blobs, &[input("x"), input("x")], 1).is_err());
    assert!(store.save_checkpoint(&session, None, &blobs, &[input(".git/config")], 1).is_err());
}
