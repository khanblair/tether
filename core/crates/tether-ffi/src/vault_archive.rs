//! ZIP import and export helpers for the private vault.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_workspace::archive::{ArchiveError, CreateOptions, create_zip, extract_zip};

use crate::archive::extract_limits;
use crate::vault::{ImportMode, VaultImportReportView, VaultRuntimeError};

const ATTACHMENTS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "pdf"];

pub(crate) fn export(root: &Path, destination: &Path) -> Result<(), VaultRuntimeError> {
    let staging = temporary_path(root, "export")?;
    fs::create_dir(&staging).map_err(|_| VaultRuntimeError::Unavailable)?;
    let result = export_files(root, &staging).and_then(|()| {
        create_zip(
            &staging,
            destination,
            &CreateOptions {
                include_ignored: true,
                ..CreateOptions::default()
            },
            &|| false,
        )
        .map(|_| ())
        .map_err(|error| map_archive(&error))
    });
    drop(fs::remove_dir_all(&staging));
    result
}

pub(crate) fn import(
    root: &Path,
    archive: &Path,
    mode: ImportMode,
    date: &str,
) -> Result<VaultImportReportView, VaultRuntimeError> {
    if matches!(mode, ImportMode::ReplaceEmpty) && !vault_empty(root)? {
        return Err(VaultRuntimeError::VaultNotEmpty);
    }
    let staging = temporary_path(root, "import")?;
    fs::create_dir(&staging).map_err(|_| VaultRuntimeError::Unavailable)?;
    let result = extract_zip(archive, &staging, &extract_limits(), &|| false, false)
        .map(|_| ())
        .map_err(|error| map_archive(&error))
        .and_then(|()| import_staged(root, &staging, date));
    drop(fs::remove_dir_all(&staging));
    result
}

pub(crate) fn import_zip_date() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let days = seconds / 86_400;
    civil_date(days)
}

fn export_files(source: &Path, destination: &Path) -> Result<(), VaultRuntimeError> {
    copy_directory(source, source, destination)
}

fn copy_directory(root: &Path, source: &Path, destination: &Path) -> Result<(), VaultRuntimeError> {
    for entry in fs::read_dir(source).map_err(|_| VaultRuntimeError::Unavailable)? {
        let entry = entry.map_err(|_| VaultRuntimeError::Unavailable)?;
        let metadata =
            fs::symlink_metadata(entry.path()).map_err(|_| VaultRuntimeError::Unavailable)?;
        let name = entry.file_name();
        if metadata.file_type().is_symlink() || reserved_name(&name) {
            continue;
        }
        let source_path = entry.path();
        let relative = source_path
            .strip_prefix(root)
            .map_err(|_| VaultRuntimeError::Unavailable)?;
        let output = destination.join(relative);
        if metadata.is_dir() {
            fs::create_dir_all(&output).map_err(|_| VaultRuntimeError::Unavailable)?;
            copy_directory(root, &source_path, destination)?;
            continue;
        }
        if !metadata.is_file() || !allowed_file(&source_path) {
            continue;
        }
        let Some(parent) = output.parent() else {
            return Err(VaultRuntimeError::Unavailable);
        };
        fs::create_dir_all(parent).map_err(|_| VaultRuntimeError::Unavailable)?;
        fs::copy(source_path, output).map_err(|_| VaultRuntimeError::Unavailable)?;
    }
    Ok(())
}

fn import_staged(
    root: &Path,
    staging: &Path,
    date: &str,
) -> Result<VaultImportReportView, VaultRuntimeError> {
    let mut files = Vec::new();
    if contains_reserved_directory(staging)? {
        return Err(VaultRuntimeError::InvalidPath);
    }
    collect_files(staging, staging, &mut files)?;
    if files.iter().any(|(path, _)| reserved_path(path)) {
        return Err(VaultRuntimeError::InvalidPath);
    }
    let mut report = VaultImportReportView::default();
    for (relative, source) in files {
        if !allowed_file(&source) {
            report.skipped += 1;
            continue;
        }
        import_file(root, &relative, &source, date, &mut report)?;
    }
    Ok(report)
}

fn collect_files(
    root: &Path,
    folder: &Path,
    output: &mut Vec<(String, PathBuf)>,
) -> Result<(), VaultRuntimeError> {
    for entry in fs::read_dir(folder).map_err(|_| VaultRuntimeError::Unavailable)? {
        let entry = entry.map_err(|_| VaultRuntimeError::Unavailable)?;
        let metadata =
            fs::symlink_metadata(entry.path()).map_err(|_| VaultRuntimeError::Unavailable)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        let path = entry.path();
        if metadata.is_dir() {
            collect_files(root, &path, output)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| VaultRuntimeError::Unavailable)?
                .to_string_lossy()
                .replace('\\', "/");
            output.push((relative, path));
        }
    }
    Ok(())
}

fn contains_reserved_directory(folder: &Path) -> Result<bool, VaultRuntimeError> {
    for entry in fs::read_dir(folder).map_err(|_| VaultRuntimeError::Unavailable)? {
        let entry = entry.map_err(|_| VaultRuntimeError::Unavailable)?;
        let metadata =
            fs::symlink_metadata(entry.path()).map_err(|_| VaultRuntimeError::Unavailable)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir()
            && (reserved_name(&entry.file_name()) || contains_reserved_directory(&entry.path())?)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn import_file(
    root: &Path,
    relative: &str,
    source: &Path,
    date: &str,
    report: &mut VaultImportReportView,
) -> Result<(), VaultRuntimeError> {
    let bytes = fs::read(source).map_err(|_| VaultRuntimeError::Unavailable)?;
    let target = safe_target(root, relative)?;
    if target.exists() {
        let existing = fs::read(&target).map_err(|_| VaultRuntimeError::Unavailable)?;
        if existing == bytes {
            report.skipped += 1;
            return Ok(());
        }
        let conflict = conflict_target(root, relative, date)?;
        write_import(root, &conflict, &bytes)?;
        report.conflicts += 1;
        report.imported += 1;
        return Ok(());
    }
    write_import(root, relative, &bytes)?;
    report.imported += 1;
    Ok(())
}

fn write_import(root: &Path, relative: &str, bytes: &[u8]) -> Result<(), VaultRuntimeError> {
    let target = safe_target(root, relative)?;
    let is_markdown = Path::new(relative)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));
    if is_markdown {
        let source = std::str::from_utf8(bytes).map_err(|_| VaultRuntimeError::InvalidNote)?;
        tether_memory::vault::VaultStore::new(root)
            .write(relative, source)
            .map_err(|error| match error {
                tether_memory::vault::VaultError::InvalidNote { .. }
                | tether_memory::vault::VaultError::OverCap => VaultRuntimeError::InvalidNote,
                tether_memory::vault::VaultError::InvalidPath => VaultRuntimeError::InvalidPath,
                tether_memory::vault::VaultError::Io(_)
                | tether_memory::vault::VaultError::MissingNote => VaultRuntimeError::Unavailable,
            })?;
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|_| VaultRuntimeError::Unavailable)?;
    }
    atomic_write(&target, bytes).map_err(|_| VaultRuntimeError::Unavailable)
}

fn conflict_target(root: &Path, relative: &str, date: &str) -> Result<String, VaultRuntimeError> {
    let path = Path::new(relative);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or(VaultRuntimeError::InvalidPath)?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin");
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let base = parent.join(format!("{stem} (imported {date}).{extension}"));
    let mut candidate = base.to_string_lossy().replace('\\', "/");
    for index in 2..=100 {
        if !safe_target(root, &candidate)?.exists() {
            return Ok(candidate);
        }
        candidate = parent
            .join(format!("{stem} (imported {date}) {index}.{extension}"))
            .to_string_lossy()
            .replace('\\', "/");
    }
    Err(VaultRuntimeError::Unavailable)
}

fn safe_target(root: &Path, relative: &str) -> Result<PathBuf, VaultRuntimeError> {
    let path = Path::new(relative);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
        || reserved_path(relative)
    {
        return Err(VaultRuntimeError::InvalidPath);
    }
    let mut target = root.to_path_buf();
    for component in path.components() {
        let Component::Normal(name) = component else {
            return Err(VaultRuntimeError::InvalidPath);
        };
        target.push(name);
        if fs::symlink_metadata(&target).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err(VaultRuntimeError::InvalidPath);
        }
    }
    Ok(target)
}

fn vault_empty(root: &Path) -> Result<bool, VaultRuntimeError> {
    let mut entries = fs::read_dir(root).map_err(|_| VaultRuntimeError::Unavailable)?;
    Ok(entries.all(|entry| entry.is_ok_and(|item| item.file_name() == ".tether-sync")))
}

fn temporary_path(root: &Path, label: &str) -> Result<PathBuf, VaultRuntimeError> {
    let parent = root.parent().ok_or(VaultRuntimeError::Unavailable)?;
    Ok(parent.join(format!(".tether-vault-{label}-{}", unique_suffix())))
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}

fn reserved_name(name: &std::ffi::OsStr) -> bool {
    name == ".obsidian" || name == ".tether-sync"
}

fn reserved_path(path: &str) -> bool {
    Path::new(path).components().any(|component| {
        matches!(component, Component::Normal(name) if name == ".obsidian" || name == ".tether-sync")
    })
}

fn allowed_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| extension == "md" || ATTACHMENTS.contains(&extension.as_str()))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = path.with_extension(format!("tmp-{}", unique_suffix()));
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)
}

fn map_archive(error: &ArchiveError) -> VaultRuntimeError {
    match error {
        ArchiveError::Cancelled => VaultRuntimeError::Cancelled,
        ArchiveError::InvalidDestination => VaultRuntimeError::InvalidPath,
        ArchiveError::InvalidEntryName
        | ArchiveError::TooManyEntries
        | ArchiveError::TooLarge
        | ArchiveError::Ratio
        | ArchiveError::NotZip => VaultRuntimeError::Archive,
        ArchiveError::Io(_) => VaultRuntimeError::Unavailable,
    }
}

fn civil_date(days_since_epoch: u64) -> String {
    let days = i64::try_from(days_since_epoch).unwrap_or(i64::MAX) + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_part =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_part + era * 400;
    let day_of_year = day_of_era - (365 * year_part + year_part / 4 - year_part / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
