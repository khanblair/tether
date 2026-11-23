//! Manifest validation tests.

use ed25519_dalek::{Signer, SigningKey};
use serde_json::{Value, json};
use tether_linux::verify_rootfs_manifest;

fn signed_manifest(mut change: impl FnMut(&mut Value)) -> (Vec<u8>, [u8; 64], [u8; 32]) {
    let key = SigningKey::from_bytes(&[9_u8; 32]);
    let mut value = json!({
        "format": 1,
        "alpine_version": "3.20",
        "images": [{
            "arch": "x86_64",
            "url": "https://example.invalid/alpine.tar.gz",
            "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "size_bytes": 3,
            "max_extracted_bytes": 100
        }],
        "base_packages": ["busybox"]
    });
    change(&mut value);
    let bytes = serde_json::to_vec(&value).unwrap();
    let signature = key.sign(&bytes).to_bytes();
    (bytes, signature, key.verifying_key().to_bytes())
}

#[test]
fn flipped_manifest_byte_is_rejected() {
    let (mut bytes, signature, key) = signed_manifest(|_| {});
    bytes[10] ^= 1;

    assert!(verify_rootfs_manifest(&bytes, &signature, &key).is_err());
}

#[test]
fn flipped_manifest_signature_byte_is_rejected() {
    let (bytes, mut signature, key) = signed_manifest(|_| {});
    signature[0] ^= 1;

    assert!(verify_rootfs_manifest(&bytes, &signature, &key).is_err());
}

#[test]
fn wrong_manifest_key_is_rejected() {
    let (bytes, signature, _) = signed_manifest(|_| {});
    let wrong = [4_u8; 32];

    assert!(verify_rootfs_manifest(&bytes, &signature, &wrong).is_err());
}

#[test]
fn http_image_url_is_rejected() {
    let (bytes, signature, key) = signed_manifest(|value| {
        value["images"][0]["url"] = Value::String("http://example.invalid/image".into());
    });

    assert!(verify_rootfs_manifest(&bytes, &signature, &key).is_err());
}

#[test]
fn bad_base_package_name_is_rejected() {
    let (bytes, signature, key) = signed_manifest(|value| {
        value["base_packages"][0] = Value::String("BusyBox".into());
    });

    assert!(verify_rootfs_manifest(&bytes, &signature, &key).is_err());
}

#[test]
fn too_many_base_packages_are_rejected() {
    let (bytes, signature, key) = signed_manifest(|value| {
        value["base_packages"] = Value::Array(
            (0..101)
                .map(|index| Value::String(format!("pkg{index}")))
                .collect(),
        );
    });

    assert!(verify_rootfs_manifest(&bytes, &signature, &key).is_err());
}

#[test]
fn oversized_image_is_rejected() {
    let (bytes, signature, key) = signed_manifest(|value| {
        value["images"][0]["size_bytes"] = Value::from(200_u64 * 1024 * 1024 + 1);
    });

    assert!(verify_rootfs_manifest(&bytes, &signature, &key).is_err());
}
