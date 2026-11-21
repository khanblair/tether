//! Safe, bounded ZIP import and export for project workspaces.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use zip::result::ZipError;
use zip::write::{FileOptions, SimpleFileOptions};
use zip::{ZipArchive, ZipWriter};

const DEFAULT_MAX_ENTRIES: u64 = 50_000;
const DEFAULT_MAX_TOTAL_BYTES: u64 = 1 << 30;
const DEFAULT_MAX_FILE_BYTES: u64 = 256 << 20;
const RATIO_MIN_BYTES: u64 = 100 << 20;
const RATIO_LIMIT: u64 = 200;

/// Limits applied while extracting a ZIP archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtractLimits {
    /// Maximum number of archive entries.
    pub max_entries: u64,
    /// Maximum total uncompressed bytes.
    pub max_total_bytes: u64,
    /// Maximum uncompressed bytes for one file.
    pub max_file_bytes: u64,
}

impl Default for ExtractLimits {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_MAX_ENTRIES,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }
}

/// Options applied while creating a ZIP archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateOptions {
    /// Include files ignored by the workspace's ignore rules.
    pub include_ignored: bool,
    /// Maximum total bytes read from regular files.
    pub max_total_bytes: u64,
}

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            include_ignored: false,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
        }
    }
}

/// Results from extracting a ZIP archive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExtractReport {
    /// Number of regular files written.
    pub files: u64,
    /// Number of directories created.
    pub directories: u64,
    /// Number of uncompressed file bytes written.
    pub bytes: u64,
    /// Number of symbolic-link entries skipped.
    pub skipped_symlinks: u64,
}

/// Results from creating a ZIP archive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CreateReport {
    /// Number of regular files archived.
    pub files: u64,
    /// Number of regular file bytes read.
    pub bytes: u64,
    /// Number of symbolic links skipped.
    pub skipped_symlinks: u64,
}

/// A safe archive operation failure.
#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    /// An entry name is unsafe.
    #[error("invalid archive entry name")]
    InvalidEntryName,
    /// The archive contains too many entries.
    #[error("archive has too many entries")]
    TooManyEntries,
    /// The archive exceeds a byte limit.
    #[error("archive is too large")]
    TooLarge,
    /// A file has an unsafe compression ratio.
    #[error("archive compression ratio is unsafe")]
    Ratio,
    /// The operation was cancelled.
    #[error("archive operation cancelled")]
    Cancelled,
    /// The destination path is unsafe.
    #[error("archive destination is invalid")]
    InvalidDestination,
    /// The input is not a valid ZIP archive.
    #[error("file is not a ZIP archive")]
    NotZip,
    /// The filesystem operation failed.
    #[error("archive I/O failure")]
    Io(#[source] io::Error),
}

impl From<ZipError> for ArchiveError {
    fn from(error: ZipError) -> Self {
        match error {
            ZipError::Io(error) => Self::Io(error),
            _ => Self::NotZip,
        }
    }
}

/// Extracts a ZIP archive into an existing empty directory.
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
    let file = File::open(zip_path).map_err(ArchiveError::Io)?;
    let mut archive = ZipArchive::new(file)?;
    let entries = inspect_entries(&mut archive, limits, strip_single_top_level)?;
    let mut written = Vec::new();
    let result = extract_entries(
        &mut archive,
        destination,
        &entries,
        limits,
        cancelled,
        &mut written,
    );
    if result.is_err() {
        cleanup_written(destination, &written);
    }
    result
}

/// Creates a ZIP archive beside the destination using the workspace ignore rules.
///
/// # Errors
/// Returns a typed error for an unsafe destination, limits, cancellation, or I/O.
pub fn create_zip(
    root: impl AsRef<Path>,
    dest_file: impl AsRef<Path>,
    options: &CreateOptions,
    cancelled: &dyn Fn() -> bool,
) -> Result<CreateReport, ArchiveError> {
    let root = fs::canonicalize(root).map_err(ArchiveError::Io)?;
    let destination = dest_file.as_ref();
    refuse_destination_inside_root(&root, destination)?;
    let parent = destination
        .parent()
        .ok_or(ArchiveError::InvalidDestination)?;
    fs::create_dir_all(parent).map_err(ArchiveError::Io)?;
    let temporary = parent.join(format!(".tether-archive-{}.tmp", unique_suffix()));
    let result = create_zip_inner(&root, &temporary, options, cancelled);
    match result {
        Ok(report) => {
            fs::rename(&temporary, destination).map_err(ArchiveError::Io)?;
            Ok(report)
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(error)
        }
    }
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
    let mut paths = std::collections::HashSet::new();
    let mut top = None;
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().map_err(ArchiveError::from)?;
        let (path, is_dir) = safe_entry_name(name.as_ref())?;
        let is_symlink = entry_is_symlink(&entry);
        let key = path.to_string_lossy().to_string();
        if !paths.insert(key) {
            return Err(ArchiveError::InvalidEntryName);
        }
        if !is_dir && !is_symlink {
            let size = entry.size();
            if size > limits.max_file_bytes {
                return Err(ArchiveError::TooLarge);
            }
            total = total.checked_add(size).ok_or(ArchiveError::TooLarge)?;
            if total > limits.max_total_bytes {
                return Err(ArchiveError::TooLarge);
            }
            if size > RATIO_MIN_BYTES && size > entry.compressed_size().saturating_mul(RATIO_LIMIT)
            {
                return Err(ArchiveError::Ratio);
            }
        }
        entries.push(EntryInfo {
            index,
            path,
            is_dir,
            is_symlink,
        });
    }
    let strip_root = strip.then(|| common_top_level(&entries));
    if strip_root.as_ref().is_some_and(Option::is_some) {
        let root = strip_root.flatten();
        for entry in &mut entries {
            if let Some(root) = root.as_ref() {
                entry.path = entry
                    .path
                    .strip_prefix(root)
                    .unwrap_or(&entry.path)
                    .to_path_buf();
            }
        }
    }
    validate_path_collisions(&entries)
}

fn extract_entries(
    archive: &mut ZipArchive<File>,
    destination: &Path,
    entries: &[EntryInfo],
    limits: &ExtractLimits,
    cancelled: &dyn Fn() -> bool,
    written: &mut Vec<PathBuf>,
) -> Result<ExtractReport, ArchiveError> {
    let mut report = ExtractReport::default();
    for info in entries {
        if cancelled() {
            return Err(ArchiveError::Cancelled);
        }
        if info.is_symlink {
            report.skipped_symlinks += 1;
            continue;
        }
        if info.path.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(&info.path);
        if info.is_dir {
            create_parents(destination, &info.path, written)?;
            if !target.exists() {
                fs::create_dir(&target).map_err(ArchiveError::Io)?;
                written.push(target);
                report.directories += 1;
            }
            continue;
        }
        let parent = info.path.parent().unwrap_or_else(|| Path::new(""));
        create_parents(destination, parent, written)?;
        reject_existing_symlink(destination, &info.path)?;
        if fs::symlink_metadata(&target)
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false)
        {
            return Err(ArchiveError::InvalidEntryName);
        }
        let mut entry = archive.by_index(info.index)?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&target)
            .map_err(ArchiveError::Io)?;
        written.push(target.clone());
        let mut buffer = [0_u8; 64 * 1024];
        let mut bytes = 0_u64;
        loop {
            if cancelled() {
                return Err(ArchiveError::Cancelled);
            }
            let read = entry.read(&mut buffer).map_err(ArchiveError::Io)?;
            if read == 0 {
                break;
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
        apply_mode(&target, &entry)?;
        report.files += 1;
        report.bytes = report
            .bytes
            .checked_add(bytes)
            .ok_or(ArchiveError::TooLarge)?;
    }
    Ok(report)
}

fn create_zip_inner(
    root: &Path,
    temporary: &Path,
    options: &CreateOptions,
    cancelled: &dyn Fn() -> bool,
) -> Result<CreateReport, ArchiveError> {
    let mut paths = Vec::new();
    let mut skipped_symlinks = 0_u64;
    collect_paths(
        root,
        options.include_ignored,
        &mut paths,
        &mut skipped_symlinks,
    )?;
    paths.sort();
    let output = File::create(temporary).map_err(ArchiveError::Io)?;
    let mut writer = ZipWriter::new(output);
    let mut report = CreateReport {
        skipped_symlinks,
        ..CreateReport::default()
    };
    for path in paths {
        if cancelled() {
            return Err(ArchiveError::Cancelled);
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| ArchiveError::InvalidDestination)?;
        let name = relative.to_string_lossy().replace('\\', "/");
        let metadata = fs::symlink_metadata(&path).map_err(ArchiveError::Io)?;
        let size = metadata.len();
        report.bytes = report
            .bytes
            .checked_add(size)
            .ok_or(ArchiveError::TooLarge)?;
        if report.bytes > options.max_total_bytes {
            return Err(ArchiveError::TooLarge);
        }
        let mode = regular_mode(&metadata);
        let file_options: SimpleFileOptions = FileOptions::default().unix_permissions(mode);
        writer.start_file(name, file_options)?;
        let mut input = File::open(&path).map_err(ArchiveError::Io)?;
        io::copy(&mut input, &mut writer).map_err(ArchiveError::Io)?;
        report.files += 1;
    }
    writer.finish()?.sync_all().map_err(ArchiveError::Io)?;
    Ok(report)
}

fn collect_paths(
    root: &Path,
    include_ignored: bool,
    paths: &mut Vec<PathBuf>,
    skipped_symlinks: &mut u64,
) -> Result<(), ArchiveError> {
    let matcher = default_matcher(root)?;
    let mut builder = ignore::WalkBuilder::new(root);
    builder
        .hidden(false)
        .ignore(false)
        .git_global(false)
        .git_ignore(!include_ignored)
        .git_exclude(!include_ignored)
        .follow_links(false)
        .add_custom_ignore_filename(".tetherignore")
        .filter_entry(move |entry| {
            let name = entry.file_name().to_string_lossy();
            if name == ".git" {
                return false;
            }
            include_ignored
                || entry.file_type().is_none_or(|kind| kind.is_dir())
                || !matcher.matched(entry.path(), false).is_ignore()
        });
    for result in builder.build() {
        let entry = result.map_err(|error| ArchiveError::Io(io::Error::other(error)))?;
        if entry.depth() == 0 || entry.path_is_symlink() {
            if entry.path_is_symlink() {
                *skipped_symlinks += 1;
            }
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path()).map_err(ArchiveError::Io)?;
        if metadata.is_file() {
            paths.push(entry.path().to_path_buf());
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct EntryInfo {
    index: usize,
    path: PathBuf,
    is_dir: bool,
    is_symlink: bool,
}

fn safe_entry_name(name: &str) -> Result<(PathBuf, bool), ArchiveError> {
    if name.is_empty() || name.contains('\0') || name.contains('\\') {
        return Err(ArchiveError::InvalidEntryName);
    }
    let is_dir = name.ends_with('/');
    let trimmed = name.trim_end_matches('/');
    if trimmed.is_empty() || Path::new(trimmed).is_absolute() {
        return Err(ArchiveError::InvalidEntryName);
    }
    let mut path = PathBuf::new();
    for (index, component) in trimmed.split('/').enumerate() {
        if component.is_empty() || component == ".." || (index == 0 && component.ends_with(':')) {
            return Err(ArchiveError::InvalidEntryName);
        }
        if component == "." {
            return Err(ArchiveError::InvalidEntryName);
        }
        path.push(component);
    }
    Ok((path, is_dir))
}

fn validate_path_collisions(entries: &[EntryInfo]) -> Result<Vec<EntryInfo>, ArchiveError> {
    let mut kinds = std::collections::HashMap::new();
    for entry in entries {
        let key = entry.path.to_string_lossy().to_string();
        let kind = entry.is_dir;
        if kinds.insert(key, kind).is_some() {
            return Err(ArchiveError::InvalidEntryName);
        }
        for ancestor in entry.path.ancestors().skip(1) {
            if kinds.get(&ancestor.to_string_lossy().to_string()) == Some(&false) {
                return Err(ArchiveError::InvalidEntryName);
            }
        }
    }
    Ok(entries.to_vec())
}

fn common_top_level(entries: &[EntryInfo]) -> Option<PathBuf> {
    let mut top = None;
    for entry in entries {
        let first = entry.path.components().next()?.as_os_str().to_owned();
        match &top {
            None => top = Some(first),
            Some(existing) if existing == &first => (),
            Some(_) => return None,
        }
    }
    top.map(PathBuf::from)
}

fn ensure_empty_directory(path: &Path) -> Result<(), ArchiveError> {
    let metadata = fs::symlink_metadata(path).map_err(ArchiveError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ArchiveError::InvalidDestination);
    }
    if fs::read_dir(path)
        .map_err(ArchiveError::Io)?
        .next()
        .is_some()
    {
        return Err(ArchiveError::InvalidDestination);
    }
    Ok(())
}

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
            Ok(_) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(ArchiveError::Io)?;
                written.push(current.clone());
            }
            Err(error) => return Err(ArchiveError::Io(error)),
        }
    }
    Ok(())
}

fn cleanup_written(root: &Path, written: &[PathBuf]) {
    for path in written.iter().rev() {
        let _ = fs::remove_file(path).or_else(|_| fs::remove_dir(path));
    }
    let _ = root;
}

fn reject_existing_symlink(root: &Path, relative: &Path) -> Result<(), ArchiveError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(ArchiveError::InvalidEntryName);
        };
        current.push(name);
        if fs::symlink_metadata(&current)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(ArchiveError::InvalidDestination);
        }
    }
    Ok(())
}

fn entry_is_symlink(entry: &zip::read::ZipFile<'_, File>) -> bool {
    entry
        .unix_mode()
        .is_some_and(|mode| mode & 0o170000 == 0o120000)
}

fn apply_mode(path: &Path, entry: &zip::read::ZipFile<'_, File>) -> Result<(), ArchiveError> {
    #[cfg(unix)]
    if let Some(mode) = entry.unix_mode() {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o7777))
            .map_err(ArchiveError::Io)?;
    }
    Ok(())
}

fn regular_mode(metadata: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o7777
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        0o644
    }
}

fn refuse_destination_inside_root(root: &Path, destination: &Path) -> Result<(), ArchiveError> {
    let absolute = if destination.exists() {
        fs::canonicalize(destination).map_err(ArchiveError::Io)?
    } else {
        let parent = destination
            .parent()
            .ok_or(ArchiveError::InvalidDestination)?;
        fs::canonicalize(parent).map_err(ArchiveError::Io)?.join(
            destination
                .file_name()
                .ok_or(ArchiveError::InvalidDestination)?,
        )
    };
    if absolute.starts_with(root) {
        return Err(ArchiveError::InvalidDestination);
    }
    Ok(())
}

fn default_matcher(root: &Path) -> Result<Gitignore, ArchiveError> {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in [
        "node_modules/",
        "target/",
        "build/",
        ".gradle/",
        ".idea/",
        "__pycache__/",
        ".venv/",
        "venv/",
        "dist/",
        ".DS_Store",
    ] {
        builder
            .add_line(None, pattern)
            .map_err(|_| ArchiveError::InvalidEntryName)?;
    }
    let tetherignore = root.join(".tetherignore");
    if fs::symlink_metadata(&tetherignore)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
    {
        let _ = builder.add(&tetherignore);
    }
    builder.build().map_err(|_| ArchiveError::InvalidEntryName)
}

fn unique_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map_or_else(
        |_| "0".to_owned(),
        |duration| duration.as_nanos().to_string(),
    )
}
