use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

use super::test_support::*;
use super::*;

#[tokio::test]
async fn download_reports_each_rising_percent_once_and_finishes_once() {
    let bytes = vec![b'a'; 200];
    let server = start_server(
        vec![
            route(
                "/sums",
                reply(200, &digest_line(&bytes, "tether-full-v0.0.3.apk")),
            ),
            route_stream("/apk", bytes.clone(), 200),
        ],
        2,
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let calls_copy = Arc::clone(&calls);
    download(
        &local_config(&server.base_url),
        &test_release(&server.base_url, 200),
        dir.path(),
        &move |downloaded, total| calls_copy.lock().unwrap().push((downloaded, total)),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    let mut expected = vec![(0, 200)];
    expected.extend((1..100).map(|percent| (percent * 2, 200)));
    expected.push((200, 200));
    assert_eq!(*calls.lock().unwrap(), expected);
}

#[tokio::test]
async fn download_checks_cancellation_before_making_requests() {
    let server = start_server(Vec::new(), 0).await;
    let dir = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        download(
            &local_config(&server.base_url),
            &test_release(&server.base_url, 1),
            dir.path(),
            &|_, _| {},
            cancel
        )
        .await,
        Err(UpdateError::Cancelled)
    );
    assert!(server.requests.lock().unwrap().is_empty());
}
