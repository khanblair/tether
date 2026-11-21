use std::fs;
use std::path::PathBuf;

use super::{BlobStore, SnapshotInput, Store};
use tether_types::SessionId;

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
fn checkpoint_list_hides_rewound_anchors_but_keeps_unanchored_entries() {
    let root = tempfile::tempdir().unwrap();
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let mut store = seeded_store();
    let session = SessionId::new("session");
    let anchor = store.begin_turn(&session, "first", 2).unwrap();
    let input = [SnapshotInput {
        path: "note.txt".into(),
        content: None,
    }];
    let anchored = store
        .save_checkpoint(&session, Some(&anchor), &blobs, &input, 3)
        .unwrap();
    let unanchored = store
        .save_checkpoint(&session, None, &blobs, &input, 4)
        .unwrap();
    store.rewind_conversation(&session, &anchor).unwrap();
    let ids: Vec<_> = store
        .list_checkpoints(&session)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(ids, vec![unanchored.id]);
    assert_ne!(ids, vec![anchored.id]);
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
        SnapshotInput {
            path: PathBuf::from("existing.txt"),
            content: Some(fs::read(workspace.join("existing.txt")).unwrap()),
        },
        SnapshotInput {
            path: PathBuf::from("new.txt"),
            content: None,
        },
    ];

    let first = store
        .save_checkpoint(&SessionId::new("session"), None, &blobs, &paths, 10)
        .unwrap();
    fs::write(workspace.join("existing.txt"), b"after\n").unwrap();
    fs::write(workspace.join("new.txt"), b"created\n").unwrap();
    let loaded = store.load_checkpoint(&first.id).unwrap().unwrap();
    assert_eq!(loaded, first);
    assert!(loaded.files[0].existed);
    assert!(!loaded.files[1].existed);
    assert!(loaded.files[1].blob_hash.is_none());
    assert_eq!(
        blobs
            .get(loaded.files[0].blob_hash.as_deref().unwrap())
            .unwrap(),
        b"before\n"
    );

    fs::write(workspace.join("existing.txt"), b"before\n").unwrap();
    let second = store
        .save_checkpoint(&SessionId::new("session"), None, &blobs, &paths[..1], 20)
        .unwrap();
    assert_eq!(first.files[0].blob_hash, second.files[0].blob_hash);
    let fanout = fs::read_dir(root.path().join("blobs"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(fs::read_dir(fanout).unwrap().count(), 1);
}

#[test]
fn blob_files_are_compressed_and_named_by_blake3() {
    let root = tempfile::tempdir().unwrap();
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let hash = blobs.put(b"before\n").unwrap();
    let expected = blake3::hash(b"before\n").to_hex().to_string();
    assert_eq!(hash, expected);
    let stored = fs::read(root.path().join("blobs").join(&hash[..2]).join(&hash)).unwrap();
    assert!(stored.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]));
    assert_eq!(blobs.get(&hash).unwrap(), b"before\n");
}

#[test]
fn a_corrupt_blob_is_rejected_on_read_and_not_treated_as_deduplicated() {
    let root = tempfile::tempdir().unwrap();
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let hash = blobs.put(b"snapshot text").unwrap();
    let path = root.path().join("blobs").join(&hash[..2]).join(&hash);
    fs::write(path, b"damaged").unwrap();
    assert!(blobs.get(&hash).is_err());
    assert!(blobs.put(b"snapshot text").is_err());
}

#[test]
fn simultaneous_writers_store_one_complete_blob() {
    let root = tempfile::tempdir().unwrap();
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let content = vec![b'a'; 256 * 1024];
    let start = std::sync::Arc::new(std::sync::Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let blobs = blobs.clone();
            let content = content.clone();
            let start = std::sync::Arc::clone(&start);
            std::thread::spawn(move || {
                start.wait();
                blobs.put(&content)
            })
        })
        .collect();
    let hashes: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap().unwrap())
        .collect();
    assert!(hashes.iter().all(|hash| hash == &hashes[0]));
    assert_eq!(blobs.get(&hashes[0]).unwrap(), content);
    assert_eq!(
        fs::read_dir(root.path().join("blobs").join(&hashes[0][..2]))
            .unwrap()
            .count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn blob_reads_and_deduplication_reject_a_symlinked_hash() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let hash = blobs.put(b"private snapshot").unwrap();
    let path = root.path().join("blobs").join(&hash[..2]).join(&hash);
    let outside = root.path().join("outside.zst");
    fs::rename(&path, &outside).unwrap();
    symlink(&outside, &path).unwrap();
    assert_eq!(
        blobs.get(&hash).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        blobs.put(b"private snapshot").unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
}

#[test]
fn checkpoint_rejects_escapes_and_duplicates() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
    let mut store = seeded_store();
    let session = SessionId::new("session");

    let input = |path: &str| SnapshotInput {
        path: path.into(),
        content: None,
    };
    assert!(
        store
            .save_checkpoint(&session, None, &blobs, &[input("../outside")], 1)
            .is_err()
    );
    assert!(
        store
            .save_checkpoint(&session, None, &blobs, &[input("x"), input("x")], 1)
            .is_err()
    );
    assert!(
        store
            .save_checkpoint(&session, None, &blobs, &[input(".git/config")], 1)
            .is_err()
    );
}
