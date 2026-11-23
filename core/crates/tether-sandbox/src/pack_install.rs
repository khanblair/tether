//! Atomic runtime pack installation and on-device integrity checks.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use crate::pack::{
    PackEntry, PackError, PackFetcher, PackManifest, PackMount, validate_manifest, validate_name,
    validate_path,
};
use crate::pack_io::hash_file;
use crate::pack_layout::{
    install_files, prepare_root, temporary_pack_path, verified_root, write_manifest,
};

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
        let root = verified_root(packs_root, id, version)?;
        let manifest_path = root.join("manifest.json");
        let metadata = fs::symlink_metadata(&manifest_path).map_err(|_| PackError::NotFound {
            id: id.into(),
            path: "manifest.json".into(),
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(PackError::Path {
                id: id.into(),
                path: "manifest.json".into(),
            });
        }
        let bytes = fs::read(manifest_path).map_err(|_| PackError::Io {
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
        let mut current = self.root.clone();
        let components = path.split('/').collect::<Vec<_>>();
        for (index, component) in components.iter().enumerate() {
            current.push(component);
            let is_intermediate = index + 1 < components.len();
            self.check_component(&current, path, is_intermediate)?;
        }
        Ok(current)
    }

    fn check_component(
        &self,
        current: &Path,
        path: &str,
        is_intermediate: bool,
    ) -> Result<(), PackError> {
        let metadata = fs::symlink_metadata(current).map_err(|_| self.not_found(path))?;
        if metadata.file_type().is_symlink() || (is_intermediate && !metadata.is_dir()) {
            return Err(self.path_error(path));
        }
        Ok(())
    }

    fn not_found(&self, path: &str) -> PackError {
        PackError::NotFound {
            id: self.entry.id.clone(),
            path: path.into(),
        }
    }

    fn path_error(&self, path: &str) -> PackError {
        PackError::Path {
            id: self.entry.id.clone(),
            path: path.into(),
        }
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
        let Ok(metadata) = fs::symlink_metadata(id_entry.path()) else {
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
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
