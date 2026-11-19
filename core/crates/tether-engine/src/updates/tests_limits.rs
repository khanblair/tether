use super::test_support::*;
use super::*;
use tokio_util::sync::CancellationToken;

#[test]
fn the_apk_size_limit_is_256_mebibytes() {
    assert_eq!(MAX_APK_BYTES, 268_435_456);
}

#[tokio::test]
async fn apk_size_at_the_limit_is_requested_but_one_byte_over_is_rejected() {
    let server = start_server(vec![route("/sums", reply(404, ""))], 1).await;
    let release = Release {
        apk: Asset {
            size: MAX_APK_BYTES,
            ..test_release(&server.base_url, 1).apk
        },
        ..test_release(&server.base_url, 1)
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
        Err(UpdateError::Network)
    );
    assert_eq!(server.requests.lock().unwrap().len(), 1);

    let server = start_server(Vec::new(), 0).await;
    let release = Release {
        apk: Asset {
            size: MAX_APK_BYTES + 1,
            ..test_release(&server.base_url, 1).apk
        },
        ..test_release(&server.base_url, 1)
    };
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
async fn apk_stream_length_is_exact_and_failed_parts_are_removed() {
    let longer = download_stream_result(b"123456", 5).await;
    assert_eq!(longer.0, Err(UpdateError::TooLarge));
    assert!(!longer.1);

    let shorter = download_stream_result(b"1234", 5).await;
    assert_eq!(shorter.0, Err(UpdateError::Invalid));
    assert!(!shorter.1);
}

async fn download_stream_result(
    body: &[u8],
    size: u64,
) -> (Result<std::path::PathBuf, UpdateError>, bool) {
    let server = start_server(
        vec![
            route(
                "/sums",
                reply(200, &digest_line(body, "tether-full-v0.0.3.apk")),
            ),
            route_stream("/apk", body.to_vec(), body.len().max(1)),
        ],
        2,
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let result = download(
        &local_config(&server.base_url),
        &test_release(&server.base_url, size),
        dir.path(),
        &|_, _| {},
        CancellationToken::new(),
    )
    .await;
    let part_exists = dir.path().join("tether-full-v0.0.3.apk.part").exists();
    (result, part_exists)
}

#[tokio::test]
async fn download_validates_release_shape_before_requesting_anything() {
    for case in 0..4 {
        let server = start_server(Vec::new(), 0).await;
        let mut release = test_release(&server.base_url, 1);
        set_invalid_shape(&mut release, case);
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
            Err(UpdateError::Invalid)
        );
        assert!(server.requests.lock().unwrap().is_empty());
    }
}

fn set_invalid_shape(release: &mut Release, case: usize) {
    match case {
        0 => release.sums.name = "checksums.txt".into(),
        1 => release.apk.url = "http://evil.example/a.apk".into(),
        2 => {
            release.version = "1.2".into();
            release.apk.name = "tether-full-v1.2.apk".into();
        }
        3 => {
            release.version = "1.2.3.4".into();
            release.apk.name = "tether-full-v1.2.3.4.apk".into();
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn checksums_file_limit_rejects_70_kib_and_accepts_just_under_64_kib() {
    let line = digest_line(b"apk", "tether-full-v0.0.3.apk");
    let oversized = format!("{}{}", line, "x".repeat(70 * 1024));
    let server = start_server(vec![route("/sums", reply(200, &oversized))], 1).await;
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        download(
            &local_config(&server.base_url),
            &test_release(&server.base_url, 3),
            dir.path(),
            &|_, _| {},
            CancellationToken::new()
        )
        .await,
        Err(UpdateError::Invalid)
    );

    let under = format!("{}{}", line, "x".repeat(64 * 1024 - line.len() - 1));
    let server = start_server(
        vec![
            route("/sums", reply(200, &under)),
            route("/apk", reply_with_length(200, b"apk", Some(3))),
        ],
        2,
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let path = download(
        &local_config(&server.base_url),
        &test_release(&server.base_url, 3),
        dir.path(),
        &|_, _| {},
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(tokio::fs::read(path).await.unwrap(), b"apk");
}

#[tokio::test]
async fn stale_cleanup_keeps_invalid_names_and_removes_old_apk_names() {
    let bytes = b"apk";
    let server = start_server(
        vec![
            route(
                "/sums",
                reply(200, &digest_line(bytes, "tether-full-v0.0.3.apk")),
            ),
            route("/apk", reply_with_length(200, bytes, Some(3))),
        ],
        2,
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "tether-full-v1.2.apk",
        "tether-full-v1.2.3.4.apk",
        "tether-full-v0.0.1.apk",
        "tether-full-v0.0.1.apk.part",
    ] {
        tokio::fs::write(dir.path().join(name), b"old")
            .await
            .unwrap();
    }
    download(
        &local_config(&server.base_url),
        &test_release(&server.base_url, 3),
        dir.path(),
        &|_, _| {},
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(dir.path().join("tether-full-v1.2.apk").exists());
    assert!(dir.path().join("tether-full-v1.2.3.4.apk").exists());
    assert!(!dir.path().join("tether-full-v0.0.1.apk").exists());
    assert!(!dir.path().join("tether-full-v0.0.1.apk.part").exists());
}
