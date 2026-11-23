use std::fs::{self, OpenOptions};
use std::io;
use std::path::Path;

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
