//! Signed runtime pack manifests and streaming verification.

use std::fmt;
use std::io::Write;

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

/// A signed collection of runtime packs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    /// Manifest format version.
    pub format: u32,
    /// Runtime packs described by this manifest.
    pub packs: Vec<PackEntry>,
}

/// One runtime pack and its executable entry module.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackEntry {
    /// Stable pack identifier.
    pub id: String,
    /// Pack version.
    pub version: String,
    /// Program names provided by this pack.
    pub programs: Vec<String>,
    /// Relative path to the entry WASM module.
    pub entry: String,
    /// Files included in the pack.
    pub files: Vec<PackFile>,
}

/// One file listed in a runtime pack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackFile {
    /// Relative slash-separated path inside the pack.
    pub path: String,
    /// Exact file size in bytes.
    pub size_bytes: u64,
    /// Lowercase SHA-256 digest.
    pub sha256: String,
}

/// A typed failure while verifying or installing a runtime pack.
#[derive(Debug)]
pub enum PackError {
    /// The detached signature was invalid.
    Signature,
    /// The manifest was malformed or failed validation.
    Manifest(String),
    /// A file digest did not match its manifest entry.
    HashMismatch {
        /// Pack identifier.
        id: String,
        /// Relative file path.
        path: String,
    },
    /// A file size did not match its manifest entry.
    Size {
        /// Pack identifier.
        id: String,
        /// Relative file path.
        path: String,
    },
    /// A pack or file path violated containment rules.
    Path {
        /// Pack identifier.
        id: String,
        /// Relative file path.
        path: String,
    },
    /// A file operation failed.
    Io {
        /// Pack identifier.
        id: String,
        /// Relative file path.
        path: String,
    },
    /// The caller cancelled the operation.
    Cancelled {
        /// Pack identifier.
        id: String,
        /// Relative file path.
        path: String,
    },
    /// A requested pack or program was not found.
    NotFound {
        /// Pack identifier.
        id: String,
        /// Relative file path.
        path: String,
    },
}

impl fmt::Display for PackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Signature => write!(formatter, "runtime pack signature verification failed"),
            Self::Manifest(message) => {
                write!(formatter, "runtime pack manifest invalid: {message}")
            }
            Self::HashMismatch { id, path } => {
                write!(formatter, "pack {id} file {path} hash mismatch")
            }
            Self::Size { id, path } => write!(formatter, "pack {id} file {path} size mismatch"),
            Self::Path { id, path } => write!(formatter, "pack {id} file {path} path rejected"),
            Self::Io { id, path } => write!(formatter, "pack {id} file {path} I/O failed"),
            Self::Cancelled { id, path } => write!(formatter, "pack {id} file {path} cancelled"),
            Self::NotFound { id, path } => write!(formatter, "pack {id} file {path} not found"),
        }
    }
}

impl std::error::Error for PackError {}

/// Supplies one manifest file to the installer.
pub trait PackFetcher {
    /// Streams one file into `out`.
    ///
    /// # Errors
    /// Returns a fetch, I/O, or cancellation failure.
    fn fetch(
        &self,
        pack: &PackEntry,
        file: &PackFile,
        out: &mut dyn Write,
        cancel: &CancellationToken,
    ) -> Result<(), PackError>;
}

/// Verifies a detached Ed25519 signature and then validates its JSON manifest.
///
/// # Errors
/// Returns a signature or manifest validation error.
pub fn verify_manifest(
    manifest_bytes: &[u8],
    signature: &[u8],
    key: &[u8; 32],
) -> Result<PackManifest, PackError> {
    let verifying_key = VerifyingKey::from_bytes(key).map_err(|_| PackError::Signature)?;
    let signature = Signature::from_slice(signature).map_err(|_| PackError::Signature)?;
    verifying_key
        .verify_strict(manifest_bytes, &signature)
        .map_err(|_| PackError::Signature)?;
    let manifest: PackManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| PackError::Manifest(error.to_string()))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub(crate) fn validate_manifest(manifest: &PackManifest) -> Result<(), PackError> {
    if manifest.format != 1 {
        return Err(PackError::Manifest("unsupported format".into()));
    }
    let mut ids = std::collections::BTreeSet::new();
    for pack in &manifest.packs {
        validate_name(&pack.id)
            .map_err(|message| PackError::Manifest(format!("{}: {message}", pack.id)))?;
        validate_name(&pack.version)
            .map_err(|message| PackError::Manifest(format!("{}: {message}", pack.id)))?;
        if !ids.insert((&pack.id, &pack.version)) {
            return Err(PackError::Manifest(format!(
                "duplicate pack {} {}",
                pack.id, pack.version
            )));
        }
        if pack.files.is_empty() || pack.files.len() > 256 {
            return Err(PackError::Manifest(format!(
                "{}: file count rejected",
                pack.id
            )));
        }
        let mut paths = std::collections::BTreeSet::new();
        for file in &pack.files {
            validate_file(&pack.id, file, &mut paths)?;
        }
        if !paths.contains(&pack.entry) {
            return Err(PackError::Manifest(format!(
                "{}: entry not listed",
                pack.id
            )));
        }
    }
    Ok(())
}

fn validate_file(
    id: &str,
    file: &PackFile,
    paths: &mut std::collections::BTreeSet<String>,
) -> Result<(), PackError> {
    validate_path(id, &file.path)?;
    if file.path == "manifest.json" {
        return Err(PackError::Path {
            id: id.into(),
            path: file.path.clone(),
        });
    }
    if !paths.insert(file.path.clone()) {
        return Err(PackError::Manifest(format!(
            "{id}: duplicate file {}",
            file.path
        )));
    }
    if file.size_bytes > 512 * 1024 * 1024 {
        return Err(PackError::Manifest(format!(
            "{id}: file {} too large",
            file.path
        )));
    }
    if file.sha256.len() != 64
        || !file
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(PackError::Manifest(format!(
            "{id}: hash for {} rejected",
            file.path
        )));
    }
    Ok(())
}

pub(crate) fn validate_name(name: &str) -> Result<(), &'static str> {
    if !(1..=64).contains(&name.len())
        || !name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        return Err("name rejected");
    }
    Ok(())
}

pub(crate) fn validate_path(id: &str, path: &str) -> Result<(), PackError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(PackError::Path {
            id: id.into(),
            path: path.into(),
        });
    }
    Ok(())
}
