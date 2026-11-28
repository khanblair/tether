//! Guest `/etc/hosts` tests: servers started in Linux mode must reach IPv4 loopback.

mod fixtures;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use tempfile::tempdir;
use tether_linux::SessionKind;

const IPV4_ONLY: &str = "127.0.0.1\tlocalhost localhost.localdomain\n";

#[test]
fn the_command_builder_writes_an_ipv4_only_hosts_file() {
    let folder = tempdir().unwrap();
    let linux = folder.path().join("linux");
    let config = fixtures::config(
        &linux,
        None,
        fixtures::fake_proot(folder.path(), folder.path()),
    );

    config
        .command(&["true".into()], &[], "/", SessionKind::Command)
        .unwrap();

    let hosts = fs::read_to_string(linux.join("etc/hosts")).unwrap();
    assert_eq!(hosts, IPV4_ONLY);
    assert!(!hosts.contains("::1"));
}

#[test]
fn an_old_hosts_file_with_ipv6_localhost_is_replaced() {
    let folder = tempdir().unwrap();
    let linux = folder.path().join("linux");
    let config = fixtures::config(
        &linux,
        None,
        fixtures::fake_proot(folder.path(), folder.path()),
    );
    fs::write(linux.join("etc/hosts"), b"::1 localhost\n").unwrap();

    config
        .command(&["true".into()], &[], "/", SessionKind::Command)
        .unwrap();

    assert_eq!(
        fs::read_to_string(linux.join("etc/hosts")).unwrap(),
        IPV4_ONLY
    );
}

#[test]
fn a_hosts_symlink_is_rejected() {
    let folder = tempdir().unwrap();
    let linux = folder.path().join("linux");
    let config = fixtures::config(
        &linux,
        None,
        fixtures::fake_proot(folder.path(), folder.path()),
    );
    let target = folder.path().join("target");
    fs::write(&target, b"keep me\n").unwrap();
    std::os::unix::fs::symlink(&target, linux.join("etc/hosts")).unwrap();

    let result = config.command(&["true".into()], &[], "/", SessionKind::Command);

    assert!(result.is_err());
    assert_eq!(fs::read(&target).unwrap(), b"keep me\n");
}

#[test]
fn a_hosts_file_the_guest_made_unreadable_is_replaced() {
    let folder = tempdir().unwrap();
    let linux = folder.path().join("linux");
    let config = fixtures::config(
        &linux,
        None,
        fixtures::fake_proot(folder.path(), folder.path()),
    );
    let hosts = linux.join("etc/hosts");
    fs::write(&hosts, b"::1 localhost\n").unwrap();
    fs::set_permissions(&hosts, fs::Permissions::from_mode(0o000)).unwrap();

    config
        .command(&["true".into()], &[], "/", SessionKind::Command)
        .unwrap();

    assert_eq!(fs::read_to_string(&hosts).unwrap(), IPV4_ONLY);
}

#[test]
fn no_temp_file_is_left_behind() {
    let folder = tempdir().unwrap();
    let linux = folder.path().join("linux");
    let config = fixtures::config(
        &linux,
        None,
        fixtures::fake_proot(folder.path(), folder.path()),
    );
    fs::write(linux.join("etc/hosts"), b"::1 localhost\n").unwrap();

    config
        .command(&["true".into()], &[], "/", SessionKind::Command)
        .unwrap();

    let leftovers: Vec<_> = fs::read_dir(linux.join("etc"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().starts_with("hosts.tmp"))
        .collect();
    assert!(leftovers.is_empty(), "leftover temp files: {leftovers:?}");
}

#[test]
fn a_hosts_directory_is_rejected() {
    let folder = tempdir().unwrap();
    let linux = folder.path().join("linux");
    let config = fixtures::config(
        &linux,
        None,
        fixtures::fake_proot(folder.path(), folder.path()),
    );
    fs::create_dir(linux.join("etc/hosts")).unwrap();

    let error = config
        .command(&["true".into()], &[], "/", SessionKind::Command)
        .expect_err("a hosts directory must be rejected")
        .to_string();

    assert!(error.contains("not a file"), "unexpected error: {error}");
}
