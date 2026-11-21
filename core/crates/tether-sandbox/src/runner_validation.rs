//! Validation for sandbox support mounts and guest settings.

use std::fs;
use std::path::Path;

use crate::{ExtraMount, RunSpec, SandboxError};

pub(crate) fn validate_spec(spec: &RunSpec) -> Result<(), SandboxError> {
    if !valid_guest_cwd(&spec.cwd) {
        return Err(SandboxError::Runtime(
            "sandbox working directory is invalid".into(),
        ));
    }
    let root = fs::canonicalize(&spec.pack_root)
        .map_err(|_| SandboxError::Runtime("pack root is unavailable".into()))?;
    for mount in &spec.extra_mounts {
        validate_mount(&root, mount)?;
    }
    for (name, value) in &spec.extra_env {
        if !valid_env_name(name) || value.chars().count() > 256 {
            return Err(SandboxError::Runtime("pack environment is invalid".into()));
        }
        if matches!(name.as_str(), "HOME" | "PATH" | "TMPDIR" | "LANG") {
            return Err(SandboxError::Runtime("pack environment is reserved".into()));
        }
    }
    Ok(())
}

fn validate_mount(root: &Path, mount: &ExtraMount) -> Result<(), SandboxError> {
    let metadata = fs::symlink_metadata(&mount.host)
        .map_err(|_| SandboxError::Runtime("support mount is unavailable".into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(SandboxError::Runtime(
            "support mount is not a folder".into(),
        ));
    }
    let host = fs::canonicalize(&mount.host)
        .map_err(|_| SandboxError::Runtime("support mount is unavailable".into()))?;
    if !host.starts_with(root) || !valid_guest_path(&mount.guest) {
        return Err(SandboxError::Runtime("support mount path rejected".into()));
    }
    Ok(())
}

fn valid_env_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_uppercase() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn valid_guest_path(guest: &str) -> bool {
    let path = Path::new(guest);
    path.is_absolute()
        && path.components().all(|component| {
            !matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
        && guest != "/workspace"
        && guest != "/tmp"
        && !guest.starts_with("/workspace/")
        && !guest.starts_with("/tmp/")
}

fn valid_guest_cwd(cwd: &str) -> bool {
    let path = Path::new(cwd);
    path.is_absolute()
        && (cwd == "/workspace" || cwd.starts_with("/workspace/"))
        && path.components().all(|component| {
            !matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
}
