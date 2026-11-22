//! ZIP creation from a project folder.

use std::fs::{self, File};
use std::io;
use std::path::Path;

use zip::ZipWriter;
use zip::write::{FileOptions, SimpleFileOptions};

use super::{ArchiveError, CreateOptions, CreateReport};
use crate::ignore_rules::IgnoreRules;

/// Creates a ZIP archive of a project using the workspace ignore rules.
///
/// The archive is written to a temporary file beside `dest_file` and renamed on success, so a
/// failure or a cancel leaves no partial file. `.git` is never included, symbolic links are
/// skipped and counted, and entries are sorted by path.
///
/// # Errors
/// Returns a typed error for a destination inside the project, a limit, cancellation, or I/O.
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
    match write_archive(&root, &temporary, options, cancelled) {
        Ok(report) => {
            fs::rename(&temporary, destination).map_err(ArchiveError::Io)?;
            Ok(report)
        }
        Err(error) => {
            drop(fs::remove_file(&temporary));
            Err(error)
        }
    }
}

fn write_archive(
    root: &Path,
    temporary: &Path,
    options: &CreateOptions,
    cancelled: &dyn Fn() -> bool,
) -> Result<CreateReport, ArchiveError> {
    let mut rules = IgnoreRules::load(root).map_err(ArchiveError::Io)?;
    let walk = rules
        .walk(options.include_ignored)
        .map_err(ArchiveError::Io)?;
    let mut files = walk.files;
    files.sort_by(|left, right| left.relative.cmp(&right.relative));
    let mut writer = ZipWriter::new(File::create(temporary).map_err(ArchiveError::Io)?);
    let mut report = CreateReport {
        skipped_symlinks: walk.skipped_symlinks,
        ..CreateReport::default()
    };
    for file in files {
        if cancelled() {
            return Err(ArchiveError::Cancelled);
        }
        report.bytes = report
            .bytes
            .checked_add(file.bytes)
            .ok_or(ArchiveError::TooLarge)?;
        if report.bytes > options.max_total_bytes {
            return Err(ArchiveError::TooLarge);
        }
        let metadata = fs::symlink_metadata(&file.absolute).map_err(ArchiveError::Io)?;
        let name = file.relative.to_string_lossy().replace('\\', "/");
        let file_options: SimpleFileOptions =
            FileOptions::default().unix_permissions(regular_mode(&metadata));
        writer.start_file(name, file_options)?;
        let mut input = File::open(&file.absolute).map_err(ArchiveError::Io)?;
        io::copy(&mut input, &mut writer).map_err(ArchiveError::Io)?;
        report.files += 1;
    }
    writer.finish()?.sync_all().map_err(ArchiveError::Io)?;
    Ok(report)
}

fn regular_mode(metadata: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o777
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        0o644
    }
}

/// Resolves the destination through its nearest existing folder and refuses a path inside `root`.
fn refuse_destination_inside_root(root: &Path, destination: &Path) -> Result<(), ArchiveError> {
    let name = destination
        .file_name()
        .ok_or(ArchiveError::InvalidDestination)?;
    let mut missing = vec![name.to_owned()];
    let mut existing = destination
        .parent()
        .ok_or(ArchiveError::InvalidDestination)?;
    while !existing.exists() {
        missing.push(
            existing
                .file_name()
                .ok_or(ArchiveError::InvalidDestination)?
                .to_owned(),
        );
        existing = existing.parent().ok_or(ArchiveError::InvalidDestination)?;
    }
    let mut resolved = fs::canonicalize(existing).map_err(ArchiveError::Io)?;
    resolved.extend(missing.iter().rev());
    if resolved.starts_with(root) {
        return Err(ArchiveError::InvalidDestination);
    }
    Ok(())
}

fn unique_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map_or_else(
        |_| "0".to_owned(),
        |duration| duration.as_nanos().to_string(),
    )
}
