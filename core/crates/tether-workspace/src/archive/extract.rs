//! Bounded ZIP extraction into an empty directory.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

use zip::ZipArchive;
use zip::read::ZipFile;

use super::names::{EntryInfo, reject_collisions, safe_entry_name, strip_single_top_level};
use super::{ArchiveError, ExtractLimits, ExtractReport, RATIO_LIMIT, RATIO_MIN_BYTES};

const SYMLINK_MODE_MASK: u32 = 0o17_0000;
const SYMLINK_MODE: u32 = 0o12_0000;
const COPY_CHUNK: usize = 64 * 1024;

/// Extracts a ZIP archive into an existing empty directory.
///
/// Entry names are validated before anything is written. Symbolic-link entries are skipped and
/// counted. A failure or a cancel removes everything this call wrote.
///
/// # Errors
/// Returns a typed error for unsafe names, limits, cancellation, invalid ZIP data, or I/O.
pub fn extract_zip(
    zip_path: impl AsRef<Path>,
    dest_dir: impl AsRef<Path>,
    limits: &ExtractLimits,
    cancelled: &dyn Fn() -> bool,
    strip_single_top_level: bool,
) -> Result<ExtractReport, ArchiveError> {
    let destination = dest_dir.as_ref();
    ensure_empty_directory(destination)?;
    let mut file = File::open(zip_path).map_err(ArchiveError::Io)?;
    let declared = declared_entry_count(&mut file).map_err(ArchiveError::Io)?;
    let mut archive = ZipArchive::new(file)?;
    // The zip reader keeps one entry per name; fewer entries than declared means duplicates.
    if declared.is_some_and(|count| count != archive.len() as u64) {
        return Err(ArchiveError::InvalidEntryName);
    }
    let entries = inspect_entries(&mut archive, limits, strip_single_top_level)?;
    let mut run = Extraction {
        archive,
        destination,
        limits,
        cancelled,
        written: Vec::new(),
        report: ExtractReport::default(),
    };
    let result = run.run(&entries);
    if result.is_err() {
        cleanup_written(&run.written);
    }
    result.map(|()| run.report)
}

/// Reads the entry count from the end-of-central-directory record, when it is a plain
/// (non-ZIP64) record.
fn declared_entry_count(file: &mut File) -> io::Result<Option<u64>> {
    const SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
    const RECORD: usize = 22;
    const MAX_TAIL: u64 = 65_535 + RECORD as u64;
    let length = file.metadata()?.len();
    let start = length.saturating_sub(MAX_TAIL);
    file.seek(SeekFrom::Start(start))?;
    let mut tail = Vec::new();
    file.take(MAX_TAIL).read_to_end(&mut tail)?;
    file.seek(SeekFrom::Start(0))?;
    let found = (0..=tail.len().saturating_sub(RECORD)).rev().find(|&at| {
        tail.len() >= at + RECORD
            && tail[at..at + 4] == SIGNATURE
            && usize::from(u16::from_le_bytes([tail[at + 20], tail[at + 21]]))
                == tail.len() - at - RECORD
    });
    let Some(at) = found else {
        return Ok(None);
    };
    let count = u16::from_le_bytes([tail[at + 10], tail[at + 11]]);
    Ok((count != u16::MAX).then_some(u64::from(count)))
}

fn inspect_entries(
    archive: &mut ZipArchive<File>,
    limits: &ExtractLimits,
    strip: bool,
) -> Result<Vec<EntryInfo>, ArchiveError> {
    let count = u64::try_from(archive.len()).map_err(|_| ArchiveError::TooManyEntries)?;
    if count > limits.max_entries {
        return Err(ArchiveError::TooManyEntries);
    }
    let mut entries = Vec::with_capacity(archive.len());
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().map_err(ArchiveError::from)?;
        let (path, is_dir) = safe_entry_name(name.as_ref())?;
        let is_symlink = entry_is_symlink(&entry);
        if !is_dir && !is_symlink {
            total = check_declared_size(&entry, limits, total)?;
        }
        entries.push(EntryInfo {
            index,
            path,
            is_dir,
            is_symlink,
        });
    }
    reject_collisions(&entries)?;
    if strip {
        strip_single_top_level(&mut entries);
    }
    Ok(entries)
}

/// Checks the size an entry declares against the limits and returns the new running total.
fn check_declared_size(
    entry: &ZipFile<'_, File>,
    limits: &ExtractLimits,
    total: u64,
) -> Result<u64, ArchiveError> {
    let size = entry.size();
    if size > limits.max_file_bytes {
        return Err(ArchiveError::TooLarge);
    }
    let total = total.checked_add(size).ok_or(ArchiveError::TooLarge)?;
    if total > limits.max_total_bytes {
        return Err(ArchiveError::TooLarge);
    }
    if size > RATIO_MIN_BYTES && size > entry.compressed_size().saturating_mul(RATIO_LIMIT) {
        return Err(ArchiveError::Ratio);
    }
    Ok(total)
}

struct Extraction<'a> {
    archive: ZipArchive<File>,
    destination: &'a Path,
    limits: &'a ExtractLimits,
    cancelled: &'a dyn Fn() -> bool,
    written: Vec<PathBuf>,
    report: ExtractReport,
}

impl Extraction<'_> {
    fn run(&mut self, entries: &[EntryInfo]) -> Result<(), ArchiveError> {
        entries.iter().try_for_each(|info| self.entry(info))
    }

    fn entry(&mut self, info: &EntryInfo) -> Result<(), ArchiveError> {
        if (self.cancelled)() {
            return Err(ArchiveError::Cancelled);
        }
        if info.is_symlink {
            self.report.skipped_symlinks += 1;
            return Ok(());
        }
        if info.path.as_os_str().is_empty() {
            return Ok(());
        }
        if info.is_dir {
            return self.directory(&info.path);
        }
        self.file(info)
    }

    fn directory(&mut self, relative: &Path) -> Result<(), ArchiveError> {
        let before = self.written.len();
        create_parents(self.destination, relative, &mut self.written)?;
        self.report.directories +=
            u64::try_from(self.written.len() - before).map_err(|_| ArchiveError::TooManyEntries)?;
        Ok(())
    }

    fn file(&mut self, info: &EntryInfo) -> Result<(), ArchiveError> {
        let parent = info.path.parent().unwrap_or_else(|| Path::new(""));
        create_parents(self.destination, parent, &mut self.written)?;
        let target = self.destination.join(&info.path);
        if fs::symlink_metadata(&target).is_ok_and(|metadata| metadata.is_dir()) {
            return Err(ArchiveError::InvalidEntryName);
        }
        let mut entry = self.archive.by_index(info.index)?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&target)
            .map_err(ArchiveError::Io)?;
        self.written.push(target.clone());
        let bytes = copy_entry(&mut entry, &mut output, self.limits, self.cancelled)?;
        apply_mode(&target, &entry)?;
        self.report.files += 1;
        self.report.bytes = self
            .report
            .bytes
            .checked_add(bytes)
            .ok_or(ArchiveError::TooLarge)?;
        if self.report.bytes > self.limits.max_total_bytes {
            return Err(ArchiveError::TooLarge);
        }
        Ok(())
    }
}

/// Copies one entry, stopping on cancel and on more bytes than the per-file limit.
fn copy_entry(
    entry: &mut impl Read,
    output: &mut impl Write,
    limits: &ExtractLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<u64, ArchiveError> {
    let mut buffer = vec![0_u8; COPY_CHUNK];
    let mut bytes = 0_u64;
    loop {
        if cancelled() {
            return Err(ArchiveError::Cancelled);
        }
        let read = entry.read(&mut buffer).map_err(ArchiveError::Io)?;
        if read == 0 {
            return Ok(bytes);
        }
        bytes = bytes
            .checked_add(u64::try_from(read).map_err(|_| ArchiveError::TooLarge)?)
            .ok_or(ArchiveError::TooLarge)?;
        if bytes > limits.max_file_bytes {
            return Err(ArchiveError::TooLarge);
        }
        output
            .write_all(&buffer[..read])
            .map_err(ArchiveError::Io)?;
    }
}

fn ensure_empty_directory(path: &Path) -> Result<(), ArchiveError> {
    let metadata = fs::symlink_metadata(path).map_err(ArchiveError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ArchiveError::InvalidDestination);
    }
    let mut entries = fs::read_dir(path).map_err(ArchiveError::Io)?;
    if entries.next().is_some() {
        return Err(ArchiveError::InvalidDestination);
    }
    Ok(())
}

/// Creates each missing folder of `relative` under `root`, one component at a time, and refuses
/// to pass through a symlink or a file.
fn create_parents(
    root: &Path,
    relative: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), ArchiveError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(ArchiveError::InvalidEntryName);
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(ArchiveError::InvalidDestination);
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(ArchiveError::Io)?;
                written.push(current.clone());
            }
            Err(error) => return Err(ArchiveError::Io(error)),
        }
    }
    Ok(())
}

/// Removes what a failed extraction wrote, newest first.
fn cleanup_written(written: &[PathBuf]) {
    for path in written.iter().rev() {
        if fs::remove_file(path).is_err() {
            drop(fs::remove_dir(path));
        }
    }
}

fn entry_is_symlink(entry: &ZipFile<'_, File>) -> bool {
    entry
        .unix_mode()
        .is_some_and(|mode| mode & SYMLINK_MODE_MASK == SYMLINK_MODE)
}

fn apply_mode(path: &Path, entry: &ZipFile<'_, File>) -> Result<(), ArchiveError> {
    #[cfg(unix)]
    if let Some(mode) = entry.unix_mode() {
        use std::os::unix::fs::PermissionsExt;
        // Keep the permission bits but never setuid or sticky, and never lock out the owner.
        let safe = (mode & 0o777) | 0o600;
        fs::set_permissions(path, fs::Permissions::from_mode(safe)).map_err(ArchiveError::Io)?;
    }
    #[cfg(not(unix))]
    let _ = (path, entry);
    Ok(())
}
