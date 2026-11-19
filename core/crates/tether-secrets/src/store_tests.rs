//! Tests for the secret store, using a fake platform in place of the Keystore.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{PlatformError, Secret, SecretRef, SecretStore, SecretsError, SecretsPlatform};

const KEY_TEXT: &str = "sk-ant-api03-TESTVALUE-not-a-real-key";

/// A stand-in for the Keystore. It is not encryption: it scrambles the bytes and appends a
/// tag over the context, so tests can check that the store passes the context through.
#[derive(Default)]
struct FakePlatform {
    wraps: AtomicUsize,
    fail: bool,
}

fn tag(context: &[u8]) -> u8 {
    context.iter().fold(0x5a, |acc, b| acc.wrapping_add(*b))
}

impl SecretsPlatform for FakePlatform {
    fn wrap(&self, plaintext: &[u8], context: &[u8]) -> Result<Vec<u8>, PlatformError> {
        self.wraps.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(PlatformError {
                message: "keystore unavailable".into(),
            });
        }
        let mut out: Vec<u8> = plaintext.iter().map(|b| b ^ 0xa5).collect();
        out.push(tag(context));
        Ok(out)
    }

    fn unwrap(&self, wrapped: &[u8], context: &[u8]) -> Result<Vec<u8>, PlatformError> {
        let (body, found) = wrapped.split_at(wrapped.len().saturating_sub(1));
        if found != [tag(context)] {
            return Err(PlatformError {
                message: "authentication failed".into(),
            });
        }
        Ok(body.iter().map(|b| b ^ 0xa5).collect())
    }
}

fn reference(name: &str) -> SecretRef {
    SecretRef::new(name).unwrap()
}

fn store_in(dir: &tempfile::TempDir) -> SecretStore {
    SecretStore::new(
        dir.path().join("secrets"),
        Arc::new(FakePlatform::default()),
    )
}

#[test]
fn a_stored_secret_is_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(&dir);
    store
        .put(
            &reference("provider-anthropic"),
            &Secret::from_text(KEY_TEXT),
        )
        .unwrap();
    let back = store
        .get(&reference("provider-anthropic"))
        .unwrap()
        .unwrap();
    assert_eq!(back.expose_text().unwrap(), KEY_TEXT);
}

#[test]
fn a_new_store_over_the_same_folder_reads_the_secret_like_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    store_in(&dir)
        .put(&reference("github-token"), &Secret::from_text(KEY_TEXT))
        .unwrap();
    let restarted = store_in(&dir);
    let back = restarted.get(&reference("github-token")).unwrap().unwrap();
    assert_eq!(back.expose_text().unwrap(), KEY_TEXT);
}

#[test]
fn the_file_does_not_contain_the_secret() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(&dir);
    store
        .put(
            &reference("provider-anthropic"),
            &Secret::from_text(KEY_TEXT),
        )
        .unwrap();
    let bytes = fs::read(dir.path().join("secrets").join("provider-anthropic.bin")).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("TESTVALUE"));
    assert!(
        !bytes
            .windows(KEY_TEXT.len())
            .any(|w| w == KEY_TEXT.as_bytes())
    );
    assert!(bytes.starts_with(b"TSEC"));
}

#[test]
fn a_missing_secret_is_none_and_deleting_it_is_fine() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(&dir);
    assert!(store.get(&reference("nothing")).unwrap().is_none());
    assert!(!store.exists(&reference("nothing")).unwrap());
    store.delete(&reference("nothing")).unwrap();
}

#[test]
fn delete_removes_the_secret() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(&dir);
    store.put(&reference("k"), &Secret::from_text("v")).unwrap();
    assert!(store.exists(&reference("k")).unwrap());
    store.delete(&reference("k")).unwrap();
    assert!(!store.exists(&reference("k")).unwrap());
}

#[test]
fn putting_again_replaces_the_secret_and_leaves_no_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(&dir);
    store
        .put(&reference("k"), &Secret::from_text("first"))
        .unwrap();
    store
        .put(&reference("k"), &Secret::from_text("second"))
        .unwrap();
    let back = store.get(&reference("k")).unwrap().unwrap();
    assert_eq!(back.expose_text().unwrap(), "second");
    let names: Vec<_> = fs::read_dir(dir.path().join("secrets"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, vec!["k.bin".to_owned()]);
}

#[test]
fn a_file_moved_to_another_reference_does_not_unwrap() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(&dir);
    store
        .put(&reference("github-token"), &Secret::from_text(KEY_TEXT))
        .unwrap();
    let folder = dir.path().join("secrets");
    fs::copy(
        folder.join("github-token.bin"),
        folder.join("provider-anthropic.bin"),
    )
    .unwrap();
    let result = store.get(&reference("provider-anthropic"));
    assert!(matches!(result, Err(SecretsError::Platform { .. })));
}

#[test]
fn a_platform_failure_stores_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let platform = FakePlatform {
        fail: true,
        ..FakePlatform::default()
    };
    let store = SecretStore::new(dir.path().join("secrets"), Arc::new(platform));
    let result = store.put(&reference("k"), &Secret::from_text("v"));
    assert!(matches!(result, Err(SecretsError::Platform { .. })));
    assert!(!dir.path().join("secrets").join("k.bin").exists());
}

fn write_raw(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) {
    let folder = dir.path().join("secrets");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join(name), bytes).unwrap();
}

#[test]
fn damaged_files_are_reported_as_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(&dir);
    let cases: [(&str, &[u8]); 4] = [
        ("empty.bin", b""),
        ("short.bin", b"TSE"),
        ("magic.bin", b"XXXX\x01data"),
        ("version.bin", b"TSEC\x09data"),
    ];
    for (file, bytes) in cases {
        write_raw(&dir, file, bytes);
        let name = file.trim_end_matches(".bin");
        let result = store.get(&reference(name));
        assert!(
            matches!(result, Err(SecretsError::Corrupt { .. })),
            "{file}"
        );
    }
}

#[test]
fn an_invalid_reference_never_reaches_the_file_system() {
    let result = SecretRef::new("../escape");
    assert!(matches!(result, Err(SecretsError::InvalidReference { .. })));
}

#[test]
fn debug_output_never_shows_the_secret() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(&dir);
    let secret = Secret::from_text(KEY_TEXT);
    store.put(&reference("k"), &secret).unwrap();
    let shown = format!("{store:?} {secret:?}");
    assert!(!shown.contains("TESTVALUE"));
}

#[cfg(unix)]
#[test]
fn secret_files_are_private_to_the_owner() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(&dir);
    store.put(&reference("k"), &Secret::from_text("v")).unwrap();
    let mode = fs::metadata(dir.path().join("secrets").join("k.bin"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

fn spawn_put(
    store: &Arc<SecretStore>,
    n: usize,
) -> std::thread::JoinHandle<Result<(), SecretsError>> {
    let store = Arc::clone(store);
    std::thread::spawn(move || {
        let secret = Secret::from_text(&format!("{KEY_TEXT}-{n}-{}", "x".repeat(n * 50)));
        store.put(&reference("k"), &secret)
    })
}

#[test]
fn concurrent_puts_to_one_reference_leave_one_complete_secret() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(store_in(&dir));
    for _ in 0..20 {
        let writers: Vec<_> = (0..8).map(|n| spawn_put(&store, n)).collect();
        for writer in writers {
            writer.join().unwrap().unwrap();
        }
        let back = store.get(&reference("k")).unwrap().unwrap();
        assert!(back.expose_text().unwrap().starts_with(KEY_TEXT));
    }
    let names: Vec<_> = fs::read_dir(dir.path().join("secrets"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, vec!["k.bin".to_owned()]);
}
