//! Platform-folder sync adapter and manifest persistence.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use tether_memory::vault::sync::{SyncManifest, VaultFolder, sync_folders};

use crate::vault::{
    VaultFolderPlatform, VaultFolderPlatformError, VaultRuntimeError, VaultSyncReportView,
};
use crate::vault_manifest::{Bytes, Observed, Snapshot};

pub(crate) fn sync(
    root: &Path,
    platform: Arc<dyn VaultFolderPlatform>,
) -> Result<VaultSyncReportView, VaultRuntimeError> {
    let private = PrivateFolder::new(root.to_path_buf());
    let shared = PlatformFolder(platform);
    let private_snapshot = private.snapshot()?;
    let shared_snapshot = shared.snapshot()?;
    let observed = crate::vault_manifest::load(root)?;
    let private_view = SnapshotFolder::new(
        &private,
        private_snapshot.clone(),
        shared_snapshot.clone(),
        &observed,
        Side::Private,
    );
    let shared_view = SnapshotFolder::new(
        &shared,
        private_snapshot,
        shared_snapshot,
        &observed,
        Side::Shared,
    );
    let report = sync_folders(&private_view, &shared_view, &mut SyncManifest::default())
        .map_err(|error| map_sync_error(&error))?;
    let current_private = private.snapshot()?;
    let current_shared = shared.snapshot()?;
    crate::vault_manifest::save(root, &current_private, &current_shared)?;
    Ok(VaultSyncReportView {
        copied_to_private: report.copied_to_private,
        copied_to_shared: report.copied_to_shared,
        conflicts: report.conflicts,
    })
}

struct PrivateFolder {
    root: PathBuf,
}

impl PrivateFolder {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn snapshot(&self) -> Result<Snapshot, VaultRuntimeError> {
        let mut paths = Vec::new();
        collect_markdown(&self.root, &self.root, &mut paths)?;
        paths
            .into_iter()
            .try_fold(BTreeMap::new(), |mut files, path| {
                let bytes = read_private(&self.root, &path)?;
                files.insert(path, bytes);
                Ok(files)
            })
    }
}

impl VaultFolder for PrivateFolder {
    fn list(&self) -> io::Result<Vec<String>> {
        let mut paths = Vec::new();
        collect_markdown_io(&self.root, &self.root, &mut paths)?;
        Ok(paths)
    }

    fn read(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
        read_private_io(&self.root, path)
    }

    fn write(&self, path: &str, bytes: &[u8]) -> io::Result<()> {
        let target = safe_target(&self.root, path)?;
        let parent = target.parent().ok_or_else(invalid_path)?;
        fs::create_dir_all(parent)?;
        fs::write(target, bytes)
    }
}

struct PlatformFolder(Arc<dyn VaultFolderPlatform>);

impl PlatformFolder {
    fn snapshot(&self) -> Result<Snapshot, VaultRuntimeError> {
        self.0
            .list()
            .map_err(|_| VaultRuntimeError::Unavailable)?
            .into_iter()
            .try_fold(BTreeMap::new(), |mut files, path| {
                validate_sync_path(&path)?;
                let bytes = self
                    .0
                    .read(path.clone())
                    .map_err(|_| VaultRuntimeError::Unavailable)?;
                files.insert(path, bytes);
                Ok(files)
            })
    }
}

impl VaultFolder for PlatformFolder {
    fn list(&self) -> io::Result<Vec<String>> {
        self.0.list().map_err(|error| platform_io(&error))
    }

    fn read(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
        self.0
            .read(path.to_owned())
            .map_err(|error| platform_io(&error))
    }

    fn write(&self, path: &str, bytes: &[u8]) -> io::Result<()> {
        self.0
            .write(path.to_owned(), bytes.to_vec())
            .map_err(|error| platform_io(&error))
    }
}

struct SnapshotFolder<'a, F> {
    folder: &'a F,
    private: Snapshot,
    shared: Snapshot,
    observed: &'a Observed,
    side: Side,
}

impl<'a, F: VaultFolder> SnapshotFolder<'a, F> {
    fn new(
        folder: &'a F,
        private: Snapshot,
        shared: Snapshot,
        observed: &'a Observed,
        side: Side,
    ) -> Self {
        Self {
            folder,
            private,
            shared,
            observed,
            side,
        }
    }

    fn visible(&self, path: &str) -> Option<Vec<u8>> {
        let private_now = self.private.get(path).cloned().flatten();
        let shared_now = self.shared.get(path).cloned().flatten();
        let value = match self.side {
            Side::Private => private_now.clone(),
            Side::Shared => shared_now.clone(),
        };
        let Some((private_old, shared_old)) = self.observed.get(path) else {
            return value;
        };
        match self.side {
            Side::Private if private_now == *private_old && shared_now != *shared_old => None,
            Side::Shared if private_now != *private_old && shared_now == *shared_old => None,
            _ => value,
        }
    }
}

impl<F: VaultFolder> VaultFolder for SnapshotFolder<'_, F> {
    fn list(&self) -> io::Result<Vec<String>> {
        let current = match self.side {
            Side::Private => &self.private,
            Side::Shared => &self.shared,
        };
        Ok(current.keys().cloned().collect())
    }

    fn read(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
        Ok(self.visible(path))
    }

    fn write(&self, path: &str, bytes: &[u8]) -> io::Result<()> {
        self.folder.write(path, bytes)
    }
}

#[derive(Clone, Copy)]
enum Side {
    Private,
    Shared,
}

fn collect_markdown(
    root: &Path,
    folder: &Path,
    paths: &mut Vec<String>,
) -> Result<(), VaultRuntimeError> {
    collect_markdown_io(root, folder, paths).map_err(|_| VaultRuntimeError::Unavailable)
}

fn collect_markdown_io(root: &Path, folder: &Path, paths: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink()
            || entry.file_name() == ".tether-sync"
            || entry.file_name() == ".obsidian"
        {
            continue;
        }
        if metadata.is_dir() {
            collect_markdown_io(root, &path, paths)?;
        } else if metadata.is_file()
            && path.extension().and_then(|value| value.to_str()) == Some("md")
        {
            paths.push(relative(root, &path)?);
        }
    }
    paths.sort();
    Ok(())
}

fn read_private(root: &Path, path: &str) -> Result<Bytes, VaultRuntimeError> {
    read_private_io(root, path).map_err(|_| VaultRuntimeError::Unavailable)
}

fn read_private_io(root: &Path, path: &str) -> io::Result<Bytes> {
    let target = safe_target(root, path)?;
    match fs::read(target) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn safe_target(root: &Path, path: &str) -> io::Result<PathBuf> {
    let input = Path::new(path);
    if input
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(invalid_path());
    }
    let mut target = root.to_path_buf();
    for component in input.components() {
        let Component::Normal(name) = component else {
            return Err(invalid_path());
        };
        target.push(name);
        if fs::symlink_metadata(&target).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err(invalid_path());
        }
    }
    Ok(target)
}

fn relative(root: &Path, path: &Path) -> io::Result<String> {
    path.strip_prefix(root)
        .map_err(|_| invalid_path())
        .map(|value| value.to_string_lossy().replace('\\', "/"))
}

fn invalid_path() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid vault path")
}

fn platform_io(error: &VaultFolderPlatformError) -> io::Error {
    let _ = error;
    io::Error::other("vault platform failure")
}

fn map_sync_error(error: &tether_memory::vault::sync::SyncError) -> VaultRuntimeError {
    match error {
        tether_memory::vault::sync::SyncError::InvalidPath(_) => VaultRuntimeError::InvalidPath,
        tether_memory::vault::sync::SyncError::Io(_) => VaultRuntimeError::Unavailable,
    }
}

fn validate_sync_path(path: &str) -> Result<(), VaultRuntimeError> {
    let value = Path::new(path);
    if value.extension().and_then(|ext| ext.to_str()) != Some("md")
        || path.starts_with('.')
        || path.contains("/.obsidian/")
        || !value
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(VaultRuntimeError::InvalidPath);
    }
    Ok(())
}
