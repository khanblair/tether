//! Root filesystem cache and repair tests.

mod fixtures;

use std::fs;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use sha2::{Digest, Sha256};
use tempfile::tempdir;
use tether_linux::{NoopProgress, RootfsImage, RootfsManager, RootfsStatus};
use tokio_util::sync::CancellationToken;

fn image(bytes: &[u8]) -> RootfsImage {
    RootfsImage {
        arch: "x86_64".into(),
        url: "https://example.invalid/image".into(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        size_bytes: bytes.len() as u64,
        max_extracted_bytes: 1024,
    }
}

fn simple_archive() -> Vec<u8> {
    fixtures::simple_archive()
}

#[test]
fn repair_fetches_again_when_cached_archive_is_removed() {
    let folder = tempdir().unwrap();
    let bytes = simple_archive();
    let image = image(&bytes);
    let calls = Arc::new(AtomicUsize::new(0));
    let fetcher = fixtures::BytesFetcher {
        bytes: Arc::new(bytes),
        calls: calls.clone(),
    };
    let manager = RootfsManager::new(folder.path().join("linux"));
    manager
        .install(
            &image,
            &fetcher,
            &mut NoopProgress,
            &CancellationToken::new(),
        )
        .unwrap();
    fs::remove_file(
        folder
            .path()
            .join("linux/cache")
            .join(format!("{}.tar.gz", image.sha256)),
    )
    .unwrap();
    fs::remove_file(folder.path().join("linux/rootfs/alpine/bin/busybox")).unwrap();

    manager
        .repair(&fetcher, &mut NoopProgress, &CancellationToken::new())
        .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn repair_leaves_user_added_file_untouched() {
    let folder = tempdir().unwrap();
    let bytes = simple_archive();
    let image = image(&bytes);
    let fetcher = fixtures::BytesFetcher {
        bytes: Arc::new(bytes),
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let manager = RootfsManager::new(folder.path().join("linux"));
    manager
        .install(
            &image,
            &fetcher,
            &mut NoopProgress,
            &CancellationToken::new(),
        )
        .unwrap();
    fs::write(
        folder.path().join("linux/rootfs/alpine/user-file"),
        b"user data",
    )
    .unwrap();
    fs::remove_file(folder.path().join("linux/rootfs/alpine/bin/busybox")).unwrap();

    manager
        .repair(&fetcher, &mut NoopProgress, &CancellationToken::new())
        .unwrap();

    assert_eq!(
        fs::read(folder.path().join("linux/rootfs/alpine/user-file")).unwrap(),
        b"user data"
    );
}

#[test]
fn status_does_not_follow_a_symlinked_rootfs() {
    let folder = tempdir().unwrap();
    let bytes = simple_archive();
    let image = image(&bytes);
    let fetcher = fixtures::BytesFetcher {
        bytes: Arc::new(bytes),
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let manager = RootfsManager::new(folder.path().join("linux"));
    manager
        .install(
            &image,
            &fetcher,
            &mut NoopProgress,
            &CancellationToken::new(),
        )
        .unwrap();
    let outside = folder.path().join("outside");
    fs::rename(folder.path().join("linux/rootfs/alpine"), &outside).unwrap();
    std::os::unix::fs::symlink(&outside, folder.path().join("linux/rootfs/alpine")).unwrap();

    assert_eq!(manager.status().unwrap(), RootfsStatus::NotInstalled);
}

#[test]
fn status_does_not_follow_a_symlinked_state_file() {
    let folder = tempdir().unwrap();
    let bytes = simple_archive();
    let image = image(&bytes);
    let fetcher = fixtures::BytesFetcher {
        bytes: Arc::new(bytes),
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let manager = RootfsManager::new(folder.path().join("linux"));
    manager
        .install(
            &image,
            &fetcher,
            &mut NoopProgress,
            &CancellationToken::new(),
        )
        .unwrap();
    let state = folder.path().join("linux/rootfs/alpine.json");
    let outside = folder.path().join("outside.json");
    fs::rename(&state, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &state).unwrap();

    assert_eq!(manager.status().unwrap(), RootfsStatus::NotInstalled);
}

#[test]
fn repair_does_not_follow_a_symlinked_cache() {
    let folder = tempdir().unwrap();
    let bytes = simple_archive();
    let image = image(&bytes);
    let calls = Arc::new(AtomicUsize::new(0));
    let fetcher = fixtures::BytesFetcher {
        bytes: Arc::new(bytes.clone()),
        calls: calls.clone(),
    };
    let manager = RootfsManager::new(folder.path().join("linux"));
    manager
        .install(
            &image,
            &fetcher,
            &mut NoopProgress,
            &CancellationToken::new(),
        )
        .unwrap();
    let cache = folder
        .path()
        .join("linux/cache")
        .join(format!("{}.tar.gz", image.sha256));
    let outside = folder.path().join("outside.tar.gz");
    fs::rename(&cache, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &cache).unwrap();
    fs::remove_file(folder.path().join("linux/rootfs/alpine/bin/busybox")).unwrap();

    manager
        .repair(&fetcher, &mut NoopProgress, &CancellationToken::new())
        .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
