use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::LinuxError;
use crate::error::io_error;
use crate::extract::IndexedFile;
use crate::manifest::{RootfsImage, is_lower_hex, sha256_file};

pub(crate) fn is_cached(path: &Path, image: &RootfsImage) -> Result<bool, LinuxError> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(false);
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Ok(false);
    }
    let size = metadata.len();
    if size != image.size_bytes || !is_lower_hex(&image.sha256, 64) {
        return Ok(false);
    }
    Ok(sha256_file(path)? == image.sha256)
}

pub(crate) fn check_index(
    rootfs: &Path,
    files: &[IndexedFile],
) -> Result<(usize, usize), LinuxError> {
    let mut missing = 0;
    let mut changed = 0;
    for file in files {
        match fs::symlink_metadata(rootfs.join(&file.path)) {
            Ok(metadata) if metadata.is_file() => {
                changed += usize::from(file_changed(rootfs, file)?);
            }
            _ => missing += 1,
        }
    }
    Ok((missing, changed))
}

fn file_changed(rootfs: &Path, file: &IndexedFile) -> Result<bool, LinuxError> {
    let path = rootfs.join(&file.path);
    let digest = sha256_file(&path)?;
    let size = fs::symlink_metadata(path)
        .map_err(|_| io_error("inspect root filesystem file"))?
        .len();
    Ok(size != file.size || digest != file.sha256)
}

/// Stops tracking indexed files that the package manager changed.
///
/// A file is removed only when it still exists as a regular file with a different
/// size or digest. Missing and unchanged files stay indexed, so repair still restores them.
/// On error, `files` is left empty, so callers must discard it.
///
/// # Errors
/// Returns a storage error when a file cannot be read.
pub(crate) fn prune_changed(
    rootfs: &Path,
    files: &mut Vec<IndexedFile>,
) -> Result<usize, LinuxError> {
    let mut kept = Vec::with_capacity(files.len());
    let mut removed = 0;
    for file in std::mem::take(files) {
        let is_regular =
            fs::symlink_metadata(rootfs.join(&file.path)).is_ok_and(|metadata| metadata.is_file());
        if is_regular && file_changed(rootfs, &file)? {
            removed += 1;
        } else {
            kept.push(file);
        }
    }
    *files = kept;
    Ok(removed)
}

pub(crate) fn file_missing_or_changed(rootfs: &Path, file: &IndexedFile) -> bool {
    match fs::symlink_metadata(rootfs.join(&file.path)) {
        Ok(metadata) if metadata.is_file() => {
            metadata.len() != file.size
                || sha256_file(&rootfs.join(&file.path)).map_or(true, |hash| hash != file.sha256)
        }
        _ => true,
    }
}

pub(crate) fn touch(path: &Path) -> Result<(), LinuxError> {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(LinuxError::Path("Linux resolver is a symlink".into()));
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(|_| ())
        .map_err(|_| io_error("create Linux resolver file"))
}

/// The guest's `/etc/hosts` content.
///
/// There is no `::1` line on purpose. A server that listens on `localhost` must land on
/// `127.0.0.1`, because the app reaches servers there.
pub(crate) const LOOPBACK_HOSTS: &[u8] = b"127.0.0.1\tlocalhost localhost.localdomain\n";

static HOSTS_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Makes sure the hosts file holds exactly `LOOPBACK_HOSTS`.
///
/// The file is replaced by a rename from a new temp file in the same folder. It is
/// never truncated in place, so a bind that happens during the write never sees an
/// empty file, and a file the guest made unreadable (mode 000) is still replaced.
pub(crate) fn ensure_hosts(path: &Path) -> Result<(), LinuxError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(LinuxError::Path("Linux hosts file is a symlink".into()));
        }
        Ok(metadata) if metadata.is_dir() => {
            return Err(LinuxError::Path("Linux hosts file is not a file".into()));
        }
        _ => {}
    }
    if fs::read(path).is_ok_and(|bytes| bytes == LOOPBACK_HOSTS) {
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| io_error("write Linux hosts file"))?;
    let counter = HOSTS_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!("hosts.tmp-{}-{counter}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|_| io_error("write Linux hosts file"))?;
    let replaced = file
        .write_all(LOOPBACK_HOSTS)
        .and_then(|()| fs::rename(&temp, path));
    if replaced.is_err() {
        fs::remove_file(&temp).ok();
    }
    replaced.map_err(|_| io_error("write Linux hosts file"))
}

pub(crate) fn remove_path(path: &Path) -> Result<(), LinuxError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)
        }
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
    .map_err(|_| io_error("remove Linux staging path"))
}

pub(crate) fn error_to_io(_error: io::Error, message: &str) -> LinuxError {
    io_error(message)
}
