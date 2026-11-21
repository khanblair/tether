//! Atomic runtime pack installation and on-device integrity checks.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use tokio_util::sync::CancellationToken;

use crate::pack::{
    PackEntry, PackError, PackFetcher, PackManifest, PackMount, validate_manifest, validate_name,
    validate_path,
};
use crate::pack_io::{HashingWriter, hash_file};

/// A verified pack installed below a caller-owned root.
#[derive(Debug, Clone)]
pub struct InstalledPack {
    root: PathBuf,
    entry: PackEntry,
}

/// Verified module bytes and guest settings for a pack program.
pub type RunSpecParts = (
    Vec<u8>,
    Vec<String>,
    Vec<PackMount>,
    BTreeMap<String, String>,
);

impl InstalledPack {
    /// Opens an installed pack without trusting its files.
    ///
    /// # Errors
    /// Returns a not-found, path, I/O, or manifest error.
    pub fn open(packs_root: &Path, id: &str, version: &str) -> Result<Self, PackError> {
        validate_name(id).map_err(|_| PackError::Path {
            id: id.into(),
            path: String::new(),
        })?;
        validate_name(version).map_err(|_| PackError::Path {
            id: id.into(),
            path: String::new(),
        })?;
        let root = packs_root.join(id).join(version);
        let metadata = fs::symlink_metadata(&root).map_err(|_| PackError::NotFound {
            id: id.into(),
            path: "manifest.json".into(),
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(PackError::Path {
                id: id.into(),
                path: "manifest.json".into(),
            });
        }
        let bytes = fs::read(root.join("manifest.json")).map_err(|_| PackError::NotFound {
            id: id.into(),
            path: "manifest.json".into(),
        })?;
        let manifest: PackManifest = serde_json::from_slice(&bytes)
            .map_err(|_| PackError::Manifest("installed manifest is invalid".into()))?;
        validate_manifest(&manifest)?;
        let entry = manifest
            .packs
            .into_iter()
            .find(|pack| pack.id == id && pack.version == version)
            .ok_or_else(|| PackError::NotFound {
                id: id.into(),
                path: "manifest.json".into(),
            })?;
        Ok(Self { root, entry })
    }

    /// Re-hashes every listed payload file.
    ///
    /// # Errors
    /// Returns a missing, resized, changed, or unsafe file error.
    pub fn verify(&self) -> Result<(), PackError> {
        for file in &self.entry.files {
            self.verify_file(file)?;
        }
        Ok(())
    }

    /// Returns the verified entry module bytes.
    ///
    /// # Errors
    /// Returns an integrity, missing-file, or I/O error.
    pub fn module_bytes(&self) -> Result<Vec<u8>, PackError> {
        self.verify()?;
        let path = self.file_path(&self.entry.entry)?;
        fs::read(path).map_err(|_| PackError::Io {
            id: self.entry.id.clone(),
            path: self.entry.entry.clone(),
        })
    }

    /// Builds the verified module and guest settings for a program invocation.
    ///
    /// # Errors
    /// Returns an integrity, missing-file, or invalid-pack error.
    pub fn run_spec_parts(&self, program: &str) -> Result<RunSpecParts, PackError> {
        self.verify()?;
        if !self.entry.programs.iter().any(|name| name == program) {
            return Err(PackError::NotFound {
                id: self.entry.id.clone(),
                path: program.to_owned(),
            });
        }
        let module = self.module_bytes()?;
        let mut argv_prefix = vec![program.to_owned()];
        argv_prefix.extend(self.entry.args_prefix.iter().cloned());
        Ok((
            module,
            argv_prefix,
            self.entry.mounts.clone(),
            self.entry.env.clone(),
        ))
    }

    /// Returns the pack metadata.
    #[must_use]
    pub fn entry(&self) -> &PackEntry {
        &self.entry
    }

    /// Returns the canonical installed pack folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn file_path(&self, path: &str) -> Result<PathBuf, PackError> {
        validate_path(&self.entry.id, path)?;
        Ok(self.root.join(path))
    }

    fn verify_file(&self, file: &crate::PackFile) -> Result<(), PackError> {
        let path = self.file_path(&file.path)?;
        let metadata = fs::symlink_metadata(&path).map_err(|_| PackError::NotFound {
            id: self.entry.id.clone(),
            path: file.path.clone(),
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(PackError::Path {
                id: self.entry.id.clone(),
                path: file.path.clone(),
            });
        }
        if metadata.len() != file.size_bytes {
            return Err(PackError::Size {
                id: self.entry.id.clone(),
                path: file.path.clone(),
            });
        }
        let digest = hash_file(&path).map_err(|_| PackError::Io {
            id: self.entry.id.clone(),
            path: file.path.clone(),
        })?;
        if digest != file.sha256 {
            return Err(PackError::HashMismatch {
                id: self.entry.id.clone(),
                path: file.path.clone(),
            });
        }
        Ok(())
    }
}

/// Installs a verified pack atomically below packs_root.
///
/// # Errors
/// Returns a manifest, fetch, cancellation, integrity, path, or I/O error.
pub fn install_pack(
    manifest: &PackManifest,
    pack_id: &str,
    fetcher: &dyn PackFetcher,
    packs_root: &Path,
    cancel: &CancellationToken,
) -> Result<InstalledPack, PackError> {
    validate_manifest(manifest)?;
    let pack = manifest
        .packs
        .iter()
        .find(|pack| pack.id == pack_id)
        .ok_or_else(|| PackError::NotFound {
            id: pack_id.into(),
            path: String::new(),
        })?;
    let root = prepare_root(packs_root, pack)?;
    let temporary = temporary_pack_path(&root, pack)?;
    let result = install_files(pack, fetcher, &temporary, cancel)
        .and_then(|()| write_manifest(pack, &temporary))
        .and_then(|()| {
            let installed = InstalledPack {
                root: temporary.clone(),
                entry: pack.clone(),
            };
            installed.verify()
        });
    if let Err(error) = result {
        drop(fs::remove_dir_all(&temporary));
        crate::pack_cleanup::remove_empty_directory(&root);
        return Err(error);
    }
    if let Err(error) = crate::pack_replace::replace_verified(pack, &root, &temporary) {
        drop(fs::remove_dir_all(&temporary));
        crate::pack_cleanup::remove_empty_directory(&root);
        return Err(error);
    }
    Ok(InstalledPack {
        root: root.join(&pack.version),
        entry: pack.clone(),
    })
}

/// Finds and opens an installed pack that provides program.
///
/// # Errors
/// Returns a not-found or installed-pack integrity error.
#[expect(
    clippy::excessive_nesting,
    reason = "the directory scan has two bounded path levels"
)]
pub fn program_for(packs_root: &Path, program: &str) -> Result<InstalledPack, PackError> {
    let entries = fs::read_dir(packs_root).map_err(|_| PackError::NotFound {
        id: program.into(),
        path: program.into(),
    })?;
    for id_entry in entries.flatten() {
        let id = id_entry.file_name().to_string_lossy().into_owned();
        if validate_name(&id).is_err() {
            continue;
        }
        let Ok(versions) = fs::read_dir(id_entry.path()) else {
            continue;
        };
        for version_entry in versions.flatten() {
            let version = version_entry.file_name().to_string_lossy().into_owned();
            if validate_name(&version).is_err() {
                continue;
            }
            let Ok(pack) = InstalledPack::open(packs_root, &id, &version) else {
                continue;
            };
            if pack.entry.programs.iter().any(|name| name == program) {
                return Ok(pack);
            }
        }
    }
    Err(PackError::NotFound {
        id: program.into(),
        path: program.into(),
    })
}

fn prepare_root(root: &Path, pack: &PackEntry) -> Result<PathBuf, PackError> {
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

fn temporary_pack_path(id_root: &Path, pack: &PackEntry) -> Result<PathBuf, PackError> {
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

fn install_files(
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
        let destination = temporary.join(&file.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|_| PackError::Io {
                id: pack.id.clone(),
                path: file.path.clone(),
            })?;
        }
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

fn write_manifest(pack: &PackEntry, temporary: &Path) -> Result<(), PackError> {
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
