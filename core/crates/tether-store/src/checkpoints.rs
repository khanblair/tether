//! Content-addressed snapshots taken before an edit batch.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

use rusqlite::{OptionalExtension, params};
use tether_types::{MessageId, SessionId};
use ulid::Ulid;

use crate::{Store, StoreError};

const MAX_BLOB_BYTES: u64 = 16 * 1024 * 1024;
const MAX_COMPRESSED_BYTES: u64 = MAX_BLOB_BYTES + 128 * 1024;

/// A private folder containing zstd-compressed, BLAKE3-addressed snapshots.
#[derive(Debug, Clone)]
pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    /// Creates a blob folder under app-private storage.
    ///
    /// # Errors
    /// Returns an error when the folder cannot be created or is a symlink.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        fs::create_dir_all(path)?;
        if fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "blob folder is a symlink",
            ));
        }
        Ok(Self {
            root: fs::canonicalize(path)?,
        })
    }

    /// Compresses bytes and stores them once per BLAKE3 hash.
    ///
    /// # Errors
    /// Returns an error for oversized content or a filesystem failure.
    pub fn put(&self, bytes: &[u8]) -> io::Result<String> {
        if bytes.len() as u64 > MAX_BLOB_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "checkpoint blob is too large",
            ));
        }
        let hash = blake3::hash(bytes).to_hex().to_string();
        let path = self.path_for(&hash)?;
        if path.try_exists()? {
            self.get(&hash)?;
            return Ok(hash);
        }
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "blob path has no parent")
        })?;
        fs::create_dir_all(parent)?;
        if fs::symlink_metadata(parent)?.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "blob folder is a symlink",
            ));
        }
        let compressed = zstd::stream::encode_all(bytes, 3)?;
        if compressed.len() as u64 > MAX_COMPRESSED_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "compressed blob is too large",
            ));
        }
        let temporary = path.with_extension(format!("tmp-{}", Ulid::new()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(&compressed)?;
            file.sync_all()?;
            fs::rename(&temporary, &path)?;
            File::open(parent)?.sync_all()
        })();
        if result.is_err() {
            remove_temporary(&temporary)?;
        }
        result?;
        Ok(hash)
    }

    /// Decompresses a saved blob and verifies its name against its bytes.
    ///
    /// # Errors
    /// Returns an error for malformed names, corrupt content, or a read failure.
    pub fn get(&self, hash: &str) -> io::Result<Vec<u8>> {
        let path = self.path_for(hash)?;
        if !fs::symlink_metadata(&path)?.file_type().is_file() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "blob is not a regular file",
            ));
        }
        if fs::metadata(&path)?.len() > MAX_COMPRESSED_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "checkpoint blob is too large",
            ));
        }
        let mut bytes = Vec::new();
        zstd::stream::read::Decoder::new(File::open(path)?)?
            .take(MAX_BLOB_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BLOB_BYTES || blake3::hash(&bytes).to_hex().as_str() != hash {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "checkpoint blob is corrupt",
            ));
        }
        Ok(bytes)
    }

    fn path_for(&self, hash: &str) -> io::Result<PathBuf> {
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid blob hash",
            ));
        }
        Ok(self.root.join(&hash[..2]).join(hash))
    }
}

fn remove_temporary(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// One path's content before an edit batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointFile {
    /// Workspace-relative file path.
    pub path: PathBuf,
    /// Content hash, absent when the file did not exist yet.
    pub blob_hash: Option<String>,
    /// Whether the file existed before the batch.
    pub existed: bool,
}

/// A saved edit batch checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    /// Unique checkpoint id.
    pub id: String,
    /// Session that owns the checkpoint.
    pub session_id: SessionId,
    /// Message preceding the batch, if any.
    pub message_id: Option<MessageId>,
    /// Time in Unix milliseconds.
    pub created_at_ms: i64,
    /// Paths and their prior content hashes.
    pub files: Vec<CheckpointFile>,
}

/// One workspace-relative path captured before an edit batch.
#[derive(Debug, Clone)]
pub struct SnapshotInput {
    /// Path of the file that will be edited.
    pub path: PathBuf,
    /// Prior bytes, or `None` if the file does not yet exist.
    pub content: Option<Vec<u8>>,
}

impl Store {
    /// Lists a session's checkpoints from newest to oldest.
    ///
    /// # Errors
    /// Returns an error if SQLite cannot read a checkpoint.
    pub fn list_checkpoints(&self, session_id: &SessionId) -> Result<Vec<Checkpoint>, StoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT id FROM checkpoints WHERE session_id = ?1 ORDER BY rowid DESC")?;
        let ids = statement
            .query_map([session_id.as_str()], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| {
                self.load_checkpoint(id)?
                    .ok_or(StoreError::MissingCheckpoint)
            })
            .collect()
    }

    /// Loads the target and every later checkpoint in reverse edit order.
    ///
    /// # Errors
    /// Returns an error if the checkpoint is absent, belongs to another session, or SQLite fails.
    pub fn checkpoint_chain(
        &self,
        session_id: &SessionId,
        checkpoint_id: &str,
    ) -> Result<Vec<Checkpoint>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT id FROM checkpoints WHERE session_id = ?1 AND rowid >= \
             (SELECT rowid FROM checkpoints WHERE id = ?2 AND session_id = ?1) ORDER BY rowid DESC",
        )?;
        let ids = statement
            .query_map(params![session_id.as_str(), checkpoint_id], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        if ids.is_empty() {
            return Err(StoreError::MissingCheckpoint);
        }
        ids.iter()
            .map(|id| {
                self.load_checkpoint(id)?
                    .ok_or(StoreError::MissingCheckpoint)
            })
            .collect()
    }

    /// Hides the checkpoint's assistant call and every following message from future context.
    /// The call is included because its tool results will be rewound with it.
    ///
    /// # Errors
    /// Returns an error if its message is absent, belongs to another session, or SQLite fails.
    pub fn rewind_conversation(
        &mut self,
        session_id: &SessionId,
        message_id: &MessageId,
    ) -> Result<(), StoreError> {
        self.validate_rewind_anchor(session_id, message_id)?;
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE messages SET rewound = 1 WHERE session_id = ?1 AND seq >= \
             (SELECT seq FROM messages WHERE id = ?2 AND session_id = ?1)",
            params![session_id.as_str(), message_id.as_str()],
        )?;
        if changed == 0 {
            return Err(StoreError::MissingCheckpointMessage);
        }
        tx.commit()?;
        Ok(())
    }

    /// Checks that a conversation rewind anchor belongs to the given session.
    ///
    /// # Errors
    /// Returns an error when the anchor is absent, belongs to another session, or SQLite fails.
    pub fn validate_rewind_anchor(
        &self,
        session_id: &SessionId,
        message_id: &MessageId,
    ) -> Result<(), StoreError> {
        let exists = self
            .conn
            .query_row(
                "SELECT 1 FROM messages WHERE id = ?1 AND session_id = ?2",
                params![message_id.as_str(), session_id.as_str()],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !exists {
            return Err(StoreError::MissingCheckpointMessage);
        }
        Ok(())
    }

    /// Saves each supplied file's prior content before an edit batch changes it.
    /// The engine captures bytes through its workspace guard. Paths must be
    /// unique and workspace-relative. Blob files are written
    /// before one database transaction records the checkpoint.
    ///
    /// # Errors
    /// Returns an error for invalid paths, unreadable files, or database failure.
    #[expect(
        clippy::too_many_arguments,
        reason = "checkpoint metadata and blob input are kept explicit at the store boundary"
    )]
    pub fn save_checkpoint(
        &mut self,
        session_id: &SessionId,
        message_id: Option<&MessageId>,
        blobs: &BlobStore,
        snapshots: &[SnapshotInput],
        now_ms: i64,
    ) -> Result<Checkpoint, StoreError> {
        if snapshots.is_empty() {
            return Err(StoreError::InvalidCheckpointPath);
        }
        let files = checkpoint_files(blobs, snapshots)?;
        let id = Ulid::new().to_string();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO checkpoints (id, session_id, message_id, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, session_id.as_str(), message_id.map(MessageId::as_str), now_ms],
        )?;
        for file in &files {
            tx.execute(
                "INSERT INTO checkpoint_files (checkpoint_id, path, blob_hash, existed) VALUES (?1, ?2, ?3, ?4)",
                params![id, file.path.to_string_lossy(), file.blob_hash.as_deref(), file.existed],
            )?;
        }
        tx.commit()?;
        Ok(Checkpoint {
            id,
            session_id: session_id.clone(),
            message_id: message_id.cloned(),
            created_at_ms: now_ms,
            files,
        })
    }

    /// Loads one checkpoint and its ordered file snapshots.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the checkpoint.
    pub fn load_checkpoint(&self, id: &str) -> Result<Option<Checkpoint>, StoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT session_id, message_id, created_at FROM checkpoints WHERE id = ?1")?;
        let row = statement.query_row([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)?,
            ))
        });
        let (session_id, message_id, created_at_ms) = match row {
            Ok(row) => row,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut statement = self.conn.prepare(
            "SELECT path, blob_hash, existed FROM checkpoint_files WHERE checkpoint_id = ?1 ORDER BY path",
        )?;
        let files = statement
            .query_map([id], |row| {
                Ok(CheckpointFile {
                    path: PathBuf::from(row.get::<_, String>(0)?),
                    blob_hash: row.get(1)?,
                    existed: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(Checkpoint {
            id: id.to_owned(),
            session_id: SessionId::new(session_id),
            message_id: message_id.map(MessageId::new),
            created_at_ms,
            files,
        }))
    }
}

fn checkpoint_files(
    blobs: &BlobStore,
    snapshots: &[SnapshotInput],
) -> Result<Vec<CheckpointFile>, StoreError> {
    let mut seen = HashSet::new();
    let mut files = Vec::with_capacity(snapshots.len());
    for snapshot in snapshots {
        if !valid_path(&snapshot.path) || !seen.insert(&snapshot.path) {
            return Err(StoreError::InvalidCheckpointPath);
        }
        let blob_hash = snapshot
            .content
            .as_deref()
            .map(|content| blobs.put(content))
            .transpose()?;
        files.push(CheckpointFile {
            path: snapshot.path.clone(),
            existed: blob_hash.is_some(),
            blob_hash,
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn valid_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(name) if name != ".git"))
}

#[cfg(test)]
mod tests;
