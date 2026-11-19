//! The secret store: one file per secret, wrapped by the platform.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{PlatformError, Secret, SecretRef, SecretsError, SecretsPlatform};

/// The first bytes of every secret file.
const MAGIC: &[u8; 4] = b"TSEC";

/// The file format version after the magic bytes.
const FORMAT_VERSION: u8 = 1;

/// Length of the header: magic plus version.
const HEADER_LEN: usize = MAGIC.len() + 1;

impl From<PlatformError> for SecretsError {
    fn from(error: PlatformError) -> Self {
        Self::Platform {
            message: error.message,
        }
    }
}

/// Stores secrets as `<dir>/<reference>.bin`. The file holds a small header and the bytes
/// the platform wrapped, never the secret itself.
pub struct SecretStore {
    dir: PathBuf,
    platform: Arc<dyn SecretsPlatform>,
}

impl std::fmt::Debug for SecretStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretStore")
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

impl SecretStore {
    /// Creates a store that keeps its files in `dir`. The folder is created on the first `put`.
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>, platform: Arc<dyn SecretsPlatform>) -> Self {
        Self {
            dir: dir.into(),
            platform,
        }
    }

    /// Wraps `secret` with the platform key and writes it. An existing secret with the
    /// same reference is replaced, and a crash leaves either the old file or the new one.
    ///
    /// # Errors
    /// Returns an error when the platform cannot wrap or the file cannot be written.
    pub fn put(&self, reference: &SecretRef, secret: &Secret) -> Result<(), SecretsError> {
        let wrapped = self
            .platform
            .wrap(secret.expose(), reference.as_str().as_bytes())?;
        let mut bytes = Vec::with_capacity(HEADER_LEN + wrapped.len());
        bytes.extend_from_slice(MAGIC);
        bytes.push(FORMAT_VERSION);
        bytes.extend_from_slice(&wrapped);
        fs::create_dir_all(&self.dir)
            .map_err(|source| io_error("create the folder for", source))?;
        write_atomic(&self.path(reference), &bytes)
    }

    /// Reads and unwraps a secret, or returns `None` when there is none.
    ///
    /// # Errors
    /// Returns an error when the file is damaged or the platform cannot unwrap it.
    pub fn get(&self, reference: &SecretRef) -> Result<Option<Secret>, SecretsError> {
        let bytes = match fs::read(self.path(reference)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(io_error("read", source)),
        };
        let wrapped = strip_header(&bytes).ok_or_else(|| SecretsError::Corrupt {
            reference: reference.as_str().to_owned(),
        })?;
        let plaintext = self
            .platform
            .unwrap(wrapped, reference.as_str().as_bytes())?;
        Ok(Some(Secret::new(plaintext)))
    }

    /// True when a secret file exists for the reference.
    ///
    /// # Errors
    /// Returns an error when the folder cannot be read.
    pub fn exists(&self, reference: &SecretRef) -> Result<bool, SecretsError> {
        match fs::metadata(self.path(reference)) {
            Ok(meta) => Ok(meta.is_file()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(source) => Err(io_error("check", source)),
        }
    }

    /// Deletes a secret. Deleting one that does not exist is not an error.
    ///
    /// # Errors
    /// Returns an error when the file exists but cannot be removed.
    pub fn delete(&self, reference: &SecretRef) -> Result<(), SecretsError> {
        match fs::remove_file(self.path(reference)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(io_error("remove", source)),
        }
    }

    fn path(&self, reference: &SecretRef) -> PathBuf {
        self.dir.join(format!("{}.bin", reference.as_str()))
    }
}

fn io_error(action: &'static str, source: io::Error) -> SecretsError {
    SecretsError::Io { action, source }
}

/// The wrapped bytes after a valid header, or `None` when the header is wrong.
fn strip_header(bytes: &[u8]) -> Option<&[u8]> {
    let (header, wrapped) = bytes.split_at_checked(HEADER_LEN)?;
    (header.starts_with(MAGIC) && header.last() == Some(&FORMAT_VERSION)).then_some(wrapped)
}

/// Writes to a temporary file in the same folder, then renames it over the target, so a
/// crash never leaves a half-written secret.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), SecretsError> {
    let temp = path.with_extension("bin.tmp");
    let result = write_private_file(&temp, bytes).and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        // The write error is the one worth reporting; a leftover temp file is harmless.
        drop(fs::remove_file(&temp));
    }
    result.map_err(|source| io_error("write", source))
}

fn write_private_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = open_private(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(unix)]
fn open_private(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn open_private(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
}
