use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::{Archive, EntryType};
use tokio_util::sync::CancellationToken;

use crate::LinuxError;
use crate::error::io_error;
use crate::manifest::hex_digest;

pub(crate) const ENTRY_LIMIT: usize = 50_000;
pub(crate) const FILE_LIMIT: u64 = 200 * 1024 * 1024;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub(crate) struct IndexedFile {
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) sha256: String,
}

#[derive(Debug, Default)]
pub(crate) struct ExtractReport {
    pub(crate) files: Vec<IndexedFile>,
    pub(crate) entries: usize,
    pub(crate) skipped_special: usize,
}

pub(crate) fn extract_archive(
    archive_path: &Path,
    destination: &Path,
    max_bytes: u64,
    cancel: &CancellationToken,
) -> Result<ExtractReport, LinuxError> {
    let file = File::open(archive_path).map_err(|_| io_error("open root filesystem archive"))?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    let mut context = ExtractContext::new(destination, max_bytes);
    for item in archive
        .entries()
        .map_err(|_| archive_error("read archive entries"))?
    {
        let mut entry = item.map_err(|_| archive_error("read archive entry"))?;
        context.entries += 1;
        if context.entries > ENTRY_LIMIT {
            return Err(archive_error("entry limit exceeded"));
        }
        if cancel.is_cancelled() {
            return Err(LinuxError::Cancelled);
        }
        extract_entry(&mut entry, &mut context, cancel)?;
    }
    Ok(context.report)
}

pub(crate) fn repair_archive(
    archive_path: &Path,
    destination: &Path,
    wanted: &BTreeSet<String>,
    cancel: &CancellationToken,
) -> Result<(), LinuxError> {
    let file = File::open(archive_path).map_err(|_| io_error("open root filesystem archive"))?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    let context = RepairContext { destination };
    for item in archive
        .entries()
        .map_err(|_| archive_error("read archive entries"))?
    {
        let mut entry = item.map_err(|_| archive_error("read archive entry"))?;
        let path = safe_entry_path(entry.path_bytes().as_ref())?;
        if !wanted.contains(&path) {
            continue;
        }
        if cancel.is_cancelled() {
            return Err(LinuxError::Cancelled);
        }
        let target = context.destination.join(&path);
        refuse_symlink(&target)?;
        remove_existing_file(&target)?;
        let mode = entry
            .header()
            .mode()
            .map_err(|_| archive_error("read file mode"))?;
        write_regular(&mut entry, &target, cancel, None)?;
        set_mode(&target, mode)?;
    }
    Ok(())
}

struct ExtractContext<'a> {
    destination: &'a Path,
    max_bytes: u64,
    total_bytes: u64,
    entries: usize,
    report: ExtractReport,
    hard_links: BTreeMap<String, PathBuf>,
}

impl<'a> ExtractContext<'a> {
    fn new(destination: &'a Path, max_bytes: u64) -> Self {
        Self {
            destination,
            max_bytes,
            total_bytes: 0,
            entries: 0,
            report: ExtractReport::default(),
            hard_links: BTreeMap::new(),
        }
    }
}

struct RepairContext<'a> {
    destination: &'a Path,
}

fn extract_entry(
    entry: &mut tar::Entry<'_, GzDecoder<File>>,
    context: &mut ExtractContext<'_>,
    cancel: &CancellationToken,
) -> Result<(), LinuxError> {
    let path = safe_entry_path(entry.path_bytes().as_ref())?;
    let target = context.destination.join(&path);
    ensure_parents(context.destination, &path)?;
    let entry_type = entry.header().entry_type();
    if entry_type.is_dir() {
        create_directory(
            &target,
            entry
                .header()
                .mode()
                .map_err(|_| archive_error("read directory mode"))?,
        )?;
    } else if entry_type.is_symlink() {
        create_symlink(entry, &target)?;
    } else if entry_type.is_hard_link() {
        create_hard_link(entry, &target, context)?;
    } else if entry_type == EntryType::new(b'0') || entry_type == EntryType::new(b'7') {
        let mode = entry
            .header()
            .mode()
            .map_err(|_| archive_error("read file mode"))?;
        let indexed = write_regular(
            entry,
            &target,
            cancel,
            Some((&mut context.total_bytes, context.max_bytes)),
        )?;
        set_mode(&target, mode)?;
        context.hard_links.insert(path.clone(), target);
        context.report.files.push(IndexedFile {
            path,
            size: indexed.0,
            sha256: indexed.1,
        });
    } else {
        context.report.skipped_special += 1;
    }
    context.report.entries = context.entries;
    Ok(())
}

fn write_regular(
    entry: &mut tar::Entry<'_, GzDecoder<File>>,
    target: &Path,
    cancel: &CancellationToken,
    mut limits: Option<(&mut u64, u64)>,
) -> Result<(u64, String), LinuxError> {
    refuse_symlink(target)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|_| io_error("create root filesystem file"))?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
    let mut size = 0_u64;
    loop {
        if cancel.is_cancelled() {
            return Err(LinuxError::Cancelled);
        }
        let count = entry
            .read(&mut buffer)
            .map_err(|_| archive_error("read file data"))?;
        if count == 0 {
            break;
        }
        let next = size
            .checked_add(count as u64)
            .ok_or_else(|| archive_error("file size overflow"))?;
        if next > FILE_LIMIT {
            return Err(archive_error("file limit exceeded"));
        }
        update_total(&mut limits, count)?;
        file.write_all(&buffer[..count])
            .map_err(|_| io_error("write root filesystem file"))?;
        hash.update(&buffer[..count]);
        size = next;
    }
    Ok((size, hex_digest(&hash.finalize())))
}

fn create_hard_link(
    entry: &mut tar::Entry<'_, GzDecoder<File>>,
    target: &Path,
    context: &mut ExtractContext<'_>,
) -> Result<(), LinuxError> {
    let link = entry
        .link_name()
        .map_err(|_| archive_error("read hard link"))?
        .ok_or_else(|| archive_error("missing hard link target"))?;
    let name = safe_entry_path(link.as_os_str().to_string_lossy().as_bytes())?;
    let source = context
        .hard_links
        .get(&name)
        .ok_or_else(|| archive_error("hard link target was not extracted"))?;
    refuse_symlink(target)?;
    if fs::hard_link(source, target).is_err() {
        let bytes = fs::read(source).map_err(|_| io_error("copy hard link"))?;
        fs::write(target, bytes).map_err(|_| io_error("copy hard link"))?;
    }
    Ok(())
}

fn create_symlink(
    entry: &mut tar::Entry<'_, GzDecoder<File>>,
    target: &Path,
) -> Result<(), LinuxError> {
    refuse_symlink(target)?;
    let link = entry
        .link_name()
        .map_err(|_| archive_error("read symlink"))?
        .ok_or_else(|| archive_error("missing symlink target"))?;
    symlink(link, target).map_err(|_| io_error("create root filesystem symlink"))
}

fn create_directory(target: &Path, mode: u32) -> Result<(), LinuxError> {
    if let Ok(metadata) = fs::symlink_metadata(target) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(archive_error("directory collision"));
        }
    } else {
        fs::create_dir(target).map_err(|_| io_error("create root filesystem directory"))?;
    }
    set_mode(target, mode)
}

fn ensure_parents(destination: &Path, relative: &str) -> Result<(), LinuxError> {
    let mut current = destination.to_path_buf();
    let parent = Path::new(relative)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    for component in parent.components() {
        if let Component::Normal(name) = component {
            current.push(name);
            ensure_parent_directory(&current)?;
        }
    }
    Ok(())
}

fn refuse_symlink(path: &Path) -> Result<(), LinuxError> {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        Err(archive_error("archive writes through a symlink"))
    } else {
        Ok(())
    }
}

fn ensure_parent_directory(path: &Path) -> Result<(), LinuxError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(archive_error("parent traverses a symlink"))
        }
        Ok(_) => Ok(()),
        Err(_) => fs::create_dir(path).map_err(|_| io_error("create archive parent")),
    }
}

fn remove_existing_file(path: &Path) -> Result<(), LinuxError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            fs::remove_file(path).map_err(|_| io_error("replace root filesystem file"))
        }
        Ok(_) => Err(archive_error("repair target is not a file")),
        Err(_) => Ok(()),
    }
}

fn update_total(limits: &mut Option<(&mut u64, u64)>, count: usize) -> Result<(), LinuxError> {
    let Some((total, maximum)) = limits else {
        return Ok(());
    };
    **total = (**total)
        .checked_add(count as u64)
        .ok_or_else(|| archive_error("extracted size overflow"))?;
    if **total > *maximum {
        return Err(archive_error("extracted size limit exceeded"));
    }
    Ok(())
}

fn set_mode(path: &Path, mode: u32) -> Result<(), LinuxError> {
    let permissions = fs::Permissions::from_mode(mode & 0o777);
    fs::set_permissions(path, permissions).map_err(|_| io_error("set root filesystem mode"))
}

fn safe_entry_path(bytes: &[u8]) -> Result<String, LinuxError> {
    if bytes.is_empty() || bytes.contains(&0) || bytes.contains(&b'\\') {
        return Err(archive_error("archive path rejected"));
    }
    let path =
        std::str::from_utf8(bytes).map_err(|_| archive_error("archive path encoding rejected"))?;
    let parsed = Path::new(path);
    if parsed.is_absolute() || parsed.components().any(|part| part == Component::ParentDir) {
        return Err(archive_error("archive path escapes root"));
    }
    let normalized = parsed
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    if normalized.is_empty() {
        return Err(archive_error("archive path rejected"));
    }
    Ok(normalized)
}

fn archive_error(message: &str) -> LinuxError {
    LinuxError::Archive(message.into())
}
