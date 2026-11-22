use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tether_secrets::Secret;

use super::test_support::block_on;
use super::{
    CheckRun, GithubClient, GithubError, HttpClient, HttpClientError, HttpRequest, HttpResponse,
    PullRequestComment, PullRequestDraft, RepoInfo, parse_github_url, should_clone_shallow,
};

struct FakeHttp {
    responses: Mutex<VecDeque<Result<HttpResponse, HttpClientError>>>,
    requests: Mutex<Vec<HttpRequest>>,
}

impl FakeHttp {
    fn new(responses: Vec<Result<HttpResponse, HttpClientError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn request(&self, index: usize) -> HttpRequest {
        self.requests.lock().expect("request lock")[index].clone()
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

macro_rules! response {
    ($status:expr, $value:expr $(,)?) => {
        Ok(HttpResponse {
            status: $status,
            headers: std::collections::BTreeMap::new(),
            body: $value.as_bytes().to_vec(),
        })
    };
}

fn client(http: Arc<FakeHttp>) -> GithubClient {
    GithubClient::with_base_url(http, Secret::from_text("token-123"), "https://api.test")
        .expect("valid test API URL")
}

fn repo_json(default_branch: &str, private: bool, size: u64) -> String {
    format!(
        r#"{{"default_branch":"{default_branch}","private":{private},"size":{size},"clone_url":"https://github.com/acme/repo.git","permissions":{{"admin":true,"push":false}}}}"#
    )
}

fn pr_json(number: u64) -> String {
    format!(
        r#"{{"number":{number},"html_url":"https://github.com/acme/repo/pull/{number}","title":"Improve it","body":"Details","state":"open","head":{{"sha":"head-sha","ref":"feature"}},"base":{{"ref":"main"}}}}"#
    )
}

#[test]
#[expect(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "one success fixture proves all public client calls together"
)]
fn github_client_parses_success_fixtures_and_sends_safe_requests() {
    let responses = vec![
        response!(
            200,
            r#"{"login":"blair","id":42,"name":"Blair","email":"b@example.test"}"#,
        ),
        response!(200, &repo_json("main", true, 250)),
        response!(200, &format!("[{}]", repo_json("main", false, 3))),
        response!(200, &pr_json(7)),
        response!(200, &pr_json(8)),
        response!(200, &format!("[{}]", pr_json(9))),
        response!(
            200,
            r#"{"check_runs":[{"name":"CI","status":"completed","conclusion":"success","details_url":"https://ci.test/1"}]}"#,
        ),
        response!(200, r#"[{"user":{"login":"issue-user"},"body":"issue"}]"#),
        response!(
            200,
            r#"[{"user":{"login":"review-user"},"body":"review","path":"src/lib.rs","line":12}]"#,
        ),
    ];
    let http = Arc::new(FakeHttp::new(responses));
    let api = client(http.clone());
    let identity = block_on(api.current_user()).expect("current user");
    assert_eq!(identity.login, "blair");
    assert_eq!(identity.id, 42);
    assert_eq!(identity.name, "Blair");
    assert_eq!(identity.email, "b@example.test");
    assert_eq!(
        block_on(api.get_repo("acme", "repo")).expect("repo"),
        RepoInfo {
            default_branch: String::from("main"),
            private: true,
            size_kb: 250,
            clone_url: String::from("https://github.com/acme/repo.git"),
            permissions: [(String::from("admin"), true), (String::from("push"), false)]
                .into_iter()
                .collect(),
        }
    );
    assert_eq!(
        block_on(api.list_user_repos(2, 10)).expect("repos").len(),
        1
    );
    let draft = PullRequestDraft {
        head: String::from("feature"),
        base: String::from("main"),
        title: String::from("Improve it"),
        body: String::from("Details"),
        draft: true,
    };
    assert_eq!(
        block_on(api.create_pull_request("acme", "repo", draft))
            .expect("create")
            .number,
        7
    );
    assert_eq!(
        block_on(api.get_pull_request("acme", "repo", 8))
            .expect("get")
            .number,
        8
    );
    let listed = block_on(api.list_pull_requests("acme", "repo", "open")).expect("list");
    assert_eq!(listed[0].number, 9);
    assert_eq!(listed[0].head_ref, "feature");
    assert_eq!(listed[0].head_sha, "head-sha");
    assert_eq!(listed[0].base, "main");
    assert_eq!(
        block_on(api.pull_request_checks("acme", "repo", "sha")).expect("checks"),
        vec![CheckRun {
            name: String::from("CI"),
            status: String::from("completed"),
            conclusion: Some(String::from("success")),
            details_url: Some(String::from("https://ci.test/1")),
        }]
    );
    assert_eq!(
        block_on(api.pull_request_comments("acme", "repo", 7)).expect("comments"),
        vec![
            PullRequestComment {
                author: String::from("issue-user"),
                body: String::from("issue"),
                path: None,
                line: None,
            },
            PullRequestComment {
                author: String::from("review-user"),
                body: String::from("review"),
                path: Some(String::from("src/lib.rs")),
                line: Some(12),
            },
        ]
    );
    let request = http.request(0);
    assert_eq!(request.url, "https://api.test/user");
    assert_eq!(
        request.headers.get("user-agent"),
        Some(&String::from("Tether"))
    );
    assert!(request.headers.contains_key("authorization"));
    assert!(!format!("{request:?}").contains("token-123"));
}

#[test]
fn current_user_uses_verified_primary_email_or_noreply_fallback() {
    let email_http = Arc::new(FakeHttp::new(vec![
        response!(200, r#"{"login":"user","id":9,"name":null,"email":null}"#),
        response!(
            200,
            r#"[{"email":"unverified@test","primary":true,"verified":false},{"email":"verified@test","primary":true,"verified":true}]"#,
        ),
    ]));
    let identity = block_on(client(email_http).current_user()).expect("email fallback");
    assert_eq!(identity.name, "user");
    assert_eq!(identity.email, "verified@test");

    let fallback_http = Arc::new(FakeHttp::new(vec![
        response!(200, r#"{"login":"user","id":9,"name":"","email":null}"#),
        Err(HttpClientError),
    ]));
    let identity = block_on(client(fallback_http).current_user()).expect("noreply fallback");
    assert_eq!(identity.email, "9+user@users.noreply.github.com");
}

#[test]
fn github_errors_are_typed_and_do_not_leak_tokens() {
    let cases = [
        (401, "{}", GithubError::BadCredentials),
        (
            403,
            r#"{"message":"denied"}"#,
            GithubError::Forbidden {
                message: String::from("denied"),
            },
        ),
        (404, "{}", GithubError::NotFoundOrNoAccess),
        (500, "{}", GithubError::Retryable),
    ];
    for (status, body, expected) in cases {
        let http = Arc::new(FakeHttp::new(vec![response!(status, body)]));
        let error = block_on(client(http).get_repo("acme", "repo")).expect_err("typed error");
        assert_eq!(error, expected);
        assert!(!error.to_string().contains("token-123"));
        assert!(!format!("{error:?}").contains("token-123"));
    }
    let rate_http = Arc::new(FakeHttp::new(vec![Ok(HttpResponse {
        status: 403,
        headers: [
            (String::from("x-ratelimit-remaining"), String::from("0")),
            (String::from("x-ratelimit-reset"), String::from("1234")),
        ]
        .into_iter()
        .collect(),
        body: Vec::new(),
    })]));
    assert_eq!(
        block_on(client(rate_http).get_repo("acme", "repo")).expect_err("rate limit"),
        GithubError::RateLimited { reset_at: 1234 }
    );
    let validation = Arc::new(FakeHttp::new(vec![response!(
        422,
        r#"{"message":"bad","errors":[{"message":"title required"},{"message":"head invalid"}]}"#,
    )]));
    assert_eq!(
        block_on(client(validation).get_repo("acme", "repo")).expect_err("validation"),
        GithubError::Validation {
            messages: vec![String::from("title required"), String::from("head invalid")]
        }
    );
}

#[test]
fn github_url_and_shallow_clone_rules_are_exact() {
    let cases = [
        ("https://github.com/acme/repo", Some(("acme", "repo"))),
        ("https://github.com/acme/repo.git", Some(("acme", "repo"))),
        ("http://github.com/acme/repo", None),
        ("https://user@github.com/acme/repo", None),
        ("https://github.com/acme/repo?x=1", None),
        ("https://github.com/acme", None),
        ("https://evil.test/acme/repo", None),
    ];
    for (url, expected) in cases {
        let actual = parse_github_url(url);
        assert_eq!(
            actual
                .as_ref()
                .map(|(owner, repo)| (owner.as_str(), repo.as_str())),
            expected
        );
    }
    assert!(!should_clone_shallow(200 * 1024));
    assert!(should_clone_shallow(200 * 1024 + 1));
}

#[test]
fn github_rejects_oversized_fake_response() {
    let http = Arc::new(FakeHttp::new(vec![Ok(HttpResponse {
        status: 200,
        headers: std::collections::BTreeMap::new(),
        body: vec![b'x'; 8 * 1024 * 1024 + 1],
    })]));
    let error = block_on(client(http).get_repo("acme", "repo")).expect_err("size limit");
    assert_eq!(error, GithubError::InvalidResponse);
}
