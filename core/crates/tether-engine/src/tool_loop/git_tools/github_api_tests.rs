use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tether_secrets::Secret;
use tether_vcs::{GithubClient, HttpClient, HttpClientError, HttpRequest, HttpResponse};
use tokio_util::sync::CancellationToken;

use super::github_api;
use super::{GitTools, GitToolsConfig, GithubClientFactory};
use crate::permissions::Effect;
use crate::tool_loop::{PrepareContext, ToolHandler};

struct FakeHttp {
    responses: Mutex<VecDeque<Result<HttpResponse, HttpClientError>>>,
    requests: Mutex<Vec<HttpRequest>>,
}

impl FakeHttp {
    fn new(responses: Vec<Result<HttpResponse, HttpClientError>>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
        })
    }

    fn request(&self) -> HttpRequest {
        self.requests.lock().expect("request lock")[0].clone()
    }
}

impl HttpClient for FakeHttp {
    fn send(
        &self,
        request: HttpRequest,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpClientError>> + Send + '_>> {
        self.requests.lock().expect("request lock").push(request);
        let response = self
            .responses
            .lock()
            .expect("response lock")
            .pop_front()
            .expect("fixture response");
        Box::pin(async move { response })
    }
}

struct Factory {
    http: Arc<FakeHttp>,
}

impl GithubClientFactory for Factory {
    fn client(&self) -> Result<GithubClient, String> {
        GithubClient::with_base_url(
            self.http.clone(),
            Secret::from_text("ghp_test_token_123"),
            "https://api.github.com",
        )
        .map_err(|error| error.to_string())
    }
}

struct NotSignedIn;

impl GithubClientFactory for NotSignedIn {
    fn client(&self) -> Result<GithubClient, String> {
        Err("GitHub is not signed in".to_owned())
    }
}

fn prepare(args: &str) -> Result<crate::tool_loop::PreparedCall, String> {
    let root = tempfile::tempdir().expect("temporary root");
    let guard = tether_workspace::PathGuard::new(root.path()).expect("path guard");
    let handler = empty_handler();
    handler.prepare("github_api", args, &PrepareContext { guard: &guard })
}

fn empty_handler() -> GitTools {
    GitTools::new(GitToolsConfig {
        remote_url: None,
        default_branch: None,
        identity: Arc::new(NoIdentity),
        transport: Arc::new(NoTransport),
        github: Arc::new(NotSignedIn),
        github_owner: None,
        remote_writer: Arc::new(NoRemote),
        gates: Vec::new(),
    })
}

struct NoIdentity;
impl super::GitIdentityReader for NoIdentity {
    fn identity(&self) -> Result<Option<tether_vcs::Identity>, String> {
        Ok(None)
    }
}

struct NoTransport;
impl super::TransportFactory for NoTransport {
    fn transport(&self) -> Result<Box<dyn tether_vcs::Transport>, String> {
        Err("unused".to_owned())
    }
}

struct NoRemote;
impl super::GitRemoteWriter for NoRemote {
    fn set_remote(&self, _: &str, _: &str) -> Result<(), String> {
        Ok(())
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "fixtures need the transport result shape"
)]
fn response(
    status: u16,
    headers: &[(&str, &str)],
    body: &str,
) -> Result<HttpResponse, HttpClientError> {
    Ok(HttpResponse {
        status,
        headers: headers
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
        body: body.as_bytes().to_vec(),
    })
}

#[test]
fn specs_include_github_api_with_a_valid_schema() {
    let handler = empty_handler();
    let spec = handler
        .specs()
        .into_iter()
        .find(|spec| spec.name == "github_api")
        .expect("github_api spec");
    let schema: Value = serde_json::from_str(&spec.input_schema_json).expect("schema JSON");
    assert_eq!(schema["required"], serde_json::json!(["method", "path"]));
    assert!(spec.description.contains("signed-in user"));
}

#[test]
fn validation_rejects_unsafe_github_api_arguments() {
    for args in [
        r#"{"method":"TRACE","path":"/user"}"#,
        r#"{"method":"GET","path":"https://evil.com/x"}"#,
        r#"{"method":"GET","path":"//evil.com/x"}"#,
        r#"{"method":"GET","path":"user"}"#,
        r#"{"method":"GET","path":"/a/../b"}"#,
        r#"{"method":"GET","path":"/repos/o/r/%2e%2e/%2e%2e/user/keys"}"#,
        r#"{"method":"GET","path":"/x/%2E%2E/y"}"#,
        r#"{"method":"GET","path":"/x/%5c/y"}"#,
        r#"{"method":"GET","path":"/x%0a/y"}"#,
        r#"{"method":"GET","path":"/x/%zz"}"#,
        r#"{"method":"GET","path":"/x/%c3%28"}"#,
        r#"{"method":"GET","path":"/%2f%2fevil"}"#,
        r#"{"method":"GET","path":"/a b"}"#,
        r#"{"method":"GET","path":"/x#frag"}"#,
        r#"{"method":"GET","path":"/x?access_token=abc"}"#,
        r#"{"method":"GET","path":"/x?access_token"}"#,
        r#"{"method":"GET","path":"/x","body":{}}"#,
    ] {
        assert!(prepare(args).is_err(), "{args}");
    }
    let long_path = format!(r#"{{"method":"GET","path":"/{}"}}"#, "a".repeat(2000));
    assert!(prepare(&long_path).is_err());
    let large_body = format!(
        r#"{{"method":"POST","path":"/x","body":{{"value":"{}"}}}}"#,
        "x".repeat(100 * 1024)
    );
    assert!(prepare(&large_body).is_err());
}

#[test]
fn effects_match_github_read_write_and_sensitive_calls() {
    let cases = [
        (r#"{"method":"GET","path":"/user/repos"}"#, Effect::Network),
        (
            r#"{"method":"POST","path":"/repos/o/r/issues"}"#,
            Effect::Push,
        ),
        (r#"{"method":"PATCH","path":"/repos/o/r"}"#, Effect::Push),
        (
            r#"{"method":"DELETE","path":"/repos/o/r"}"#,
            Effect::ForcePush,
        ),
        (
            r#"{"method":"POST","path":"/user/keys"}"#,
            Effect::ForcePush,
        ),
        (
            r#"{"method":"POST","path":"/user/%6beys"}"#,
            Effect::ForcePush,
        ),
        (
            r#"{"method":"POST","path":"/user/%4BEYS"}"#,
            Effect::ForcePush,
        ),
        (
            r#"{"method":"PUT","path":"/repos/o/r/%63ollaborators/x"}"#,
            Effect::ForcePush,
        ),
        (
            r#"{"method":"POST","path":"/graphql","body":{"query":"query Viewer { viewer { login } }"}}"#,
            Effect::Network,
        ),
        (
            r#"{"method":"POST","path":"/graphql","body":{"query":"mutation Create { createIssue }"}}"#,
            Effect::Push,
        ),
        (r#"{"method":"GET","path":"/user/keys"}"#, Effect::Network),
    ];
    for (args, expected) in cases {
        assert_eq!(
            prepare(args).expect("valid call").effect,
            expected,
            "{args}"
        );
    }
}

#[tokio::test]
async fn valid_percent_encoded_paths_keep_the_request_url_unchanged() {
    let http = FakeHttp::new(vec![response(200, &[], "{}"), response(200, &[], "{}")]);
    let factory: Arc<dyn GithubClientFactory> = Arc::new(Factory {
        http: Arc::clone(&http),
    });
    for path in ["/search/code?q=a%20b", "/repos/o/r/git/ref/heads%2Ffeature"] {
        let args = serde_json::json!({"method": "GET", "path": path}).to_string();
        let call = prepare(&args).expect("valid call");
        assert_eq!(call.effect, Effect::Network);
        github_api::execute(&call, &factory, &CancellationToken::new())
            .await
            .expect("GitHub call");
    }
    let requests = http.requests.lock().expect("request lock");
    assert_eq!(
        requests[0].url,
        "https://api.github.com/search/code?q=a%20b"
    );
    assert_eq!(
        requests[1].url,
        "https://api.github.com/repos/o/r/git/ref/heads%2Ffeature"
    );
}

#[tokio::test]
async fn execution_sends_auth_headers_formats_metadata_and_redacts_body() {
    let http = FakeHttp::new(vec![response(
        200,
        &[
            (
                "Link",
                "<https://api.github.com/user/repos?page=2>; rel=\"next\", <https://evil.test/x>; rel=\"next\"",
            ),
            ("x-ratelimit-remaining", "42"),
        ],
        r#"{"token":"ghp_test_token_123"}"#,
    )]);
    let factory: Arc<dyn GithubClientFactory> = Arc::new(Factory {
        http: Arc::clone(&http),
    });
    let call = prepare(r#"{"method":"GET","path":"/user/repos?per_page=1"}"#).expect("call");
    let output = github_api::execute(&call, &factory, &CancellationToken::new())
        .await
        .expect("GitHub call");
    assert!(
        output.starts_with("status: 200\nnext: /user/repos?page=2\nrate limit remaining: 42\n\n")
    );
    assert!(output.contains("[REDACTED]"));
    assert!(!output.contains("ghp_test_token_123"));
    let request = http.request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.url, "https://api.github.com/user/repos?per_page=1");
    assert_eq!(
        request.headers.get("accept"),
        Some(&"application/vnd.github+json".to_owned())
    );
    assert_eq!(
        request.headers.get("user-agent"),
        Some(&"Tether".to_owned())
    );
    assert_eq!(
        request.headers.get("authorization"),
        Some(&"Basic eC1hY2Nlc3MtdG9rZW46Z2hwX3Rlc3RfdG9rZW5fMTIz".to_owned())
    );
}

#[tokio::test]
async fn execution_sends_a_json_body_for_post() {
    let http = FakeHttp::new(vec![response(201, &[], r#"{"ok":true}"#)]);
    let factory: Arc<dyn GithubClientFactory> = Arc::new(Factory {
        http: Arc::clone(&http),
    });
    let call =
        prepare(r#"{"method":"POST","path":"/gists","body":{"public":false}}"#).expect("call");
    github_api::execute(&call, &factory, &CancellationToken::new())
        .await
        .expect("GitHub call");
    let request = http.request();
    assert_eq!(request.body, Some(br#"{"public":false}"#.to_vec()));
}

#[tokio::test]
async fn execution_caps_a_large_response_body() {
    let body = format!("{}é", "x".repeat(100 * 1024));
    let http = FakeHttp::new(vec![response(200, &[], &body)]);
    let factory: Arc<dyn GithubClientFactory> = Arc::new(Factory { http });
    let call = prepare(r#"{"method":"GET","path":"/user"}"#).expect("call");
    let output = github_api::execute(&call, &factory, &CancellationToken::new())
        .await
        .expect("GitHub call");
    assert!(output.ends_with("[response truncated at 48 KiB; narrow the path or use per_page]"));
}

#[tokio::test]
async fn execution_reports_non_success_statuses_with_hints() {
    for (status, body, expected) in [
        (
            401,
            r#"{"message":"bad token"}"#,
            "GitHub returned 401: bad token The saved GitHub token was rejected.",
        ),
        (
            403,
            r#"{"message":"forbidden"}"#,
            "GitHub returned 403: forbidden The saved token may lack the scope",
        ),
        (
            404,
            "not found",
            "GitHub returned 404: not found The saved token may lack the scope",
        ),
    ] {
        let http = FakeHttp::new(vec![response(status, &[], body)]);
        let factory: Arc<dyn GithubClientFactory> = Arc::new(Factory { http });
        let call = prepare(r#"{"method":"GET","path":"/user"}"#).expect("call");
        let error = github_api::execute(&call, &factory, &CancellationToken::new())
            .await
            .expect_err("status error");
        assert!(error.starts_with(expected), "{error}");
    }
}

#[tokio::test]
async fn a_missing_github_sign_in_uses_the_existing_message() {
    let factory: Arc<dyn GithubClientFactory> = Arc::new(NotSignedIn);
    let call = prepare(r#"{"method":"GET","path":"/user"}"#).expect("call");
    assert_eq!(
        github_api::execute(&call, &factory, &CancellationToken::new())
            .await
            .expect_err("missing sign in"),
        "Sign in to GitHub in Settings first"
    );
}

#[test]
fn response_body_cap_keeps_characters_intact() {
    let capped = github_api::test_cap_body(format!("{}é", "x".repeat(48 * 1024)));
    assert!(capped.ends_with("[response truncated at 48 KiB; narrow the path or use per_page]"));
    assert!(capped.is_char_boundary(capped.len()));
}
