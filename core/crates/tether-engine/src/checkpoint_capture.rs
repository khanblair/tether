//! Capture prior file bytes through the workspace guard before an edit batch.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use tether_store::{BlobStore, Checkpoint, SnapshotInput, Store, StoreError};
use tether_types::{MessageId, SessionId};
use tether_workspace::PathGuard;

const MAX_SNAPSHOT_BYTES: u64 = 16 * 1024 * 1024;

/// Captures the named paths and saves one checkpoint before any edit is applied.
/// Callers must pass every path the batch may change.
///
/// # Errors
/// Rejects escaped, duplicate, directory, or oversized paths and storage failures.
#[expect(
    clippy::too_many_arguments,
    reason = "checkpoint capture needs its session, workspace, store, and paths"
)]
pub fn capture_checkpoint(
    store: &mut Store,
    session_id: &SessionId,
    message_id: Option<&MessageId>,
    guard: &PathGuard,
    blobs: &BlobStore,
    paths: &[PathBuf],
    now_ms: i64,
) -> Result<Checkpoint, StoreError> {
    let mut snapshots = Vec::with_capacity(paths.len());
    for relative in paths {
        if !safe_path(relative) {
            return Err(StoreError::InvalidCheckpointPath);
        }
        let path = guard.resolve(relative)?;
        let content = read_snapshot(&path)?;
        snapshots.push(SnapshotInput {
            path: relative.clone(),
            content,
        });
    }
    store.save_checkpoint(session_id, message_id, blobs, &snapshots, now_ms)
}

fn read_snapshot(path: &Path) -> Result<Option<Vec<u8>>, StoreError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => return Err(StoreError::InvalidCheckpointPath),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if metadata.len() > MAX_SNAPSHOT_BYTES {
        return Err(oversized_snapshot());
    }
    let mut content = Vec::new();
    File::open(path)?
        .take(MAX_SNAPSHOT_BYTES + 1)
        .read_to_end(&mut content)?;
    if content.len() as u64 > MAX_SNAPSHOT_BYTES {
        return Err(oversized_snapshot());
    }
    Ok(Some(content))
}

fn oversized_snapshot() -> StoreError {
    io::Error::new(io::ErrorKind::InvalidInput, "snapshot is too large").into()
}

fn safe_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(name) if name != ".git"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn capture_records_bytes_before_a_batch_changes_them() {
        let root = tempfile::tempdir().unwrap();
        let database = root.path().join("sessions.db");
        drop(Store::open(&database).unwrap());
        let conn = rusqlite::Connection::open(&database).unwrap();
        conn.execute_batch(
            "INSERT INTO projects (id, name, path, created_at) VALUES ('project', 'fixture', 'fixture', 1);
             INSERT INTO sessions (id, project_id, provider, model_id, mode, status, created_at, updated_at)
             VALUES ('session', 'project', 'openai', 'fixture', 'ask', 'idle', 1, 1);",
        ).unwrap();
        drop(conn);
        let mut store = Store::open(&database).unwrap();
        let workspace = root.path().join("workspace");
        fs::create_dir(&workspace).unwrap();
        fs::write(workspace.join("note.txt"), b"before").unwrap();
        let guard = PathGuard::new(&workspace).unwrap();
        let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
        let checkpoint = capture_checkpoint(
            &mut store,
            &SessionId::new("session"),
            None,
            &guard,
            &blobs,
            &["note.txt".into(), "new.txt".into()],
            2,
        )
        .unwrap();
        fs::write(workspace.join("note.txt"), b"after").unwrap();
        assert_eq!(checkpoint.files.len(), 2);
        assert_eq!(
            blobs
                .get(checkpoint.files[1].blob_hash.as_deref().unwrap())
                .unwrap(),
            b"before"
        );
        assert!(!checkpoint.files[0].existed);
        assert_eq!(
            store.load_checkpoint(&checkpoint.id).unwrap().unwrap(),
            checkpoint
        );
    }

    #[test]
    fn capture_rejects_escaped_and_git_paths() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        fs::create_dir(&workspace).unwrap();
        let guard = PathGuard::new(&workspace).unwrap();
        let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
        let mut store = Store::open_in_memory().unwrap();
        let session = SessionId::new("fixture");
        for path in ["../outside", ".git/config", "./note.txt"] {
            assert!(
                capture_checkpoint(
                    &mut store,
                    &session,
                    None,
                    &guard,
                    &blobs,
                    &[path.into()],
                    1
                )
                .is_err()
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn capture_rejects_symlinked_file() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        fs::create_dir(&workspace).unwrap();
        fs::write(root.path().join("outside"), b"private").unwrap();
        symlink(root.path().join("outside"), workspace.join("link")).unwrap();
        let guard = PathGuard::new(&workspace).unwrap();
        let blobs = BlobStore::open(root.path().join("blobs")).unwrap();
        let mut store = Store::open_in_memory().unwrap();
        assert!(
            capture_checkpoint(
                &mut store,
                &SessionId::new("fixture"),
                None,
                &guard,
                &blobs,
                &["link".into()],
                1
            )
            .is_err()
        );
    }
}
