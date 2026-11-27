//! Archive extraction safety tests.

use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use flate2::{Compression, write::GzEncoder};
use sha2::{Digest, Sha256};
use tar::{Builder, EntryType, Header};
use tempfile::tempdir;
use tether_linux::{
    LinuxError, NoopProgress, RootfsFetcher, RootfsImage, RootfsManager, RootfsProgress,
};
use tokio_util::sync::CancellationToken;

enum Entry {
    Directory(String, u32),
    File(String, Vec<u8>, u32),
    Symlink(String, String),
    HardLink(String, String),
    Special(String, EntryType),
    Raw(Vec<u8>, Vec<u8>),
}

fn archive(entries: Vec<Entry>) -> Vec<u8> {
    let mut compressed = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut builder = Builder::new(&mut compressed);
        for entry in entries {
            append_entry(&mut builder, entry);
        }
        builder.finish().unwrap();
    }
    compressed.finish().unwrap()
}

fn append_entry(builder: &mut Builder<&mut GzEncoder<Vec<u8>>>, entry: Entry) {
    match entry {
        Entry::Directory(path, mode) => {
            let header = header(&path, EntryType::dir(), 0, mode);
            builder.append(&header, &[][..]).unwrap();
        }
        Entry::File(path, bytes, mode) => {
            let header = header(&path, EntryType::Regular, bytes.len() as u64, mode);
            builder.append(&header, bytes.as_slice()).unwrap();
        }
        Entry::Symlink(path, target) => {
            let mut header = header(&path, EntryType::symlink(), 0, 0o777);
            header.set_link_name(target).unwrap();
            header.set_cksum();
            builder.append(&header, &[][..]).unwrap();
        }
        Entry::HardLink(path, target) => {
            let mut header = header(&path, EntryType::Link, 0, 0o755);
            header.set_link_name(target).unwrap();
            header.set_cksum();
            builder.append(&header, &[][..]).unwrap();
        }
        Entry::Special(path, entry_type) => {
            let header = header(&path, entry_type, 0, 0o644);
            builder.append(&header, &[][..]).unwrap();
        }
        Entry::Raw(name, bytes) => {
            let mut header = Header::new_gnu();
            header.as_mut_bytes()[..100].fill(0);
            header.as_mut_bytes()[..name.len()].copy_from_slice(&name);
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append(&header, bytes.as_slice()).unwrap();
        }
    }
}

fn header(path: &str, entry_type: EntryType, size: u64, mode: u32) -> Header {
    let mut header = Header::new_gnu();
    header.set_path(path).unwrap();
    header.set_entry_type(entry_type);
    header.set_size(size);
    header.set_mode(mode);
    header.set_cksum();
    header
}

fn image(bytes: &[u8], max_extracted_bytes: u64) -> RootfsImage {
    RootfsImage {
        arch: "x86_64".into(),
        url: "https://example.invalid/image".into(),
        sha256: hex(&Sha256::digest(bytes)),
        size_bytes: bytes.len() as u64,
        max_extracted_bytes,
    }
}

fn install(
    bytes: Vec<u8>,
    max_extracted_bytes: u64,
) -> (tempfile::TempDir, RootfsManager, RootfsImage) {
    let folder = tempdir().unwrap();
    let manager = RootfsManager::new(folder.path().join("linux"));
    let image = image(&bytes, max_extracted_bytes);
    let fetcher = BytesFetcher {
        bytes: Arc::new(bytes),
    };
    manager
        .install(
            &image,
            &fetcher,
            &mut NoopProgress,
            &CancellationToken::new(),
        )
        .unwrap();
    (folder, manager, image)
}

struct BytesFetcher {
    bytes: Arc<Vec<u8>>,
}

impl RootfsFetcher for BytesFetcher {
    fn fetch(
        &self,
        _image: &RootfsImage,
        output: &mut dyn Write,
        _cancel: &CancellationToken,
    ) -> Result<(), LinuxError> {
        output
            .write_all(&self.bytes)
            .map_err(|_| LinuxError::Fetch("write failed".into()))
    }
}

struct CancellingFetcher {
    bytes: Vec<u8>,
    cancel: CancellationToken,
}

impl RootfsFetcher for CancellingFetcher {
    fn fetch(
        &self,
        _image: &RootfsImage,
        output: &mut dyn Write,
        _cancel: &CancellationToken,
    ) -> Result<(), LinuxError> {
        let midpoint = self.bytes.len() / 2;
        output
            .write_all(&self.bytes[..midpoint])
            .map_err(|_| LinuxError::Fetch("write failed".into()))?;
        self.cancel.cancel();
        Err(LinuxError::Cancelled)
    }
}

struct CancelAfterFirstExtraction {
    cancel: CancellationToken,
    calls: Arc<AtomicUsize>,
}

impl RootfsProgress for CancelAfterFirstExtraction {
    fn extracting(&mut self, _entries: usize) {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            self.cancel.cancel();
        }
    }
}

#[test]
fn extracts_modes_links_and_skips_special_entries() {
    let bytes = archive(vec![
        Entry::Directory("bin".into(), 0o755),
        Entry::File("bin/tool".into(), b"tool".to_vec(), 0o4755),
        Entry::Symlink("bin/absolute".into(), "/bin/tool".into()),
        Entry::Symlink("bin/relative".into(), "tool".into()),
        Entry::HardLink("bin/tool-copy".into(), "bin/tool".into()),
        Entry::Special("dev-null".into(), EntryType::block_special()),
        Entry::Special("named-pipe".into(), EntryType::fifo()),
    ]);
    let (folder, manager, _) = install(bytes, 1024);
    let root = folder.path().join("linux/rootfs/alpine");
    let mode = fs::symlink_metadata(root.join("bin/tool"))
        .unwrap()
        .permissions()
        .mode();

    assert_eq!(mode & 0o7777, 0o755);
    assert_eq!(fs::read(root.join("bin/tool-copy")).unwrap(), b"tool");
    assert_eq!(
        fs::read_link(root.join("bin/absolute")).unwrap(),
        PathBuf::from("/bin/tool")
    );
    assert_eq!(
        fs::read_link(root.join("bin/relative")).unwrap(),
        PathBuf::from("tool")
    );
    assert_eq!(
        fs::symlink_metadata(root.join("bin/tool")).unwrap().ino(),
        fs::symlink_metadata(root.join("bin/tool-copy"))
            .unwrap()
            .ino()
    );
    assert!(!root.join("dev-null").exists());
    assert!(!root.join("named-pipe").exists());
    let state = fs::read_to_string(folder.path().join("linux/rootfs/alpine.json")).unwrap();
    assert!(state.contains("\"skipped_special\": 2"));
    assert!(manager.status().is_ok());
}

#[test]
fn rejects_absolute_archive_path_without_leaving_extracted_files() {
    let bytes = archive(vec![Entry::Raw(b"/outside".to_vec(), b"bad".to_vec())]);
    assert_rejected_archive(bytes, 1024);
}

#[test]
fn rejects_parent_archive_path_without_leaving_extracted_files() {
    let bytes = archive(vec![Entry::Raw(b"../outside".to_vec(), b"bad".to_vec())]);
    assert_rejected_archive(bytes, 1024);
}

#[test]
fn rejects_backslash_archive_path_without_leaving_extracted_files() {
    let bytes = archive(vec![Entry::Raw(b"dir\\escape".to_vec(), b"bad".to_vec())]);
    assert_rejected_archive(bytes, 1024);
}

#[test]
fn rejects_nul_archive_path_without_leaving_extracted_files() {
    let bytes = archive(vec![Entry::Raw(b"dir\0escape".to_vec(), b"bad".to_vec())]);
    assert_rejected_archive(bytes, 1024);
}

#[test]
fn rejects_symlink_then_file_escape_without_leaving_extracted_files() {
    let bytes = archive(vec![
        Entry::Symlink("pivot".into(), "../../outside".into()),
        Entry::File("pivot/secret".into(), b"bad".to_vec(), 0o644),
    ]);
    assert_rejected_archive(bytes, 1024);
}

#[test]
fn rejects_hard_link_to_file_outside_archive_without_leaving_extracted_files() {
    let bytes = archive(vec![Entry::HardLink("copy".into(), "missing".into())]);
    assert_rejected_archive(bytes, 1024);
}

#[test]
fn rejects_duplicate_archive_entries_without_leaving_extracted_files() {
    let bytes = archive(vec![
        Entry::File("same".into(), b"one".to_vec(), 0o644),
        Entry::File("same".into(), b"two".to_vec(), 0o644),
    ]);
    assert_rejected_archive(bytes, 1024);
}

#[test]
fn rejects_file_used_as_directory_without_leaving_extracted_files() {
    let bytes = archive(vec![
        Entry::File("plain".into(), b"one".to_vec(), 0o644),
        Entry::File("plain/child".into(), b"two".to_vec(), 0o644),
    ]);
    assert_rejected_archive(bytes, 1024);
}

#[test]
fn rejects_more_than_entry_limit_without_leaving_extracted_files() {
    let entries = (0..50_001)
        .map(|index| Entry::Special(format!("special-{index}"), EntryType::fifo()))
        .collect();
    assert_rejected_archive(archive(entries), 1024);
}

#[test]
fn rejects_more_than_extracted_size_limit_without_leaving_extracted_files() {
    let bytes = archive(vec![Entry::File("large".into(), b"1234".to_vec(), 0o644)]);
    assert_rejected_archive(bytes, 3);
}

#[test]
fn rejects_archive_hash_mismatch_without_leaving_extracted_files() {
    let bytes = archive(vec![Entry::File("file".into(), b"data".to_vec(), 0o644)]);
    let folder = tempdir().unwrap();
    let manager = RootfsManager::new(folder.path().join("linux"));
    let mut image = image(&bytes, 1024);
    image.sha256 = "b".repeat(64);
    let fetcher = BytesFetcher {
        bytes: Arc::new(bytes),
    };

    assert!(
        manager
            .install(
                &image,
                &fetcher,
                &mut NoopProgress,
                &CancellationToken::new()
            )
            .is_err()
    );
    assert!(
        !folder
            .path()
            .join("linux/cache")
            .join(format!("{}.tar.gz", image.sha256))
            .exists()
    );
}

#[test]
fn rejects_download_size_overrun_without_leaving_partial_cache() {
    let bytes = archive(vec![Entry::File("file".into(), b"data".to_vec(), 0o644)]);
    let folder = tempdir().unwrap();
    let manager = RootfsManager::new(folder.path().join("linux"));
    let mut image = image(&bytes, 1024);
    image.size_bytes -= 1;
    let fetcher = BytesFetcher {
        bytes: Arc::new(bytes),
    };

    assert!(
        manager
            .install(
                &image,
                &fetcher,
                &mut NoopProgress,
                &CancellationToken::new()
            )
            .is_err()
    );
    assert!(
        !folder
            .path()
            .join("linux/cache")
            .join(format!("{}.tar.gz.part", image.sha256))
            .exists()
    );
}

#[test]
fn cancellation_mid_download_removes_partial_cache() {
    let bytes = archive(vec![Entry::File("file".into(), b"data".to_vec(), 0o644)]);
    let folder = tempdir().unwrap();
    let manager = RootfsManager::new(folder.path().join("linux"));
    let image = image(&bytes, 1024);
    let cancel = CancellationToken::new();
    let fetcher = CancellingFetcher {
        bytes,
        cancel: cancel.clone(),
    };

    assert!(matches!(
        manager.install(&image, &fetcher, &mut NoopProgress, &cancel),
        Err(LinuxError::Cancelled)
    ));
    assert!(
        !folder
            .path()
            .join("linux/cache")
            .join(format!("{}.tar.gz.part", image.sha256))
            .exists()
    );
}

#[test]
fn cancellation_mid_extract_removes_partial_rootfs() {
    let bytes = archive(vec![
        Entry::Directory("bin".into(), 0o755),
        Entry::File("bin/one".into(), b"one".to_vec(), 0o755),
        Entry::File("bin/two".into(), b"two".to_vec(), 0o755),
    ]);
    let folder = tempdir().unwrap();
    let manager = RootfsManager::new(folder.path().join("linux"));
    let image = image(&bytes, 1024);
    let token = CancellationToken::new();
    let mut progress = CancelAfterFirstExtraction {
        cancel: token.clone(),
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let fetcher = BytesFetcher {
        bytes: Arc::new(bytes),
    };

    assert!(matches!(
        manager.install(&image, &fetcher, &mut progress, &token),
        Err(LinuxError::Cancelled)
    ));
    assert!(!folder.path().join("linux/rootfs/alpine.partial").exists());
    assert!(!folder.path().join("linux/rootfs/alpine").exists());
}

fn assert_rejected_archive(bytes: Vec<u8>, max_extracted_bytes: u64) {
    let folder = tempdir().unwrap();
    let manager = RootfsManager::new(folder.path().join("linux"));
    let image = image(&bytes, max_extracted_bytes);
    let fetcher = BytesFetcher {
        bytes: Arc::new(bytes),
    };

    assert!(
        manager
            .install(
                &image,
                &fetcher,
                &mut NoopProgress,
                &CancellationToken::new()
            )
            .is_err()
    );
    assert!(!folder.path().join("linux/rootfs/alpine.partial").exists());
    assert!(!folder.path().join("linux/rootfs/alpine").exists());
    assert!(!folder.path().join("linux/rootfs/alpine.json").exists());
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}").expect("writing to a String cannot fail");
        output
    })
}
