//! Manifest and PRoot builder integration tests.

mod fixtures;

use std::collections::BTreeMap;
use std::fs;

use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use tempfile::tempdir;
use tether_linux::{
    NetworkPolicy, ProotConfig, SessionKind, classify_network_failure, verify_rootfs_manifest,
};

#[test]
fn signed_manifest_is_verified_before_validation() {
    let key = SigningKey::from_bytes(&[7_u8; 32]);
    let bytes = serde_json::to_vec(&json!({"format":1,"alpine_version":"3.20","images":[{"arch":"aarch64","url":"https://example.invalid/alpine.tar.gz","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size_bytes":3,"max_extracted_bytes":100}],"base_packages":["busybox"]})).unwrap();
    let signature = key.sign(&bytes).to_bytes();

    let manifest =
        verify_rootfs_manifest(&bytes, &signature, &key.verifying_key().to_bytes()).unwrap();

    assert_eq!(manifest.alpine_version, "3.20");
    assert!(verify_rootfs_manifest(b"{", &signature, &key.verifying_key().to_bytes()).is_err());
}

#[test]
fn command_builder_scrubs_environment_and_orders_binds() {
    let directory = tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let proot = fixtures::fake_proot(directory.path(), &workspace);
    let mut config = fixtures::config(&directory.path().join("linux"), Some(workspace), proot);
    config.network = NetworkPolicy::Off;

    let command = config
        .command(
            &["printf".into(), "hi".into()],
            &[("PROJECT_FLAG".into(), "yes".into())],
            "/workspace",
            SessionKind::Command,
        )
        .unwrap();
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let environment: BTreeMap<String, String> = command
        .get_envs()
        .filter_map(|(key, value)| {
            value.map(|value| {
                (
                    key.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
        })
        .collect();

    assert_eq!(args[0], "-r");
    assert!(args.iter().any(|arg| arg == "--kill-on-exit"));
    assert_eq!(args.last().map(String::as_str), Some("hi"));
    assert_eq!(
        environment.get("PROJECT_FLAG").map(String::as_str),
        Some("yes")
    );
    assert_eq!(environment.get("TERM").map(String::as_str), Some("dumb"));
    assert!(!environment.contains_key("HOST_SECRET"));
    assert_eq!(
        environment.get("http_proxy").map(String::as_str),
        Some("http://127.0.0.1:9")
    );
}

#[test]
fn network_failure_classifier_uses_tool_messages() {
    assert!(classify_network_failure(
        "apk: temporary error (try again later)"
    ));
    assert!(classify_network_failure("npm ERR! code ENOTFOUND"));
    assert!(classify_network_failure(
        "error: spurious network error: Could not resolve host"
    ));
    assert!(!classify_network_failure("error: permission denied"));
    assert!(!classify_network_failure("tests failed: assertion failed"));
}

#[test]
fn classifies_apk_network_failures() {
    for output in [
        "apk: temporary error (try again later)",
        "temporary failure in name resolution",
        "unable to select packages: fetch https://dl-cdn.alpinelinux.org failed",
    ] {
        assert!(classify_network_failure(output), "{output}");
    }
}

#[test]
fn classifies_npm_network_failures() {
    for output in [
        "npm ERR! code ENOTFOUND",
        "npm ERR! max retries exceeded",
        "npm ERR! network is unreachable",
    ] {
        assert!(classify_network_failure(output), "{output}");
    }
}

#[test]
fn classifies_cargo_network_failures() {
    for output in [
        "warning: spurious network error: Could not resolve host",
        "failed to resolve: registry.example.invalid",
        "failed to establish a new connection",
    ] {
        assert!(classify_network_failure(output), "{output}");
    }
}

#[test]
fn does_not_classify_non_network_failures() {
    for output in [
        "error: permission denied",
        "checksum mismatch for package",
        "tests failed: assertion failed",
    ] {
        assert!(!classify_network_failure(output), "{output}");
    }
}

#[test]
fn command_builder_rejects_relative_cwd_and_reserved_environment() {
    let directory = tempdir().unwrap();
    let config = ProotConfig {
        proot_path: directory.path().join("proot"),
        loader_path: None,
        linux_root: directory.path().join("linux"),
        workspace: None,
        network: NetworkPolicy::On,
    };

    assert!(
        config
            .command(&["sh".into()], &[], "../tmp", SessionKind::Command)
            .is_err()
    );
    assert!(
        config
            .command(
                &["sh".into()],
                &[("PATH".into(), "bad".into())],
                "/",
                SessionKind::Command
            )
            .is_err()
    );
}
