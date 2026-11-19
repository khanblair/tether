use std::time::Duration;

use super::test_support::*;
use super::*;

#[test]
fn redirect_hosts_require_the_github_suffix_dot() {
    for value in [
        "https://evilgithub.com/x",
        "https://notgithub.com/x",
        "https://xgithubusercontent.com/x",
    ] {
        assert!(!is_allowed_redirect(&Url::parse(value).unwrap()));
    }
    for value in [
        "https://sub.github.com/x",
        "https://objects.githubusercontent.com/x",
    ] {
        assert!(is_allowed_redirect(&Url::parse(value).unwrap()));
    }
}

#[tokio::test]
async fn client_follows_at_most_five_redirects() {
    let server = start_server(redirect_routes(5, "/r0"), 6).await;
    let response = client(&local_config(&server.base_url))
        .unwrap()
        .get(format!("{}/r0", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.text().await.unwrap(), "asset");

    let endpoint = "/repos/khanblair/tether/releases/latest";
    let server = start_server(redirect_routes(6, endpoint), 7).await;
    assert_eq!(
        check(&local_config(&server.base_url), "0.0.2").await,
        Err(UpdateError::Network)
    );
}

fn redirect_routes(count: usize, first_path: &str) -> Vec<Route> {
    let mut routes = Vec::new();
    for index in 0..count {
        let path = if index == 0 {
            first_path.to_owned()
        } else {
            format!("/r{index}")
        };
        let next = if index + 1 == count {
            "/asset".to_owned()
        } else {
            format!("/r{}", index + 1)
        };
        routes.push(redirect(&path, &format!("http://test-server{next}")));
    }
    routes.push(route("/asset", reply(200, "asset")));
    routes
}

#[tokio::test]
async fn production_client_rejects_plain_http_before_connecting() {
    let error = client(&UpdateConfig::github())
        .unwrap()
        .get("http://127.0.0.1:1/")
        .send()
        .await
        .unwrap_err();
    assert!(error.is_builder() || error.to_string().contains("https"));
}

#[tokio::test]
async fn check_timeout_is_twenty_seconds_and_can_be_shortened_for_tests() {
    assert_eq!(CHECK_TIMEOUT, Duration::from_secs(20));
    let server = start_server(vec![hanging("/repos/khanblair/tether/releases/latest")], 1).await;
    assert_eq!(
        check_with_timeout(
            &local_config(&server.base_url),
            "0.0.2",
            Duration::from_millis(50)
        )
        .await,
        Err(UpdateError::Network)
    );
}
