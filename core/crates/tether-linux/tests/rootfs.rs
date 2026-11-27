//! Root filesystem installation integration tests.

mod fixtures;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use flate2::{Compression, write::GzEncoder};
use sha2::{Digest, Sha256};
use tar::{Builder, EntryType, Header};
use tempfile::tempdir;
use tether_linux::{NoopProgress, RootfsImage, RootfsManager, RootfsStatus};
use tokio_util::sync::CancellationToken;

fn archive() -> Vec<u8> {
    let mut compressed = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut builder = Builder::new(&mut compressed);
        let mut directory = Header::new_gnu();
        directory.set_path("bin").unwrap();
        directory.set_entry_type(EntryType::dir());
        directory.set_size(0);
        directory.set_mode(0o755);
        directory.set_cksum();
        builder.append(&directory, &[][..]).unwrap();
        let bytes = b"busybox";
        let mut header = Header::new_gnu();
        header.set_path("bin/busybox").unwrap();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder.append(&header, &bytes[..]).unwrap();
        builder.finish().unwrap();
    }
    compressed.finish().unwrap()
}

#[test]
fn installs_archive_and_reuses_verified_cache() {
    let directory = tempdir().unwrap();
    let bytes = archive();
    let digest = hex(&Sha256::digest(&bytes));
    let image = RootfsImage {
        arch: "x86_64".into(),
        url: "https://example.invalid/image".into(),
        sha256: digest,
        size_bytes: bytes.len() as u64,
        max_extracted_bytes: 1024,
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let fetcher = fixtures::BytesFetcher {
        bytes: Arc::new(bytes),
        calls: calls.clone(),
    };
    let manager = RootfsManager::new(directory.path().join("linux"));
    let token = CancellationToken::new();
    let mut progress = NoopProgress;

    manager
        .install(&image, &fetcher, &mut progress, &token)
        .unwrap();
    assert_eq!(
        fs::read(directory.path().join("linux/rootfs/alpine/bin/busybox")).unwrap(),
        b"busybox"
    );
    assert!(matches!(
        manager.status().unwrap(),
        RootfsStatus::Installed { .. }
    ));
    fs::remove_file(directory.path().join("linux/rootfs/alpine/bin/busybox")).unwrap();
    assert_eq!(
        manager.status().unwrap(),
        RootfsStatus::NeedsRepair {
            missing: 1,
            changed: 0
        }
    );

    manager.repair(&fetcher, &mut progress, &token).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        fs::metadata(directory.path().join("linux/rootfs/alpine/bin/busybox"))
            .unwrap()
            .permissions()
            .mode()
            & 0o111,
        0o111
    );
}

#[test]
fn rejects_archive_path_escape_and_cleans_staging() {
    let mut compressed = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut builder = Builder::new(&mut compressed);
        let bytes = b"bad";
        let mut header = Header::new_gnu();
        header.as_mut_bytes()[..9].copy_from_slice(b"../escape");
        header.as_mut_bytes()[9] = 0;
        header.set_size(bytes.len() as u64);
        header.set_cksum();
        builder.append(&header, &bytes[..]).unwrap();
        builder.finish().unwrap();
    }
    let bytes = compressed.finish().unwrap();
    let digest = hex(&Sha256::digest(&bytes));
    let image = RootfsImage {
        arch: "x86_64".into(),
        url: "https://example.invalid/image".into(),
        sha256: digest,
        size_bytes: bytes.len() as u64,
        max_extracted_bytes: 1024,
    };
    let directory = tempdir().unwrap();
    let manager = RootfsManager::new(directory.path().join("linux"));
    let fetcher = fixtures::BytesFetcher {
        bytes: Arc::new(bytes),
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let mut progress = NoopProgress;

    assert!(
        manager
            .install(&image, &fetcher, &mut progress, &CancellationToken::new())
            .is_err()
    );
    assert!(
        !directory
            .path()
            .join("linux/rootfs/alpine.partial")
            .exists()
    );
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}").expect("writing to a String cannot fail");
        output
    })
}
