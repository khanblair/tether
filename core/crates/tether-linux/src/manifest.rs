use std::io::Write;

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::LinuxError;

/// The signed description of the Alpine root filesystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RootfsManifest {
    /// Manifest format number.
    pub format: u32,
    /// Alpine release version.
    pub alpine_version: String,
    /// Downloadable root filesystem images.
    pub images: Vec<RootfsImage>,
    /// Packages installed during setup.
    pub base_packages: Vec<String>,
}

/// One architecture-specific Alpine root filesystem archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RootfsImage {
    /// Target architecture.
    pub arch: String,
    /// HTTPS download URL.
    pub url: String,
    /// Lowercase SHA-256 digest of the archive.
    pub sha256: String,
    /// Exact compressed archive size.
    pub size_bytes: u64,
    /// Maximum total uncompressed regular-file bytes.
    pub max_extracted_bytes: u64,
}

/// Supplies a root filesystem archive without coupling this crate to HTTP.
pub trait RootfsFetcher {
    /// Streams the declared image into `out`.
    ///
    /// # Errors
    /// Returns a fetch, write, or cancellation failure.
    fn fetch(
        &self,
        image: &RootfsImage,
        out: &mut dyn Write,
        cancel: &CancellationToken,
    ) -> Result<(), LinuxError>;
}

/// Verifies a detached signature before parsing and validating the manifest.
///
/// # Errors
/// Returns a signature or manifest validation error.
pub fn verify_rootfs_manifest(
    manifest_bytes: &[u8],
    signature_bytes: &[u8],
    key: &[u8; 32],
) -> Result<RootfsManifest, LinuxError> {
    let verifying_key = VerifyingKey::from_bytes(key).map_err(|_| LinuxError::Signature)?;
    let signature = Signature::from_slice(signature_bytes).map_err(|_| LinuxError::Signature)?;
    verifying_key
        .verify_strict(manifest_bytes, &signature)
        .map_err(|_| LinuxError::Signature)?;
    let manifest: RootfsManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| LinuxError::Manifest(error.to_string()))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub(crate) fn validate_manifest(manifest: &RootfsManifest) -> Result<(), LinuxError> {
    if manifest.format != 1 {
        return Err(LinuxError::Manifest("unsupported format".into()));
    }
    if manifest.images.is_empty() {
        return Err(LinuxError::Manifest("no images declared".into()));
    }
    if manifest.base_packages.len() > 100 {
        return Err(LinuxError::Manifest("too many base packages".into()));
    }
    for package in &manifest.base_packages {
        validate_package(package)?;
    }
    for image in &manifest.images {
        validate_image(image)?;
    }
    Ok(())
}

fn validate_image(image: &RootfsImage) -> Result<(), LinuxError> {
    if image.arch != "aarch64" && image.arch != "x86_64" {
        return Err(LinuxError::Manifest(
            "unsupported image architecture".into(),
        ));
    }
    let host = image.url.strip_prefix("https://").unwrap_or_default();
    if host.is_empty()
        || host.starts_with('/')
        || image.url.len() <= "https://".len()
        || image
            .url
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte == 0)
    {
        return Err(LinuxError::Manifest("image URL must be HTTPS".into()));
    }
    if !is_lower_hex(&image.sha256, 64) {
        return Err(LinuxError::Manifest("image hash rejected".into()));
    }
    if image.size_bytes > 200 * 1024 * 1024 {
        return Err(LinuxError::Manifest("image is too large".into()));
    }
    if image.max_extracted_bytes > 1024 * 1024 * 1024 {
        return Err(LinuxError::Manifest("extracted image is too large".into()));
    }
    Ok(())
}

fn validate_package(package: &str) -> Result<(), LinuxError> {
    let valid = (1..=64).contains(&package.len())
        && package.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || (index > 0 && b"+_.-".contains(&byte))
        });
    if valid {
        Ok(())
    } else {
        Err(LinuxError::Manifest("base package name rejected".into()))
    }
}

pub(crate) fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn sha256_file(path: &std::path::Path) -> Result<String, LinuxError> {
    let mut file = std::fs::File::open(path).map_err(|_| crate::error::io_error("open archive"))?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
    loop {
        let count = std::io::Read::read(&mut file, &mut buffer)
            .map_err(|_| crate::error::io_error("read archive"))?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(hex_digest(&hash.finalize()))
}

pub(crate) fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                HEX[usize::from(byte >> 4)] as char,
                HEX[usize::from(byte & 15)] as char,
            ]
        })
        .collect()
}
