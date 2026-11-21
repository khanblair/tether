use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::LinuxError;
use crate::error::io_error;
use crate::extract::{ExtractReport, IndexedFile, extract_archive, repair_archive};
use crate::manifest::{RootfsFetcher, RootfsImage, hex_digest, is_lower_hex, sha256_file};

/// The observable state of the Alpine root filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootfsStatus {
    /// No root filesystem has been installed.
    NotInstalled,
    /// The archive is extracted but base packages are not ready.
    Installed {
        /// Alpine release version.
        alpine_version: String,
    },
    /// The extracted index has missing or changed files.
    NeedsRepair {
        /// Indexed files that are absent.
        missing: usize,
        /// Indexed files whose size or digest changed.
        changed: usize,
    },
    /// The root filesystem and base packages are ready.
    Ready {
        /// Alpine release version.
        alpine_version: String,
    },
}

/// Receives root filesystem setup progress.
pub trait RootfsProgress {
    /// Reports downloaded archive bytes.
    fn downloading(&mut self, _bytes: u64) {}
    /// Reports that archive verification started.
    fn verifying(&mut self) {}
    /// Reports the number of archive entries extracted.
    fn extracting(&mut self, _entries: usize) {}
    /// Reports successful setup completion.
    fn done(&mut self) {}
}

/// A progress sink that ignores all updates.
#[derive(Debug, Default)]
pub struct NoopProgress;

impl RootfsProgress for NoopProgress {}

/// Owns the on-device Linux root filesystem and its repair index.
#[derive(Debug, Clone)]
pub struct RootfsManager {
    linux_root: PathBuf,
}

impl RootfsManager {
    /// Creates a manager rooted at the private `linux` directory.
    #[must_use]
    pub fn new(linux_root: impl Into<PathBuf>) -> Self {
        Self {
            linux_root: linux_root.into(),
        }
    }

    /// Installs an image from a verified manifest, reusing a verified cache.
    ///
    /// # Errors
    /// Returns a validation, download, archive, storage, or cancellation error.
    pub fn install(
        &self,
        image: &RootfsImage,
        fetcher: &dyn RootfsFetcher,
        progress: &mut dyn RootfsProgress,
        cancel: &CancellationToken,
    ) -> Result<(), LinuxError> {
        self.ensure_layout()?;
        let archive = self.obtain_archive(image, fetcher, progress, cancel)?;
        if cancel.is_cancelled() {
            return Err(LinuxError::Cancelled);
        }
        progress.verifying();
        let partial = self.partial_path();
        remove_path(&partial)?;
        fs::create_dir(&partial).map_err(|_| io_error("create root filesystem staging"))?;
        let report = match extract_archive(&archive, &partial, image.max_extracted_bytes, cancel) {
            Ok(report) => report,
            Err(error) => {
                remove_path(&partial)?;
                return Err(error);
            }
        };
        progress.extracting(report.entries);
        self.replace_rootfs(&partial)?;
        self.write_state(&InstallState::from_image(image, report, false))?;
        progress.done();
        Ok(())
    }

    /// Returns the current installation and repair state.
    ///
    /// # Errors
    /// Returns a storage error when the state cannot be read.
    pub fn status(&self) -> Result<RootfsStatus, LinuxError> {
        let rootfs = self.rootfs_path();
        if !rootfs.is_dir() {
            return Ok(RootfsStatus::NotInstalled);
        }
        let Ok(state) = self.read_state() else {
            return Ok(RootfsStatus::NotInstalled);
        };
        let (missing, changed) = check_index(&rootfs, &state.files)?;
        if missing > 0 || changed > 0 {
            return Ok(RootfsStatus::NeedsRepair { missing, changed });
        }
        if state.ready {
            Ok(RootfsStatus::Ready {
                alpine_version: state.alpine_version,
            })
        } else {
            Ok(RootfsStatus::Installed {
                alpine_version: state.alpine_version,
            })
        }
    }

    /// Repairs missing or changed indexed files from the cached archive.
    ///
    /// # Errors
    /// Returns a cache, archive, storage, or cancellation error.
    pub fn repair(
        &self,
        fetcher: &dyn RootfsFetcher,
        progress: &mut dyn RootfsProgress,
        cancel: &CancellationToken,
    ) -> Result<(), LinuxError> {
        let mut state = self.read_state()?;
        let image = state.image.clone();
        let archive = self.obtain_archive(&image, fetcher, progress, cancel)?;
        let (missing, changed) = check_index(&self.rootfs_path(), &state.files)?;
        if missing == 0 && changed == 0 {
            return Ok(());
        }
        let wanted = state
            .files
            .iter()
            .filter(|file| file_missing_or_changed(&self.rootfs_path(), file))
            .map(|file| file.path.clone())
            .collect();
        repair_archive(&archive, &self.rootfs_path(), &wanted, cancel)?;
        state.ready = false;
        self.write_state(&state)
    }

    pub(crate) fn mark_ready(&self) -> Result<(), LinuxError> {
        let mut state = self.read_state()?;
        state.ready = true;
        self.write_state(&state)
    }

    pub(crate) fn set_version(&self, version: &str) -> Result<(), LinuxError> {
        let mut state = self.read_state()?;
        version.clone_into(&mut state.alpine_version);
        self.write_state(&state)
    }

    fn ensure_layout(&self) -> Result<(), LinuxError> {
        fs::create_dir_all(self.linux_root.join("rootfs"))
            .map_err(|_| io_error("create Linux rootfs storage"))?;
        fs::create_dir_all(self.linux_root.join("cache"))
            .map_err(|_| io_error("create Linux cache"))?;
        fs::create_dir_all(self.linux_root.join("tmp"))
            .map_err(|_| io_error("create Linux temp storage"))?;
        fs::create_dir_all(self.linux_root.join("etc"))
            .map_err(|_| io_error("create Linux network storage"))?;
        touch(&self.linux_root.join("etc/resolv.empty"))?;
        let resolv = self.linux_root.join("etc/resolv.conf");
        if !resolv.exists() {
            fs::write(resolv, b"nameserver 1.1.1.1\nnameserver 8.8.8.8\n")
                .map_err(|_| io_error("write Linux resolver"))?;
        }
        Ok(())
    }

    fn obtain_archive(
        &self,
        image: &RootfsImage,
        fetcher: &dyn RootfsFetcher,
        progress: &mut dyn RootfsProgress,
        cancel: &CancellationToken,
    ) -> Result<PathBuf, LinuxError> {
        let archive = self.cache_path(&image.sha256);
        if is_cached(&archive, image)? {
            return Ok(archive);
        }
        remove_path(&archive)?;
        let part = archive.with_extension("tar.gz.part");
        remove_path(&part)?;
        let file = File::create(&part).map_err(|_| io_error("create root filesystem cache"))?;
        let mut writer = DownloadWriter {
            file,
            hash: Sha256::new(),
            written: 0,
            limit: image.size_bytes,
            progress,
        };
        fetcher
            .fetch(image, &mut writer, cancel)
            .inspect_err(|_| drop(remove_path(&part)))?;
        if cancel.is_cancelled() {
            remove_path(&part)?;
            return Err(LinuxError::Cancelled);
        }
        writer.finish(image)?;
        fs::rename(&part, &archive).map_err(|_| io_error("commit root filesystem cache"))?;
        Ok(archive)
    }

    fn replace_rootfs(&self, partial: &Path) -> Result<(), LinuxError> {
        let rootfs = self.rootfs_path();
        let backup = self.linux_root.join("rootfs/alpine.previous");
        remove_path(&backup)?;
        if rootfs.exists() {
            fs::rename(&rootfs, &backup).map_err(|_| io_error("replace root filesystem"))?;
        }
        if let Err(error) = fs::rename(partial, &rootfs) {
            drop(fs::rename(&backup, &rootfs));
            return Err(error_to_io(error, "commit root filesystem"));
        }
        remove_path(&backup)
    }

    fn write_state(&self, state: &InstallState) -> Result<(), LinuxError> {
        let bytes = serde_json::to_vec_pretty(state)
            .map_err(|_| io_error("serialize root filesystem state"))?;
        let temporary = self.state_path().with_extension("json.part");
        fs::write(&temporary, bytes).map_err(|_| io_error("write root filesystem state"))?;
        fs::rename(temporary, self.state_path())
            .map_err(|_| io_error("commit root filesystem state"))
    }

    fn read_state(&self) -> Result<InstallState, LinuxError> {
        let bytes =
            fs::read(self.state_path()).map_err(|_| io_error("read root filesystem state"))?;
        serde_json::from_slice(&bytes)
            .map_err(|_| LinuxError::Manifest("root filesystem state invalid".into()))
    }

    fn rootfs_path(&self) -> PathBuf {
        self.linux_root.join("rootfs/alpine")
    }
    fn partial_path(&self) -> PathBuf {
        self.linux_root.join("rootfs/alpine.partial")
    }
    fn cache_path(&self, hash: &str) -> PathBuf {
        self.linux_root.join(format!("cache/{hash}.tar.gz"))
    }
    fn state_path(&self) -> PathBuf {
        self.linux_root.join("rootfs/alpine.json")
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct InstallState {
    alpine_version: String,
    image: RootfsImage,
    files: Vec<IndexedFile>,
    ready: bool,
    skipped_special: usize,
}

impl InstallState {
    fn from_image(image: &RootfsImage, report: ExtractReport, ready: bool) -> Self {
        Self {
            alpine_version: String::new(),
            image: image.clone(),
            files: report.files,
            ready,
            skipped_special: report.skipped_special,
        }
    }
}

struct DownloadWriter<'a> {
    file: File,
    hash: Sha256,
    written: u64,
    limit: u64,
    progress: &'a mut dyn RootfsProgress,
}

impl Write for DownloadWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let remaining = self.limit.saturating_sub(self.written);
        if bytes.len() as u64 > remaining {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "archive size exceeded",
            ));
        }
        self.file.write_all(bytes)?;
        self.hash.update(bytes);
        self.written += bytes.len() as u64;
        self.progress.downloading(self.written);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl DownloadWriter<'_> {
    fn finish(&mut self, image: &RootfsImage) -> Result<(), LinuxError> {
        self.file
            .flush()
            .map_err(|_| io_error("flush root filesystem cache"))?;
        if self.written != image.size_bytes {
            return Err(LinuxError::Fetch("archive size mismatch".into()));
        }
        if hex_digest(&self.hash.clone().finalize()) != image.sha256 {
            return Err(LinuxError::Fetch("archive hash mismatch".into()));
        }
        Ok(())
    }
}

fn is_cached(path: &Path, image: &RootfsImage) -> Result<bool, LinuxError> {
    if !path.is_file() {
        return Ok(false);
    }
    let size = fs::metadata(path)
        .map_err(|_| io_error("inspect root filesystem cache"))?
        .len();
    if size != image.size_bytes || !is_lower_hex(&image.sha256, 64) {
        return Ok(false);
    }
    Ok(sha256_file(path)? == image.sha256)
}

fn check_index(rootfs: &Path, files: &[IndexedFile]) -> Result<(usize, usize), LinuxError> {
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
    let size = fs::metadata(path)
        .map_err(|_| io_error("inspect root filesystem file"))?
        .len();
    Ok(size != file.size || digest != file.sha256)
}

fn file_missing_or_changed(rootfs: &Path, file: &IndexedFile) -> bool {
    match fs::symlink_metadata(rootfs.join(&file.path)) {
        Ok(metadata) if metadata.is_file() => {
            metadata.len() != file.size
                || sha256_file(&rootfs.join(&file.path)).map_or(true, |hash| hash != file.sha256)
        }
        _ => true,
    }
}

fn touch(path: &Path) -> Result<(), LinuxError> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(|_| ())
        .map_err(|_| io_error("create Linux resolver file"))
}

fn remove_path(path: &Path) -> Result<(), LinuxError> {
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

fn error_to_io(_error: io::Error, message: &str) -> LinuxError {
    io_error(message)
}
