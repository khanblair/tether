//! Staging, layout, and manifest writes for runtime pack installs.

use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use tokio_util::sync::CancellationToken;

use crate::pack::{PackEntry, PackError, PackFetcher, PackManifest, validate_path};
use crate::pack_io::HashingWriter;

pub(crate) fn prepare_root(root: &Path, pack: &PackEntry) -> Result<PathBuf, PackError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(PackError::Path {
                id: pack.id.clone(),
                path: String::new(),
            });
        }
        Ok(_) => {}
        Err(_) => fs::create_dir_all(root).map_err(|_| PackError::Io {
            id: pack.id.clone(),
            path: String::new(),
        })?,
    }
    let id_root = root.join(&pack.id);
    if let Ok(metadata) = fs::symlink_metadata(&id_root) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(PackError::Path {
                id: pack.id.clone(),
                path: String::new(),
            });
        }
    } else {
        fs::create_dir(&id_root).map_err(|_| PackError::Io {
            id: pack.id.clone(),
            path: String::new(),
        })?;
    }
    Ok(id_root)
}

pub(crate) fn temporary_pack_path(id_root: &Path, pack: &PackEntry) -> Result<PathBuf, PackError> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for _ in 0..32 {
        let suffix = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = id_root.join(format!(".{}.tmp-{suffix:x}", pack.version));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(_) => {
                return Err(PackError::Io {
                    id: pack.id.clone(),
                    path: String::new(),
                });
            }
        }
    }
    Err(PackError::Io {
        id: pack.id.clone(),
        path: String::new(),
    })
}

pub(crate) fn install_files(
    pack: &PackEntry,
    fetcher: &dyn PackFetcher,
    temporary: &Path,
    cancel: &CancellationToken,
) -> Result<(), PackError> {
    for file in &pack.files {
        if cancel.is_cancelled() {
            return Err(PackError::Cancelled {
                id: pack.id.clone(),
                path: file.path.clone(),
            });
        }
        validate_path(&pack.id, &file.path)?;
        create_parent_dirs(temporary, &file.path).map_err(|_| PackError::Io {
            id: pack.id.clone(),
            path: file.path.clone(),
        })?;
        let destination = temporary.join(&file.path);
        let file_handle = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|_| PackError::Io {
                id: pack.id.clone(),
                path: file.path.clone(),
            })?;
        let mut writer = HashingWriter::new(file_handle, file.size_bytes);
        let fetch_result = fetcher.fetch(pack, file, &mut writer, cancel);
        if cancel.is_cancelled() {
            return Err(PackError::Cancelled {
                id: pack.id.clone(),
                path: file.path.clone(),
            });
        }
        if writer.exceeded() {
            return Err(PackError::Size {
                id: pack.id.clone(),
                path: file.path.clone(),
            });
        }
        fetch_result?;
        writer.finish(pack, file)?;
    }
    Ok(())
}

pub(crate) fn verified_root(
    packs_root: &Path,
    id: &str,
    version: &str,
) -> Result<PathBuf, PackError> {
    let id_root = packs_root.join(id);
    let root = id_root.join(version);
    check_directory(packs_root, id, "")?;
    check_directory(&id_root, id, "")?;
    check_directory(&root, id, "manifest.json")?;
    Ok(root)
}

fn check_directory(path: &Path, id: &str, label: &str) -> Result<(), PackError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| PackError::NotFound {
        id: id.into(),
        path: label.into(),
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PackError::Path {
            id: id.into(),
            path: label.into(),
        });
    }
    Ok(())
}

fn create_parent_dirs(root: &Path, relative: &str) -> io::Result<()> {
    let mut current = root.to_path_buf();
    let mut parts = relative.rsplitn(2, '/');
    let Some(parent) = parts.nth(1) else {
        return Ok(());
    };
    for component in parent.split('/') {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unsafe pack folder",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&current)?,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

pub(crate) fn write_manifest(pack: &PackEntry, temporary: &Path) -> Result<(), PackError> {
    let manifest = PackManifest {
        format: 1,
        packs: vec![pack.clone()],
    };
    let bytes = serde_json::to_vec(&manifest)
        .map_err(|_| PackError::Manifest("manifest serialization failed".into()))?;
    fs::write(temporary.join("manifest.json"), bytes).map_err(|_| PackError::Io {
        id: pack.id.clone(),
        path: "manifest.json".into(),
    })
}
