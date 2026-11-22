use std::path::{Component, Path, PathBuf};
use std::process::Command;

use crate::LinuxError;

/// Whether guest processes may use the project's network configuration.
///
/// This is a best-effort block. A program that connects to a raw IP address
/// can still bypass the resolver and proxy settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkPolicy {
    /// Disable normal resolver and proxy based network access.
    #[default]
    Off,
    /// Bind the configured resolver and leave proxy variables unset.
    On,
}

/// Selects the guest terminal environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    /// A non-interactive command with a dumb terminal.
    Command,
    /// An interactive shell with color terminal support.
    Shell,
}

/// Host paths and guest arguments needed to launch PRoot.
#[derive(Debug, Clone)]
pub struct ProotConfig {
    /// Absolute path to the PRoot executable.
    pub proot_path: PathBuf,
    /// Optional absolute path to the PRoot loader.
    pub loader_path: Option<PathBuf>,
    /// The private Linux storage directory.
    pub linux_root: PathBuf,
    /// Optional real project directory mounted at `/workspace`.
    pub workspace: Option<PathBuf>,
    /// Per-project network policy.
    pub network: NetworkPolicy,
}

impl ProotConfig {
    /// Builds the exact host command and scrubbed environment for one guest run.
    ///
    /// # Errors
    /// Rejects unsafe host paths, guest working directories, and environment names.
    pub fn command(
        &self,
        guest_argv: &[String],
        extra_env: &[(String, String)],
        cwd: &str,
        kind: SessionKind,
    ) -> Result<Command, LinuxError> {
        let plan = self.plan(guest_argv, extra_env, cwd, kind)?;
        let mut command = Command::new(&self.proot_path);
        command.args(plan.arguments);
        command.env_clear();
        for (name, value) in plan.environment {
            command.env(name, value);
        }
        Ok(command)
    }

    pub(crate) fn plan(
        &self,
        guest_argv: &[String],
        extra_env: &[(String, String)],
        cwd: &str,
        kind: SessionKind,
    ) -> Result<CommandPlan, LinuxError> {
        validate_host_path(&self.proot_path, "PRoot")?;
        validate_host_path(&self.linux_root, "Linux storage")?;
        if guest_argv.is_empty() {
            return Err(LinuxError::Path("guest command is empty".into()));
        }
        validate_cwd(cwd)?;
        let rootfs = self.linux_root.join("rootfs/alpine");
        let tmp = self.linux_root.join("tmp");
        let empty_resolv = self.linux_root.join("etc/resolv.empty");
        let network_resolv = self.linux_root.join("etc/resolv.conf");
        for path in [&rootfs, &tmp, &empty_resolv, &network_resolv] {
            validate_host_path(path, "Linux bind")?;
        }
        let mut arguments = vec![
            "-r".into(),
            rootfs.to_string_lossy().into_owned(),
            "-0".into(),
            "--link2symlink".into(),
            "--kill-on-exit".into(),
            "-b".into(),
            "/dev".into(),
            "-b".into(),
            "/proc".into(),
            "-b".into(),
            format!("{}:/tmp", tmp.display()),
        ];
        let resolv = match self.network {
            NetworkPolicy::Off => empty_resolv,
            NetworkPolicy::On => network_resolv,
        };
        arguments.extend([
            "-b".into(),
            format!("{}:/etc/resolv.conf", resolv.display()),
        ]);
        if let Some(workspace) = &self.workspace {
            validate_workspace(workspace)?;
            arguments.extend(["-b".into(), format!("{}:/workspace", workspace.display())]);
        }
        arguments.extend(["-w".into(), cwd.into()]);
        arguments.extend(guest_argv.iter().cloned());
        let environment = guest_environment(self, extra_env, kind)?;
        Ok(CommandPlan {
            arguments,
            environment,
        })
    }
}

#[derive(Debug)]
pub(crate) struct CommandPlan {
    pub(crate) arguments: Vec<String>,
    pub(crate) environment: Vec<(String, String)>,
}

fn guest_environment(
    config: &ProotConfig,
    extra_env: &[(String, String)],
    kind: SessionKind,
) -> Result<Vec<(String, String)>, LinuxError> {
    let mut environment = vec![(
        "PROOT_TMP_DIR".into(),
        config.linux_root.join("tmp").to_string_lossy().into_owned(),
    )];
    if let Some(loader) = &config.loader_path {
        validate_host_path(loader, "PRoot loader")?;
        environment.push(("PROOT_LOADER".into(), loader.to_string_lossy().into_owned()));
    }
    let term = match kind {
        SessionKind::Command => "dumb",
        SessionKind::Shell => "xterm-256color",
    };
    environment.extend([
        ("HOME".into(), "/root".into()),
        (
            "PATH".into(),
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        ),
        ("LANG".into(), "C.UTF-8".into()),
        ("TMPDIR".into(), "/tmp".into()),
        ("TERM".into(), term.into()),
    ]);
    if config.network == NetworkPolicy::Off {
        environment.extend([
            ("http_proxy".into(), "http://127.0.0.1:9".into()),
            ("https_proxy".into(), "http://127.0.0.1:9".into()),
            ("HTTP_PROXY".into(), "http://127.0.0.1:9".into()),
            ("HTTPS_PROXY".into(), "http://127.0.0.1:9".into()),
            ("ALL_PROXY".into(), "http://127.0.0.1:9".into()),
            ("NO_PROXY".into(), String::new()),
        ]);
    }
    for (name, value) in extra_env {
        validate_env_name(name)?;
        if name == "PATH" || name == "HOME" || name.starts_with("PROOT_") || name.starts_with("LD_")
        {
            return Err(LinuxError::Path("environment variable is reserved".into()));
        }
        environment.push((name.clone(), value.clone()));
    }
    Ok(environment)
}

fn validate_env_name(name: &str) -> Result<(), LinuxError> {
    let mut bytes = name.bytes();
    let valid = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(LinuxError::Path("environment name rejected".into()))
    }
}

fn validate_cwd(cwd: &str) -> Result<(), LinuxError> {
    let path = Path::new(cwd);
    if !path.is_absolute() || path.components().any(|part| part == Component::ParentDir) {
        return Err(LinuxError::Path("guest working directory rejected".into()));
    }
    Ok(())
}

fn validate_host_path(path: &Path, label: &str) -> Result<(), LinuxError> {
    let text = path.to_string_lossy();
    if !path.is_absolute()
        || text
            .bytes()
            .any(|byte| matches!(byte, b':' | b',' | 0 | b'\n' | b'\r'))
    {
        return Err(LinuxError::Path(format!("{label} path rejected")));
    }
    Ok(())
}

fn validate_workspace(path: &Path) -> Result<(), LinuxError> {
    validate_host_path(path, "workspace")?;
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| LinuxError::Path("workspace is unavailable".into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LinuxError::Path(
            "workspace must be a real directory".into(),
        ));
    }
    Ok(())
}

/// Classifies common package and tool failures caused by the offline policy.
#[must_use]
pub fn classify_network_failure(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    let markers = [
        "temporary error",
        "temporary failure in name resolution",
        "enotfound",
        "eai_again",
        "econnrefused 127.0.0.1:9",
        "could not resolve host",
        "spurious network error",
        "network is unreachable",
        "failed to resolve",
        "max retries exceeded",
        "failed to establish a new connection",
    ];
    markers.iter().any(|marker| lower.contains(marker))
        || (lower.contains("unable to select packages") && lower.contains("fetch"))
}
