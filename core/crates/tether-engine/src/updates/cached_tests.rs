use super::test_support::*;
use super::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn verified_cached_file_skips_the_second_apk_request() {
    let bytes = b"cancel me".to_vec();
    let digest = digest_line(&bytes, "tether-full-v0.0.3.apk");
    let dir = tempfile::tempdir().unwrap();
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
    let release = test_release(&server.base_url, bytes.len() as u64);
    download(
        &local_config(&server.base_url),
        &release,
        dir.path(),
        &|_, _| {},
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let server = start_server(vec![route("/sums", reply(200, &digest))], 1).await;
    let release = test_release(&server.base_url, bytes.len() as u64);
    let path = download(
        &local_config(&server.base_url),
        &release,
        dir.path(),
        &|_, _| {},
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(path.exists());
    assert_eq!(server.requests.lock().unwrap().len(), 1);
}
