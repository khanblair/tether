//! Atomic replacement of verified runtime packs.

use std::fs;
use std::path::Path;

use crate::{PackEntry, PackError};

pub(super) fn replace_verified(
    pack: &PackEntry,
    id_root: &Path,
    temporary: &Path,
) -> Result<(), PackError> {
    let target = id_root.join(&pack.version);
    let backup = id_root.join(format!(".{}.old", pack.version));
    let had_target = fs::symlink_metadata(&target).is_ok();
    if had_target {
        let metadata = fs::symlink_metadata(&target).map_err(|_| PackError::Path {
            id: pack.id.clone(),
            path: String::new(),
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(PackError::Path {
                id: pack.id.clone(),
                path: String::new(),
            });
        }
        drop(fs::remove_dir_all(&backup));
        fs::rename(&target, &backup).map_err(|_| PackError::Io {
            id: pack.id.clone(),
            path: String::new(),
        })?;
    }
    if fs::rename(temporary, &target).is_err() {
        if had_target {
            drop(fs::rename(&backup, &target));
        }
        return Err(PackError::Io {
            id: pack.id.clone(),
            path: String::new(),
        });
    }
    if had_target {
        drop(fs::remove_dir_all(backup));
    }
    Ok(())
}
