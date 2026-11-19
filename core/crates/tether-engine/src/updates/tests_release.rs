use super::test_support::*;
use super::*;

#[tokio::test]
async fn release_notes_are_exactly_limited_to_8000_characters() {
    let long = "é".repeat(9000);
    assert_eq!(check_notes(&long).await, "é".repeat(8000));

    let exact = "é".repeat(8000);
    assert_eq!(check_notes(&exact).await, exact);
}

async fn check_notes(notes: &str) -> String {
    let body = release_json(
        "v0.0.3",
        Some(notes),
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
        panic!("expected available release")
    };
    release.notes
}

#[tokio::test]
async fn release_rejects_each_bad_asset_url_independently() {
    for urls in [
        ("http://evil.example/a.apk", "http://127.0.0.1:9/sums"),
        ("http://127.0.0.1:9/apk", "http://evil.example/SHA256SUMS"),
    ] {
        let body = release_json("v0.0.3", None, (false, false), urls, 1);
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
            Err(UpdateError::Invalid)
        );
    }
}

#[tokio::test]
async fn release_response_is_capped_at_one_mebibyte() {
    let oversized = release_json(
        "v0.0.3",
        Some(&"x".repeat(1_200_000)),
        (false, false),
        ("http://127.0.0.1:9/apk", "http://127.0.0.1:9/sums"),
        1,
    );
    let server = start_server(
        vec![route(
            "/repos/khanblair/tether/releases/latest",
            reply(200, &oversized),
        )],
        1,
    )
    .await;
    assert_eq!(
        check(&local_config(&server.base_url), "0.0.2").await,
        Err(UpdateError::Invalid)
    );

    let under = release_json(
        "v0.0.3",
        Some(&"x".repeat(1_000_000)),
        (false, false),
        ("http://127.0.0.1:9/apk", "http://127.0.0.1:9/sums"),
        1,
    );
    let server = start_server(
        vec![route(
            "/repos/khanblair/tether/releases/latest",
            reply(200, &under),
        )],
        1,
    )
    .await;
    let result = check(&local_config(&server.base_url), "0.0.2")
        .await
        .unwrap();
    let UpdateCheck::Available(release) = result else {
        panic!("expected available release")
    };
    assert_eq!(release.version, "0.0.3");
    assert_eq!(release.notes, "x".repeat(8000));
}
