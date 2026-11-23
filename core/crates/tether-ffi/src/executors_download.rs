//! Runtime pack and Linux root filesystem downloads.
use std::path::Path;
use std::sync::Arc;

use super::{ExecutorPaths, ExecutorRuntimeError, SessionRuntime, read_paths};
use reqwest::Client;
use tether_linux::{ProotConfig, RootfsImage};
use tether_sandbox::{InstalledPack, PackManifest, install_pack};
use tokio_util::sync::CancellationToken;

#[path = "executors_http.rs"]
mod http;
use http::{HttpPackFetcher, HttpRootfsFetcher, ProgressAdapter, fetch_manifest, http_client};

/// Progress reported by runtime pack and Linux setup operations.
#[uniffi::export(with_foreign)]
pub trait ExecutorProgress: Send + Sync {
    /// Receives downloaded and declared byte counts.
    fn on_progress(&self, downloaded: u64, total: u64);
}
/// One installed or available runtime pack.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RuntimePackView {
    /// Stable pack identifier.
    pub id: String,
    /// Pack version.
    pub version: String,
    /// Whether this version is installed.
    pub installed: bool,
    /// Sum of declared payload file sizes.
    pub size_bytes: u64,
}
#[uniffi::export]
impl SessionRuntime {
    /// Installs one signed runtime pack from a manifest URL.
    ///
    /// # Errors
    /// Returns an error when configuration, signatures, downloads, or storage fail.
    pub fn install_runtime_pack(
        &self,
        pack_id: String,
        manifest_url: String,
        progress: Arc<dyn ExecutorProgress>,
    ) -> Result<(), ExecutorRuntimeError> {
        let paths = read_paths(&self.files_root)?;
        let key = pack_key(&paths)?;
        let client = http_client()?;
        let (manifest_bytes, signature) = self
            .runtime
            .block_on(fetch_manifest(&client, &manifest_url))?;
        let manifest = tether_sandbox::verify_manifest(&manifest_bytes, &signature, &key)
            .map_err(|_| ExecutorRuntimeError::InvalidConfiguration)?;
        let pack = manifest
            .packs
            .iter()
            .find(|pack| pack.id == pack_id)
            .cloned()
            .ok_or(ExecutorRuntimeError::Unavailable)?;
        let cancel = CancellationToken::new();
        let fetcher = HttpPackFetcher {
            client,
            manifest_url,
            progress,
        };
        let root = paths.packs_root;
        let join = self.runtime.spawn_blocking(move || {
            install_pack(
                &PackManifest {
                    format: 1,
                    packs: vec![pack],
                },
                &pack_id,
                &fetcher,
                &root,
                &cancel,
            )
        });
        self.runtime
            .block_on(join)
            .map_err(|_| ExecutorRuntimeError::Unavailable)?
            .map(|_| ())
            .map_err(|_| ExecutorRuntimeError::Unavailable)
    }

    /// Lists installed runtime pack versions.
    ///
    /// # Errors
    /// Returns an error when the configured runtime folder cannot be read.
    pub fn runtime_packs(&self) -> Result<Vec<RuntimePackView>, ExecutorRuntimeError> {
        let paths = read_paths(&self.files_root)?;
        list_packs(&paths.packs_root)
    }

    /// Installs the signed Linux root filesystem and its base packages.
    ///
    /// # Errors
    /// Returns an error when configuration, signatures, downloads, or setup fail.
    pub fn setup_linux_mode(
        &self,
        manifest_url: &str,
        progress: Arc<dyn ExecutorProgress>,
    ) -> Result<(), ExecutorRuntimeError> {
        let paths = read_paths(&self.files_root)?;
        let key = rootfs_key(&paths)?;
        let proot_path = paths
            .proot_path
            .clone()
            .ok_or(ExecutorRuntimeError::InvalidConfiguration)?;
        let client = http_client()?;
        let (manifest_bytes, signature) = self
            .runtime
            .block_on(fetch_manifest(&client, manifest_url))?;
        let image = rootfs_image(&manifest_bytes, &signature, &key)?;
        let config = ProotConfig {
            proot_path,
            loader_path: paths.loader_path,
            linux_root: paths.linux_root,
            workspace: None,
            network: tether_linux::NetworkPolicy::On,
        };
        let inputs = RootfsInputs {
            manifest_bytes,
            signature,
            key,
            image,
        };
        run_rootfs_setup(&self.runtime, config, inputs, client, progress)
    }
}

fn rootfs_image(
    manifest_bytes: &[u8],
    signature: &[u8],
    key: &[u8; 32],
) -> Result<RootfsImage, ExecutorRuntimeError> {
    let manifest = tether_linux::verify_rootfs_manifest(manifest_bytes, signature, key)
        .map_err(|_| ExecutorRuntimeError::InvalidConfiguration)?;
    let arch = match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        "x86_64" => "x86_64",
        _ => return Err(ExecutorRuntimeError::InvalidConfiguration),
    };
    manifest
        .images
        .iter()
        .find(|image| image.arch == arch)
        .cloned()
        .ok_or(ExecutorRuntimeError::Unavailable)
}

/// Verified rootfs manifest data that the setup run needs.
struct RootfsInputs {
    manifest_bytes: Vec<u8>,
    signature: Vec<u8>,
    key: [u8; 32],
    image: RootfsImage,
}

fn run_rootfs_setup(
    runtime: &tokio::runtime::Runtime,
    config: ProotConfig,
    inputs: RootfsInputs,
    client: Client,
    progress: Arc<dyn ExecutorProgress>,
) -> Result<(), ExecutorRuntimeError> {
    let RootfsInputs {
        manifest_bytes,
        signature,
        key,
        image,
    } = inputs;
    let fetcher = HttpRootfsFetcher {
        client,
        progress: Arc::clone(&progress),
    };
    let linux_root = config.linux_root.clone();
    let cancel = CancellationToken::new();
    let join = runtime.spawn_blocking(move || {
        let mut progress_adapter = ProgressAdapter {
            callback: progress,
            total: image.size_bytes,
        };
        let mut request = tether_linux::SetupRequest {
            manifest_bytes: &manifest_bytes,
            signature: &signature,
            key: &key,
            image: &image,
            config: &config,
            fetcher: &fetcher,
            progress: &mut progress_adapter,
            cancel: &cancel,
        };
        tether_linux::setup(&tether_linux::RootfsManager::new(linux_root), &mut request)
    });
    runtime
        .block_on(join)
        .map_err(|_| ExecutorRuntimeError::Unavailable)?
        .map(|_| ())
        .map_err(|_| ExecutorRuntimeError::Unavailable)
}
fn list_packs(root: &Path) -> Result<Vec<RuntimePackView>, ExecutorRuntimeError> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(ExecutorRuntimeError::Unavailable),
    };
    let mut result = Vec::new();
    for id_entry in entries {
        let id_entry = id_entry.map_err(|_| ExecutorRuntimeError::Unavailable)?;
        let versions =
            std::fs::read_dir(id_entry.path()).map_err(|_| ExecutorRuntimeError::Unavailable)?;
        for version_entry in versions {
            let version_entry = version_entry.map_err(|_| ExecutorRuntimeError::Unavailable)?;
            result.extend(version_packs(root, &version_entry.path())?);
        }
    }
    result.sort_by(|left, right| (&left.id, &left.version).cmp(&(&right.id, &right.version)));
    Ok(result)
}

/// Reads the manifest in one version folder and reports each pack it lists.
fn version_packs(
    root: &Path,
    version_dir: &Path,
) -> Result<Vec<RuntimePackView>, ExecutorRuntimeError> {
    let bytes = std::fs::read(version_dir.join("manifest.json"))
        .map_err(|_| ExecutorRuntimeError::Unavailable)?;
    let manifest: PackManifest =
        serde_json::from_slice(&bytes).map_err(|_| ExecutorRuntimeError::InvalidConfiguration)?;
    Ok(manifest
        .packs
        .into_iter()
        .map(|pack| {
            let size_bytes = pack.files.iter().map(|file| file.size_bytes).sum();
            let installed = InstalledPack::open(root, &pack.id, &pack.version)
                .and_then(|installed| installed.verify().map(|()| installed))
                .is_ok();
            RuntimePackView {
                id: pack.id,
                version: pack.version,
                installed,
                size_bytes,
            }
        })
        .collect())
}

fn pack_key(paths: &ExecutorPaths) -> Result<[u8; 32], ExecutorRuntimeError> {
    key_from_hex(paths.pack_key.as_deref())
}

fn rootfs_key(paths: &ExecutorPaths) -> Result<[u8; 32], ExecutorRuntimeError> {
    key_from_hex(paths.rootfs_key.as_deref())
}

fn key_from_hex(value: Option<&str>) -> Result<[u8; 32], ExecutorRuntimeError> {
    let value = value.ok_or(ExecutorRuntimeError::InvalidConfiguration)?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ExecutorRuntimeError::InvalidConfiguration);
    }
    let mut key = [0_u8; 32];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;
        key[index] = (high << 4) | low;
    }
    Ok(key)
}

fn hex_digit(byte: u8) -> Result<u8, ExecutorRuntimeError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ExecutorRuntimeError::InvalidConfiguration),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_parser_decodes_lower_and_upper_hex() {
        let value = key_from_hex(Some(
            "00112233445566778899aabbccddeeff00112233445566778899AABBCCDDEEFF",
        ))
        .unwrap();
        assert_eq!(value[0], 0x00);
        assert_eq!(value[15], 0xff);
        assert_eq!(value[31], 0xff);
    }
}
