use super::{PatError, parse_account, scopes_from_headers};
use reqwest::header::{HeaderMap, HeaderValue};

#[test]
fn classic_scopes_are_listed_and_trimmed() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-oauth-scopes",
        HeaderValue::from_static("repo, read:user, workflow"),
    );
    let info = parse_account(
        br#"{"login":"example-user"}"#,
        scopes_from_headers(&headers),
    )
    .unwrap();
    assert_eq!(info.login, "example-user");
    assert_eq!(info.scopes.unwrap(), vec!["repo", "read:user", "workflow"]);
}

#[test]
fn fine_grained_token_without_scope_header_is_not_misreported() {
    let info = parse_account(
        br#"{"login":"example-user"}"#,
        scopes_from_headers(&HeaderMap::new()),
    )
    .unwrap();
    assert!(info.scopes.is_none());
}

#[test]
fn a_response_without_login_is_rejected_before_storage() {
    assert!(matches!(
        parse_account(br#"{"message":"Bad credentials"}"#, None),
        Err(PatError::InvalidResponse)
    ));
}
