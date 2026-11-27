//! Exact PRoot command and environment tests.

mod fixtures;

use std::collections::BTreeMap;
use std::fs;

use tempfile::tempdir;
use tether_linux::{NetworkPolicy, ProotConfig, SessionKind};

fn command_args(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

fn environment(command: &std::process::Command) -> BTreeMap<String, String> {
    command
        .get_envs()
        .filter_map(|(name, value)| {
            value.map(|value| {
                (
                    name.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
        })
        .collect()
}

fn workspace_arguments(linux: &std::path::Path, workspace: &std::path::Path) -> Vec<String> {
    vec![
        "-r".into(),
        linux.join("rootfs/alpine").to_string_lossy().into_owned(),
        "-0".into(),
        "--link2symlink".into(),
        "--kill-on-exit".into(),
        "-b".into(),
        "/dev".into(),
        "-b".into(),
        "/proc".into(),
        "-b".into(),
        format!("{}:/tmp", linux.join("tmp").display()),
        "-b".into(),
        format!(
            "{}:/etc/resolv.conf",
            linux.join("etc/resolv.empty").display()
        ),
        "-b".into(),
        format!("{}:/workspace", workspace.display()),
        "-w".into(),
        "/workspace".into(),
        "printf".into(),
        "hi".into(),
    ]
}

fn network_arguments(linux: &std::path::Path) -> Vec<String> {
    vec![
        "-r".into(),
        linux.join("rootfs/alpine").to_string_lossy().into_owned(),
        "-0".into(),
        "--link2symlink".into(),
        "--kill-on-exit".into(),
        "-b".into(),
        "/dev".into(),
        "-b".into(),
        "/proc".into(),
        "-b".into(),
        format!("{}:/tmp", linux.join("tmp").display()),
        "-b".into(),
        format!(
            "{}:/etc/resolv.conf",
            linux.join("etc/resolv.conf").display()
        ),
        "-w".into(),
        "/".into(),
        "true".into(),
    ]
}

fn off_environment(linux: &std::path::Path) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("ALL_PROXY".into(), "http://127.0.0.1:9".into()),
        ("HOME".into(), "/root".into()),
        ("HTTP_PROXY".into(), "http://127.0.0.1:9".into()),
        ("HTTPS_PROXY".into(), "http://127.0.0.1:9".into()),
        ("LANG".into(), "C.UTF-8".into()),
        ("NO_PROXY".into(), String::new()),
        (
            "PATH".into(),
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        ),
        ("PROJECT_FLAG".into(), "yes".into()),
        (
            "PROOT_LOADER".into(),
            linux.join("loader").to_string_lossy().into(),
        ),
        (
            "PROOT_TMP_DIR".into(),
            linux.join("tmp").to_string_lossy().into(),
        ),
        ("TERM".into(), "dumb".into()),
        ("TMPDIR".into(), "/tmp".into()),
        ("http_proxy".into(), "http://127.0.0.1:9".into()),
        ("https_proxy".into(), "http://127.0.0.1:9".into()),
    ])
}

fn on_environment(linux: &std::path::Path) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("HOME".into(), "/root".into()),
        ("LANG".into(), "C.UTF-8".into()),
        (
            "PATH".into(),
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        ),
        (
            "PROOT_TMP_DIR".into(),
            linux.join("tmp").to_string_lossy().into(),
        ),
        (
            "PROOT_LOADER".into(),
            linux.join("loader").to_string_lossy().into(),
        ),
        ("TERM".into(), "dumb".into()),
        ("TMPDIR".into(), "/tmp".into()),
    ])
}

#[test]
fn workspace_run_has_exact_arguments_and_environment() {
    let folder = tempdir().unwrap();
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let linux = folder.path().join("linux");
    let config = fixtures::config(
        &linux,
        Some(workspace.clone()),
        fixtures::fake_proot(folder.path(), &workspace),
    );

    let command = config
        .command(
            &["printf".into(), "hi".into()],
            &[("PROJECT_FLAG".into(), "yes".into())],
            "/workspace",
            SessionKind::Command,
        )
        .unwrap();

    assert_eq!(
        command_args(&command),
        workspace_arguments(&linux, &workspace)
    );
    assert_eq!(environment(&command), off_environment(&linux));
    assert!(!environment(&command).contains_key("HOST_SECRET"));
}

#[test]
fn no_workspace_network_run_has_exact_arguments_and_environment() {
    let folder = tempdir().unwrap();
    let linux = folder.path().join("linux");
    let config = fixtures::config(
        &linux,
        None,
        fixtures::fake_proot(folder.path(), folder.path()),
    );
    let mut config = config;
    config.network = NetworkPolicy::On;

    let command = config
        .command(&["true".into()], &[], "/", SessionKind::Command)
        .unwrap();

    assert_eq!(command_args(&command), network_arguments(&linux));
    assert_eq!(environment(&command), on_environment(&linux));
}

#[test]
fn shell_environment_uses_color_terminal() {
    let folder = tempdir().unwrap();
    let config = fixtures::config(
        &folder.path().join("linux"),
        None,
        fixtures::fake_proot(folder.path(), folder.path()),
    );

    let command = config
        .command(&["sh".into()], &[], "/", SessionKind::Shell)
        .unwrap();

    assert_eq!(environment(&command).get("TERM").unwrap(), "xterm-256color");
}

#[test]
fn command_builder_rejects_each_unsafe_host_path() {
    let folder = tempdir().unwrap();
    let workspace = folder.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let base = fixtures::config(
        &folder.path().join("linux"),
        Some(workspace.clone()),
        fixtures::fake_proot(folder.path(), &workspace),
    );

    let mut relative_proot = base.clone();
    relative_proot.proot_path = "relative/proot".into();
    assert!(
        relative_proot
            .command(&["true".into()], &[], "/", SessionKind::Command)
            .is_err()
    );

    let mut relative_linux = base.clone();
    relative_linux.linux_root = "relative/linux".into();
    assert!(
        relative_linux
            .command(&["true".into()], &[], "/", SessionKind::Command)
            .is_err()
    );

    let mut relative_loader = base.clone();
    relative_loader.loader_path = "relative/loader".into();
    assert!(
        relative_loader
            .command(&["true".into()], &[], "/", SessionKind::Command)
            .is_err()
    );

    let mut colon_root = base.clone();
    colon_root.linux_root = folder.path().join("linux:bad");
    assert!(
        colon_root
            .command(&["true".into()], &[], "/", SessionKind::Command)
            .is_err()
    );

    let mut comma_workspace = base.clone();
    let comma = folder.path().join("work,space");
    fs::create_dir(&comma).unwrap();
    comma_workspace.workspace = Some(comma);
    assert!(
        comma_workspace
            .command(&["true".into()], &[], "/", SessionKind::Command)
            .is_err()
    );
}

#[test]
fn command_builder_rejects_bad_environment_names() {
    let folder = tempdir().unwrap();
    let config = fixtures::config(
        &folder.path().join("linux"),
        None,
        fixtures::fake_proot(folder.path(), folder.path()),
    );

    for name in ["", "1BAD", "BAD-NAME", "BAD.NAME"] {
        assert!(
            config
                .command(
                    &["true".into()],
                    &[(name.into(), "x".into())],
                    "/",
                    SessionKind::Command
                )
                .is_err()
        );
    }
}

#[test]
fn command_builder_rejects_each_forbidden_environment_name() {
    let folder = tempdir().unwrap();
    let config = fixtures::config(
        &folder.path().join("linux"),
        None,
        fixtures::fake_proot(folder.path(), folder.path()),
    );

    for name in ["PATH", "HOME", "PROOT_TMP_DIR", "LD_PRELOAD"] {
        assert!(
            config
                .command(
                    &["true".into()],
                    &[(name.into(), "x".into())],
                    "/",
                    SessionKind::Command
                )
                .is_err()
        );
    }
}

#[test]
fn command_builder_rejects_relative_parent_and_symlinked_workspace() {
    let folder = tempdir().unwrap();
    let real = folder.path().join("real");
    let link = folder.path().join("link");
    fs::create_dir(&real).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let config = ProotConfig {
        proot_path: folder.path().join("proot"),
        loader_path: folder.path().join("loader"),
        linux_root: folder.path().join("linux"),
        workspace: Some(link),
        network: NetworkPolicy::Off,
    };

    assert!(
        config
            .command(&["true".into()], &[], "relative", SessionKind::Command)
            .is_err()
    );
    assert!(
        config
            .command(
                &["true".into()],
                &[],
                "/workspace/../tmp",
                SessionKind::Command
            )
            .is_err()
    );
    assert!(
        config
            .command(&["true".into()], &[], "/workspace", SessionKind::Command)
            .is_err()
    );
}

#[test]
fn command_builder_rejects_empty_guest_command() {
    let folder = tempdir().unwrap();
    let config = fixtures::config(
        &folder.path().join("linux"),
        None,
        fixtures::fake_proot(folder.path(), folder.path()),
    );

    assert!(config.command(&[], &[], "/", SessionKind::Command).is_err());
}
