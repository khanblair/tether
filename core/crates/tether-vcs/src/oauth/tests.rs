use super::{OAuthError, TokenPoll, parse_poll, parse_start};

#[test]
fn device_start_parses_public_code_and_poll_interval() {
    let body = br#"{"device_code":"private-code","user_code":"ABCD-EFGH","verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#;
    let grant = parse_start(body).unwrap();
    assert_eq!(grant.user_code, "ABCD-EFGH");
    assert_eq!(grant.interval_seconds, 5);
    assert_eq!(grant.verification_uri, "https://github.com/login/device");
}

#[test]
fn device_start_rejects_untrusted_browser_url() {
    let body = br#"{"device_code":"private-code","user_code":"ABCD-EFGH","verification_uri":"https://example.invalid","expires_in":900}"#;
    assert!(matches!(
        parse_start(body),
        Err(OAuthError::InvalidResponse)
    ));
}

#[test]
fn poll_errors_are_not_mistaken_for_tokens() {
    assert!(matches!(
        parse_poll(br#"{"error":"authorization_pending"}"#).unwrap(),
        TokenPoll::Pending
    ));
    assert!(matches!(
        parse_poll(br#"{"error":"slow_down"}"#).unwrap(),
        TokenPoll::SlowDown
    ));
    assert!(matches!(
        parse_poll(br#"{"error":"expired_token"}"#),
        Err(OAuthError::Expired)
    ));
    assert!(matches!(
        parse_poll(br#"{"error":"access_denied"}"#),
        Err(OAuthError::Denied)
    ));
    assert!(matches!(
        parse_poll(br#"{"access_token":"example-token","token_type":"bearer"}"#).unwrap(),
        TokenPoll::Token(_)
    ));
}
