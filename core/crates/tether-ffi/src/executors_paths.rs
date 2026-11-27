//! Path and key checks for the local executors, read from `executors/paths.conf`.

use std::fs;
use std::path::{Path, PathBuf};

use super::{ExecutorPathsView, ExecutorRuntimeError};

#[derive(Debug)]
pub(super) struct ExecutorPaths {
    pub(super) packs_root: PathBuf,
    pub(super) linux_root: PathBuf,
    pub(super) proot_path: Option<PathBuf>,
    pub(super) loader_path: PathBuf,
    pub(super) pack_key: Option<String>,
    pub(super) rootfs_key: Option<String>,
}

pub(super) fn files_root(database_path: &Path) -> PathBuf {
    let database_dir = database_path.parent().unwrap_or_else(|| Path::new("."));
    if database_dir.file_name().and_then(|name| name.to_str()) == Some("db") {
        database_dir.parent().unwrap_or(database_dir).to_owned()
    } else {
        database_dir.to_owned()
    }
}

pub(super) fn validate_paths(paths: &ExecutorPathsView) -> Result<(), ExecutorRuntimeError> {
    let values = [
        paths.packs_root.as_str(),
        paths.linux_root.as_str(),
        paths.proot_path.as_deref().unwrap_or(""),
        paths.proot_loader_path.as_deref().unwrap_or(""),
    ];
    if values.iter().any(|value| {
        value
            .bytes()
            .any(|byte| byte == 0 || byte == b'\n' || byte == b'\r')
    }) {
        return Err(ExecutorRuntimeError::InvalidConfiguration);
    }
    if paths.proot_path.is_some() != paths.proot_loader_path.is_some()
        || [
            paths.proot_path.as_deref(),
            paths.proot_loader_path.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|path| !Path::new(path).is_absolute())
    {
        return Err(ExecutorRuntimeError::InvalidConfiguration);
    }
    if [
        paths.pack_verifying_key_hex.as_deref(),
        paths.rootfs_verifying_key_hex.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|key| key.len() != 64 || !key.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err(ExecutorRuntimeError::InvalidConfiguration);
    }
    Ok(())
}

pub(super) fn read_paths(files_root: &Path) -> Result<ExecutorPaths, ExecutorRuntimeError> {
    let config = files_root.join("executors/paths.conf");
    let text = fs::read_to_string(config).map_err(|_| ExecutorRuntimeError::Unavailable)?;
    let mut packs_root = None;
    let mut linux_root = None;
    let mut proot_path = None;
    let mut loader_path = PathBuf::new();
    let mut pack_key = None;
    let mut rootfs_key = None;
    for line in text.lines() {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        match name {
            "packs_root" => packs_root = Some(PathBuf::from(value)),
            "linux_root" => linux_root = Some(PathBuf::from(value)),
            "proot_path" => proot_path = Some(PathBuf::from(value)),
            "loader_path" => loader_path = PathBuf::from(value),
            "pack_key" => pack_key = Some(value.to_owned()),
            "rootfs_key" => rootfs_key = Some(value.to_owned()),
            _ => {}
        }
    }
    Ok(ExecutorPaths {
        packs_root: packs_root.ok_or(ExecutorRuntimeError::Unavailable)?,
        linux_root: linux_root.ok_or(ExecutorRuntimeError::Unavailable)?,
        proot_path,
        loader_path,
        pack_key,
        rootfs_key,
    })
}
