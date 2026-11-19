use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

use super::test_support::*;
use super::*;

#[tokio::test]
async fn versions_compare_numerically_and_require_three_parts() {
    assert!(version::parse_tag("v0.0.10").unwrap() > version::parse_tag("v0.0.9").unwrap());
    assert!(version::parse_tag("v1.0.0").unwrap() > version::parse_tag("v0.99.99").unwrap());
    assert_eq!(
        version::parse_tag("v1.2.3").unwrap(),
        version::parse_current("1.2.3").unwrap()
    );
    for value in ["v1.2", "v1.2.3-rc1", "1.2.3"] {
        assert_eq!(version::parse_tag(value), Err(UpdateError::Invalid));
    }
    assert_eq!(version::parse_current("v1.2.3"), Err(UpdateError::Invalid));
}

#[test]
fn redirect_host_check_accepts_only_github_https_hosts() {
    for value in [
        "https://github.com/x",
        "https://api.github.com/x",
        "https://objects.githubusercontent.com/x",
        "https://release-assets.githubusercontent.com/x",
    ] {
        assert!(is_allowed_redirect(&Url::parse(value).unwrap()));
    }
    for value in [
        "https://evilgithubusercontent.com/x",
        "https://github.com.evil.example/x",
        "https://githubusercontent.com.evil.test/x",
        "http://github.com/x",
        "https://127.0.0.1/x",
        "file:///x",
    ] {
        assert!(!is_allowed_redirect(
            &Url::parse(value).unwrap_or_else(|_| Url::parse("https:///").unwrap())
        ));
    }
}

#[test]
fn sums_parser_requires_one_exact_matching_line() {
    let digest = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    assert_eq!(
        sums::digest_for(format!("{digest}  app.apk\n").as_bytes(), "app.apk").unwrap()[0],
        0x01
    );
    assert_eq!(
        sums::digest_for(format!("{digest}  *app.apk\n").as_bytes(), "app.apk").unwrap()[0],
        0x01
    );
    assert_eq!(
        sums::digest_for(format!("{digest}  APP.APK\n").as_bytes(), "APP.APK").unwrap()[0],
        0x01
    );
    for body in [
        format!("{digest}  app.apk.old\n"),
        format!("{digest}  old-app.apk\n"),
        format!("{digest}  app.apk\n{digest}  *app.apk\n"),
        "abcd  app.apk\n".to_owned(),
    ] {
        assert_eq!(
            sums::digest_for(body.as_bytes(), "app.apk"),
            Err(UpdateError::Invalid)
        );
    }
}

#[tokio::test]
async fn check_maps_release_and_request_headers() {
    let response = release_json(
        "v0.0.3",
        Some("notes"),
        (false, false),
        ("http://127.0.0.1:9/apk", "http://127.0.0.1:9/sums"),
        123,
    );
    let server = start_server(
        vec![route(
            "/repos/khanblair/tether/releases/latest",
            reply(200, &response),
        )],
        1,
    )
    .await;
    let config = local_config(&server.base_url);
    let result = check(&config, "0.0.2").await.unwrap();
    assert_eq!(
        result,
        UpdateCheck::Available(Release {
            version: "0.0.3".into(),
            tag: "v0.0.3".into(),
            notes: "notes".into(),
            published_at: "2026-11-19T00:00:00Z".into(),
            apk: Asset {
                name: "tether-full-v0.0.3.apk".into(),
                url: "http://127.0.0.1:9/apk".into(),
                size: 123
            },
            sums: Asset {
                name: "SHA256SUMS".into(),
                url: "http://127.0.0.1:9/sums".into(),
                size: 64
            },
        })
    );
    let request = server.requests.lock().unwrap().first().cloned().unwrap();
    assert_eq!(
        request.header("accept"),
        Some("application/vnd.github+json")
    );
    assert_eq!(request.header("x-github-api-version"), Some("2022-11-28"));
    assert_eq!(request.header("user-agent"), Some("tether-updater"));
    assert_eq!(request.header("authorization"), None);
}

#[tokio::test]
async fn check_handles_notes_body_status_and_release_flags() {
    check_truncates_multibyte_notes().await;
    check_maps_statuses().await;
    check_rejects_draft_and_prerelease().await;
}

async fn check_truncates_multibyte_notes() {
    let notes = format!("{}é", "a".repeat(7999));
    let body = release_json(
        "v0.0.3",
        Some(&notes),
        (false, false),
        ("http://127.0.0.1:9/apk", "http://127.0.0.1:9/sums"),
        1,
    );
    let server = start_server(
        vec![route(
            "/repos/khanblair/tether/releases/latest",
            reply(200, &body),
        )],
        1,
    )
    .await;
    let result = check(&local_config(&server.base_url), "0.0.2")
        .await
        .unwrap();
    let UpdateCheck::Available(release) = result else {
        panic!("expected available")
    };
    assert_eq!(release.notes.chars().count(), 8000);
}

async fn check_maps_statuses() {
    for (status, expected) in [
        (404, UpdateError::Unavailable),
        (403, UpdateError::Network),
        (500, UpdateError::Network),
    ] {
        let server = start_server(
            vec![route(
                "/repos/khanblair/tether/releases/latest",
                reply(status, ""),
            )],
            1,
        )
        .await;
        assert_eq!(
            check(&local_config(&server.base_url), "0.0.2").await,
            Err(expected)
        );
    }
}

async fn check_rejects_draft_and_prerelease() {
    for (draft, prerelease) in [(true, false), (false, true)] {
        let body = release_json(
            "v0.0.3",
            None,
            (draft, prerelease),
            ("http://127.0.0.1:9/apk", "http://127.0.0.1:9/sums"),
            1,
        );
        let server = start_server(
            vec![route(
                "/repos/khanblair/tether/releases/latest",
                reply(200, &body),
            )],
            1,
        )
        .await;
        assert_eq!(
            check(&local_config(&server.base_url), "0.0.2").await,
            Err(UpdateError::Unavailable)
        );
    }
}

#[tokio::test]
async fn check_returns_up_to_date_for_equal_or_older_versions() {
    let body = release_json("v0.0.9", Some("notes"), (false, false), ("apk", "sums"), 1);
    for current in ["0.0.9", "0.0.10"] {
        let server = start_server(
            vec![route(
                "/repos/khanblair/tether/releases/latest",
                reply(200, &body),
            )],
            1,
        )
        .await;
        let result = check(&local_config(&server.base_url), current)
            .await
            .unwrap();
        assert_eq!(
            result,
            UpdateCheck::UpToDate {
                current: current.into(),
                latest: "0.0.9".into()
            }
        );
    }
}

#[tokio::test]
async fn download_verifies_bytes_reports_progress_and_removes_stale_versions() {
    let bytes = b"verified apk".to_vec();
    let digest = digest_line(&bytes, "tether-full-v0.0.3.apk");
    let server = start_server(
        vec![
            route("/sums", reply(200, &digest)),
            route(
                "/apk",
                reply_with_length(200, &bytes, Some(bytes.len() as u64)),
            ),
        ],
        2,
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    tokio::fs::write(dir.path().join("tether-full-v0.0.2.apk"), b"old")
        .await
        .unwrap();
    tokio::fs::write(dir.path().join("unrelated.txt"), b"keep")
        .await
        .unwrap();
    let release = test_release(&server.base_url, bytes.len() as u64);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let calls_copy = Arc::clone(&calls);
    let path = download(
        &local_config(&server.base_url),
        &release,
        dir.path(),
        &move |downloaded, total| calls_copy.lock().unwrap().push((downloaded, total)),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(&path).await.unwrap(), bytes);
    assert!(!dir.path().join("tether-full-v0.0.2.apk").exists());
    assert!(dir.path().join("unrelated.txt").exists());
    let calls = calls.lock().unwrap();
    assert_eq!(calls.first(), Some(&(0, bytes.len() as u64)));
    assert_eq!(
        calls.last(),
        Some(&(bytes.len() as u64, bytes.len() as u64))
    );
}

#[tokio::test]
async fn download_rejects_bad_digest_content_length_and_large_apk() {
    download_rejects_bad_digest().await;
    download_rejects_content_length().await;
    download_rejects_large_apk().await;
}

async fn download_rejects_bad_digest() {
    let server = start_server(
        vec![
            route(
                "/sums",
                reply(200, &digest_line(b"wrong", "tether-full-v0.0.3.apk")),
            ),
            route("/apk", reply(200, "bytes")),
        ],
        2,
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let release = test_release(&server.base_url, 5);
    assert_eq!(
        download(
            &local_config(&server.base_url),
            &release,
            dir.path(),
            &|_, _| {},
            CancellationToken::new()
        )
        .await,
        Err(UpdateError::Checksum)
    );
    assert!(!dir.path().join("tether-full-v0.0.3.apk.part").exists());
}

async fn download_rejects_content_length() {
    let server = start_server(
        vec![
            route(
                "/sums",
                reply(200, &digest_line(b"bytes", "tether-full-v0.0.3.apk")),
            ),
            route("/apk", reply_with_length(200, "bytes", Some(99))),
        ],
        2,
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        download(
            &local_config(&server.base_url),
            &test_release(&server.base_url, 5),
            dir.path(),
            &|_, _| {},
            CancellationToken::new()
        )
        .await,
        Err(UpdateError::Invalid)
    );
}

async fn download_rejects_large_apk() {
    let server = start_server(Vec::new(), 0).await;
    let base_url = server.base_url.clone();
    let release = Release {
        apk: Asset {
            size: MAX_APK_BYTES + 1,
            ..test_release(&base_url, 5).apk
        },
        ..test_release(&base_url, 5)
    };
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        download(
            &local_config(&server.base_url),
            &release,
            dir.path(),
            &|_, _| {},
            CancellationToken::new()
        )
        .await,
        Err(UpdateError::TooLarge)
    );
    assert!(server.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn download_cancellation_removes_part_and_cached_file_skips_apk_request() {
    download_cancellation_removes_part().await;
}

async fn download_cancellation_removes_part() {
    let bytes = b"cancel me".to_vec();
    let digest = digest_line(&bytes, "tether-full-v0.0.3.apk");
    let server = start_server(
        vec![
            route("/sums", reply(200, &digest)),
            route_stream("/apk", bytes.clone(), 2),
        ],
        2,
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let cancel_copy = cancel.clone();
    let result = download(
        &local_config(&server.base_url),
        &test_release(&server.base_url, bytes.len() as u64),
        dir.path(),
        &move |downloaded, _| cancel_after_first(downloaded, &cancel_copy),
        cancel,
    )
    .await;
    assert_eq!(result, Err(UpdateError::Cancelled));
    assert!(!dir.path().join("tether-full-v0.0.3.apk.part").exists());
}

fn cancel_after_first(downloaded: u64, token: &CancellationToken) {
    if downloaded > 0 {
        token.cancel();
    }
}
