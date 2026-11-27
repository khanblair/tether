//! Integration tests for the manifest command line tool.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use tempfile::{TempDir, tempdir};
use tether_linux::verify_rootfs_manifest;

const FIXTURE: &[u8] = b"small rootfs archive fixture";

fn tool() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tether-linux-manifest"))
}

fn keygen(directory: &TempDir) -> (PathBuf, String) {
    let (path, public_key, _) = keygen_with_output(directory);
    (path, public_key)
}

fn keygen_with_output(directory: &TempDir) -> (PathBuf, String, String) {
    let path = directory.path().join("linux.key");
    let output = tool()
        .args(["keygen", "--out"])
        .arg(&path)
        .output()
        .expect("keygen should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("keygen output should be UTF-8");
    let public_key = stdout
        .lines()
        .find_map(|line| line.strip_prefix("public key: "))
        .expect("keygen should print the public key")
        .to_owned();
    (path, public_key, stdout)
}

fn archive(directory: &TempDir) -> PathBuf {
    let path = directory.path().join("rootfs.tar.gz");
    fs::write(&path, FIXTURE).expect("fixture should be writable");
    path
}

fn sign(directory: &TempDir, key: &Path, rootfs: &Path, url: &str) -> PathBuf {
    let output = tool()
        .args(["sign", "--key"])
        .arg(key)
        .args(["--rootfs"])
        .arg(rootfs)
        .args(["--url", url, "--alpine-version", "3.22.2", "--out-dir"])
        .arg(directory.path())
        .output()
        .expect("sign should run");
    assert!(output.status.success(), "sign failed: {output:?}");
    directory.path().join("manifest.json")
}

fn verify(public_key: &str, manifest: &Path, signature: &Path) -> Output {
    tool()
        .args(["verify", "--public-key", public_key, "--manifest"])
        .arg(manifest)
        .args(["--sig"])
        .arg(signature)
        .output()
        .expect("verify should run")
}

#[test]
#[cfg(unix)]
fn keygen_writes_a_private_key_with_restricted_permissions_and_refuses_overwrite() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().expect("temporary directory should exist");
    let (path, _) = keygen(&directory);
    let contents = fs::read_to_string(&path).expect("private key should be readable");
    assert_eq!(contents.len(), 64);
    assert!(contents.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(
        fs::metadata(&path)
            .expect("metadata should exist")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let second = tool()
        .args(["keygen", "--out"])
        .arg(&path)
        .output()
        .expect("second keygen should run");
    assert!(!second.status.success());
    assert_eq!(
        fs::read_to_string(path).expect("private key should remain"),
        contents
    );
}

#[test]
fn keygen_prints_only_the_public_key_and_path() {
    let directory = tempdir().expect("temporary directory should exist");
    let (path, _, stdout) = keygen_with_output(&directory);
    let private_key = fs::read_to_string(path).expect("private key should be readable");
    let shown_public = stdout
        .lines()
        .find_map(|line| line.strip_prefix("public key: "))
        .expect("keygen should print a public key");
    assert!(stdout.contains(shown_public));
    assert!(!stdout.contains(&private_key));
    assert!(stdout.contains(directory.path().to_str().expect("path should be UTF-8")));
}

#[test]
fn sign_and_verify_round_trip() {
    let directory = tempdir().expect("temporary directory should exist");
    let (key, public_key) = keygen(&directory);
    let rootfs = archive(&directory);
    let manifest = sign(
        &directory,
        &key,
        &rootfs,
        "https://example.invalid/rootfs.tar.gz",
    );
    let output = verify(
        &public_key,
        &manifest,
        &directory.path().join("manifest.json.sig"),
    );
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "valid");
}

#[test]
fn tampered_manifest_and_wrong_public_key_fail_verification() {
    let directory = tempdir().expect("temporary directory should exist");
    let (key, public_key) = keygen(&directory);
    let rootfs = archive(&directory);
    let manifest = sign(
        &directory,
        &key,
        &rootfs,
        "https://example.invalid/rootfs.tar.gz",
    );
    let signature = directory.path().join("manifest.json.sig");
    let original = fs::read(&manifest).expect("manifest should be readable");
    let mut bytes = original.clone();
    bytes[0] = b'[';
    fs::write(&manifest, bytes).expect("manifest should be writable");
    assert!(!verify(&public_key, &manifest, &signature).status.success());
    fs::write(&manifest, original).expect("manifest should be writable");
    assert!(
        !verify(&"00".repeat(32), &manifest, &signature)
            .status
            .success()
    );
}

#[test]
fn sign_records_the_archive_digest_and_size() {
    let directory = tempdir().expect("temporary directory should exist");
    let (key, _) = keygen(&directory);
    let rootfs = archive(&directory);
    sign(
        &directory,
        &key,
        &rootfs,
        "https://example.invalid/rootfs.tar.gz",
    );
    let manifest: tether_linux::RootfsManifest = serde_json::from_slice(
        &fs::read(directory.path().join("manifest.json")).expect("manifest should exist"),
    )
    .expect("manifest should be JSON");
    assert_eq!(manifest.images[0].sha256, hex(&Sha256::digest(FIXTURE)));
    assert_eq!(manifest.images[0].size_bytes, FIXTURE.len() as u64);
}

#[test]
fn sign_rejects_http_urls() {
    let directory = tempdir().expect("temporary directory should exist");
    let (key, _) = keygen(&directory);
    let rootfs = archive(&directory);
    let output = tool()
        .args(["sign", "--key"])
        .arg(key)
        .args(["--rootfs"])
        .arg(rootfs)
        .args([
            "--url",
            "http://example.invalid/rootfs.tar.gz",
            "--alpine-version",
            "3.22.2",
            "--out-dir",
        ])
        .arg(directory.path().join("output"))
        .output()
        .expect("sign should run");
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private"));
}

#[test]
fn signature_file_verifies_with_the_public_key_bytes_used_by_the_app() {
    let directory = tempdir().expect("temporary directory should exist");
    let (key_path, public_hex) = keygen(&directory);
    let rootfs = archive(&directory);
    let manifest_path = sign(
        &directory,
        &key_path,
        &rootfs,
        "https://example.invalid/rootfs.tar.gz",
    );
    let manifest = fs::read(manifest_path).expect("manifest should exist");
    let signature =
        fs::read(directory.path().join("manifest.json.sig")).expect("signature should exist");
    let private_hex = fs::read_to_string(key_path).expect("key exists");
    let key_bytes = (0..public_hex.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&public_hex[index..index + 2], 16).expect("public key is hex")
        })
        .collect::<Vec<_>>();
    let key: [u8; 32] = key_bytes.try_into().expect("public key should be 32 bytes");
    assert!(verify_rootfs_manifest(&manifest, &signature, &key).is_ok());
    let signing_key = SigningKey::from_bytes(
        &(0..64)
            .step_by(2)
            .map(|index| {
                u8::from_str_radix(&private_hex[index..index + 2], 16).expect("key is hex")
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("private key should be 32 bytes"),
    );
    assert_eq!(signing_key.verifying_key().to_bytes(), key);
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}").expect("writing to a String cannot fail");
        output
    })
}
