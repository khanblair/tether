//! Contract tests for the GitHub Actions client, poller, and log parser.

use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};

use tether_ci::{
    CiClient, CiError, CiFailure, CiProgress, CiRunState, DispatchInputs, JobProgress, PollConfig,
    RunRef, Sleeper, parse_failures, poll_run, trim_log, workflow_template,
};
use tether_secrets::Secret;
use tether_vcs::{HttpClient, HttpClientError, HttpRequest, HttpResponse};
use tokio_util::sync::CancellationToken;

struct FakeHttp {
    responses: Mutex<VecDeque<Result<HttpResponse, HttpClientError>>>,
    requests: Mutex<Vec<HttpRequest>>,
}

impl FakeHttp {
    fn new(
        responses: impl IntoIterator<Item = Result<HttpResponse, HttpClientError>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into_iter().collect()),
            requests: Mutex::new(Vec::new()),
        })
    }

    fn requests(&self) -> Vec<HttpRequest> {
        self.requests.lock().expect("request lock").clone()
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

#[expect(
    clippy::unnecessary_wraps,
    reason = "FakeHttp consumes canned Result responses."
)]
fn response(status: u16, body: &str) -> Result<HttpResponse, HttpClientError> {
    Ok(HttpResponse {
        status,
        headers: BTreeMap::new(),
        body: body.as_bytes().to_vec(),
    })
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "FakeHttp consumes canned Result responses."
)]
fn response_with_headers(
    status: u16,
    headers: &[(&str, &str)],
    body: &str,
) -> Result<HttpResponse, HttpClientError> {
    Ok(HttpResponse {
        status,
        headers: headers
            .iter()
            .map(|(key, value)| ((*key).into(), (*value).into()))
            .collect(),
        body: body.as_bytes().to_vec(),
    })
}

fn client(http: Arc<FakeHttp>) -> CiClient {
    CiClient::for_repository(http, Secret::from_text("ci-test-token"), "acme", "widget")
}

#[tokio::test]
async fn client_sends_exact_dispatch_request() {
    let http = FakeHttp::new([response(204, "")]);
    let api = client(Arc::clone(&http));

    api.dispatch(
        "acme",
        "widget",
        "feature/api",
        DispatchInputs {
            command: "cargo test".into(),
            request_id: "request-7".into(),
            setup: Some("rust".into()),
        },
    )
    .await
    .expect("dispatch");

    let requests = http.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(
        requests[0].url,
        "https://api.github.com/repos/acme/widget/actions/workflows/tether.yml/dispatches"
    );
    assert_eq!(
        requests[0].headers,
        BTreeMap::from([
            ("accept".into(), "application/vnd.github+json".into()),
            (
                "authorization".into(),
                "Basic eC1hY2Nlc3MtdG9rZW46Y2ktdGVzdC10b2tlbg==".into()
            ),
            ("user-agent".into(), "Tether".into()),
        ])
    );
    assert_eq!(
        String::from_utf8(requests[0].body.clone().expect("body")).expect("utf8"),
        r#"{"inputs":{"command":"cargo test","request_id":"request-7","setup":"rust"},"ref":"feature/api"}"#
    );
    assert!(!requests[0].url.contains("ci-test-token"));
}

#[tokio::test]
async fn client_requests_runs_jobs_cancel_and_lists_newest_twenty() {
    let http = FakeHttp::new([
        response(
            200,
            r#"{"id":9,"run_number":4,"display_title":"tether test","head_branch":"main","status":"queued","conclusion":null,"created_at":"2026-11-23T10:00:00Z","html_url":"https://github.com/acme/widget/actions/runs/9"}"#,
        ),
        response(
            200,
            r#"{"jobs":[{"id":3,"name":"build","status":"in_progress","conclusion":null,"steps":[{"name":"Test","status":"in_progress","conclusion":null}]}]}"#,
        ),
        response(202, ""),
        response(200, &runs_json()),
    ]);
    let api = client(Arc::clone(&http));

    assert_eq!(api.get_run(9).await.expect("run").id, 9);
    assert_eq!(
        api.run_jobs(9).await.expect("jobs")[0].steps[0].name,
        "Test"
    );
    api.cancel_run(9).await.expect("cancel");
    let listed = api.list_runs("acme", "widget", 99).await.expect("list");
    assert_eq!(listed.len(), 20);
    assert_eq!(listed[0].id, 25);
    assert_eq!(listed[19].id, 6);

    let requests = http.requests();
    assert_eq!(requests[0].method, "GET");
    assert_eq!(
        requests[0].url,
        "https://api.github.com/repos/acme/widget/actions/runs/9"
    );
    assert_eq!(
        requests[1].url,
        "https://api.github.com/repos/acme/widget/actions/runs/9/jobs?per_page=100"
    );
    assert_eq!(requests[2].method, "POST");
    assert_eq!(
        requests[2].url,
        "https://api.github.com/repos/acme/widget/actions/runs/9/cancel"
    );
    assert_eq!(
        requests[3].url,
        "https://api.github.com/repos/acme/widget/actions/workflows/tether.yml/runs?per_page=20"
    );
}

fn runs_json() -> String {
    let runs = (1..=25)
        .map(|id| {
            format!(
                r#"{{"id":{id},"run_number":{id},"display_title":"tether command {id}","head_branch":"main","status":"completed","conclusion":"success","created_at":"2026-11-{id:02}T00:00:00Z","run_started_at":"2026-11-{id:02}T00:00:00Z","updated_at":"2026-11-{id:02}T00:00:10Z","html_url":"https://github.com/acme/widget/actions/runs/{id}"}}"#
            )
        })
        .collect::<Vec<_>>();
    format!(r#"{{"workflow_runs":[{}]}}"#, runs.join(","))
}

#[tokio::test]
async fn find_run_selects_matching_recent_run_and_encodes_branch() {
    let http = FakeHttp::new([response(
        200,
        r#"{"workflow_runs":[{"id":1,"display_title":"tether request-7","head_branch":"feature/api","status":"queued","conclusion":null,"created_at":"2026-11-22T23:59:59Z","html_url":"u1"},{"id":2,"display_title":"other request-8","head_branch":"feature/api","status":"queued","conclusion":null,"created_at":"2026-11-23T10:00:00Z","html_url":"u2"},{"id":3,"display_title":"tether request-7","head_branch":"feature/api","status":"queued","conclusion":null,"created_at":"2026-11-23T10:00:01Z","html_url":"u3"}]}"#,
    )]);
    let api = client(Arc::clone(&http));
    let found = api
        .find_run(
            "acme",
            "widget",
            "feature/api",
            "request-7",
            UNIX_EPOCH + Duration::from_secs(1_795_428_000),
        )
        .await
        .expect("find")
        .expect("matching run");

    assert_eq!(found.id, 3);
    assert_eq!(
        http.requests()[0].url,
        "https://api.github.com/repos/acme/widget/actions/workflows/tether.yml/runs?event=workflow_dispatch&branch=feature%2Fapi&per_page=20"
    );
}

#[tokio::test]
async fn job_log_follows_redirect_without_auth_and_enforces_eight_mib_cap() {
    let signed_url = "https://logs.example.test/signed?token=not-a-github-token";
    let http = FakeHttp::new([
        response_with_headers(302, &[("location", signed_url)], ""),
        response(200, "log ci-test-token body"),
    ]);
    let api = client(Arc::clone(&http));

    assert_eq!(api.job_log(44).await.expect("log"), "log [redacted] body");
    let requests = http.requests();
    assert!(requests[0].headers.contains_key("authorization"));
    assert!(requests[1].headers.is_empty());
    assert_eq!(requests[1].url, signed_url);

    let cap = vec![b'x'; 8 * 1024 * 1024 + 1];
    let oversized = FakeHttp::new([
        response_with_headers(302, &[("location", signed_url)], ""),
        Ok(HttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: cap,
        }),
    ]);
    let error = client(oversized).job_log(44).await.expect_err("cap");
    assert_eq!(error, CiError::InvalidResponse);
}

#[tokio::test]
async fn client_maps_github_errors_without_returning_the_token() {
    let token = "ci-test-token";
    let cases = [
        (401, BTreeMap::new(), "{}", CiError::BadCredentials),
        (
            403,
            BTreeMap::from([
                ("x-ratelimit-remaining".into(), "0".into()),
                ("x-ratelimit-reset".into(), "1770000000".into()),
            ]),
            "{}",
            CiError::RateLimited {
                reset_at: 1_770_000_000,
            },
        ),
        (
            403,
            BTreeMap::new(),
            r#"{"message":"permission denied"}"#,
            CiError::Forbidden {
                message: "permission denied".into(),
            },
        ),
        (
            404,
            BTreeMap::new(),
            r#"{"message":"Workflow not found"}"#,
            CiError::WorkflowMissing(tether_ci::WorkflowMissing),
        ),
        (
            422,
            BTreeMap::new(),
            r#"{"message":"invalid","errors":[{"message":"ref is required"}]}"#,
            CiError::Validation {
                messages: vec!["ref is required".into()],
            },
        ),
    ];
    for (status, headers, body, expected) in cases {
        let http = FakeHttp::new([Ok(HttpResponse {
            status,
            headers,
            body: body.as_bytes().to_vec(),
        })]);
        let error = client(http)
            .dispatch("acme", "widget", "main", inputs())
            .await
            .expect_err("error");
        assert_eq!(error, expected);
        assert!(!error.to_string().contains(token));
    }
}

fn inputs() -> DispatchInputs {
    DispatchInputs {
        command: "true".into(),
        request_id: "request-1".into(),
        setup: None,
    }
}

#[derive(Default)]
struct RecordingSleeper {
    durations: Mutex<Vec<Duration>>,
}

impl Sleeper for RecordingSleeper {
    fn sleep<'a>(&'a self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        self.durations.lock().expect("sleep lock").push(duration);
        Box::pin(async {})
    }
}

fn run_ref() -> RunRef {
    RunRef {
        id: 7,
        display_title: "tether test".into(),
        branch: "main".into(),
        status: "queued".into(),
        conclusion: None,
        created_at: "2026-11-23T10:00:00Z".into(),
        html_url: "https://github.com/acme/widget/actions/runs/7".into(),
    }
}

fn run_body(status: &str, conclusion: Option<&str>) -> String {
    format!(
        r#"{{"id":7,"run_number":1,"display_title":"tether test","head_branch":"main","status":"{status}","conclusion":{conclusion},"created_at":"2026-11-23T10:00:00Z","html_url":"https://github.com/acme/widget/actions/runs/7"}}"#,
        conclusion = conclusion.map_or_else(|| "null".into(), |value| format!(r#""{value}""#))
    )
}

fn jobs_body(status: &str, conclusion: Option<&str>, step_status: &str) -> String {
    format!(
        r#"{{"jobs":[{{"id":8,"name":"build","status":"{status}","conclusion":{conclusion},"steps":[{{"name":"Test","status":"{step_status}","conclusion":{conclusion}}}]}}]}}"#,
        conclusion = conclusion.map_or_else(|| "null".into(), |value| format!(r#""{value}""#))
    )
}

#[tokio::test]
async fn poll_run_records_backoff_reset_progress_and_success() {
    let http = poll_success_http();
    let api = client(Arc::clone(&http));
    let sleeper = RecordingSleeper::default();
    let progress = Arc::new(Mutex::new(Vec::<CiProgress>::new()));
    let captured = Arc::clone(&progress);

    let outcome = poll_run(
        &api,
        run_ref(),
        PollConfig {
            timeout: Duration::from_secs(60),
        },
        &sleeper,
        CancellationToken::new(),
        move |event| captured.lock().expect("progress lock").push(event),
    )
    .await
    .expect("poll");

    assert_eq!(outcome.state, CiRunState::Succeeded);
    assert_eq!(
        sleeper.durations.lock().expect("sleep lock").as_slice(),
        [
            Duration::from_secs(10),
            Duration::from_secs(15),
            Duration::from_secs(22) + Duration::from_millis(500),
            Duration::from_secs(10)
        ]
    );
    let progress = progress.lock().expect("progress lock");
    assert_eq!(progress.len(), 3);
    assert_eq!(progress[0].state, CiRunState::Queued);
    assert_eq!(progress[1].state, CiRunState::InProgress);
    assert_eq!(
        progress[1].jobs[0],
        JobProgress {
            name: "build".into(),
            status: "in_progress".into(),
            conclusion: None,
            current_step: Some("Test".into())
        }
    );
    assert_eq!(progress[2].state, CiRunState::Succeeded);
}

fn poll_success_http() -> Arc<FakeHttp> {
    FakeHttp::new([
        response(200, &run_body("queued", None)),
        response(200, &jobs_body("queued", None, "queued")),
        response(200, &run_body("queued", None)),
        response(200, &jobs_body("queued", None, "queued")),
        response(200, &run_body("queued", None)),
        response(200, &jobs_body("queued", None, "queued")),
        response(200, &run_body("in_progress", None)),
        response(200, &jobs_body("in_progress", None, "in_progress")),
        response(200, &run_body("completed", Some("success"))),
        response(200, &jobs_body("completed", Some("success"), "completed")),
    ])
}

#[tokio::test]
async fn poll_run_returns_failed_state_and_job_conclusion() {
    let http = FakeHttp::new([
        response(200, &run_body("completed", Some("failure"))),
        response(200, &jobs_body("completed", Some("failure"), "completed")),
    ]);
    let api = client(http);
    let progress = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&progress);
    let outcome = poll_run(
        &api,
        run_ref(),
        PollConfig::default(),
        &RecordingSleeper::default(),
        CancellationToken::new(),
        move |event| captured.lock().expect("progress lock").push(event),
    )
    .await
    .expect("failed outcome");

    assert_eq!(outcome.state, CiRunState::Failed);
    assert_eq!(outcome.jobs[0].conclusion.as_deref(), Some("failure"));
    assert_eq!(
        progress.lock().expect("progress lock")[0].state,
        CiRunState::Failed
    );
}

#[tokio::test]
async fn poll_run_cancels_on_token_and_on_timeout() {
    assert_eq!(
        poll_cancel_case(true, Duration::from_secs(60)).await,
        CiRunState::Cancelled
    );
    assert_eq!(
        poll_cancel_case(false, Duration::ZERO).await,
        CiRunState::TimedOut
    );
}

async fn poll_cancel_case(cancel: bool, timeout: Duration) -> CiRunState {
    let http = FakeHttp::new([response(202, "")]);
    let api = client(Arc::clone(&http));
    let token = CancellationToken::new();
    if cancel {
        token.cancel();
    }
    let outcome = poll_run(
        &api,
        run_ref(),
        PollConfig { timeout },
        &RecordingSleeper::default(),
        token,
        |_| {},
    )
    .await
    .expect("cancelled outcome");
    assert_eq!(
        http.requests()[0].url,
        "https://api.github.com/repos/acme/widget/actions/runs/7/cancel"
    );
    outcome.state
}

#[test]
fn parser_normalizes_paths_rejects_escape_and_caps_failures() {
    let log = "e: file:///home/runner/work/app/app/src/Main.kt:4:2 bad type\n/home/outside.kt:8:1: error: outside\n../escape.rs:2:1: error: escape";
    let failures = parse_failures(log, "/home/runner/work/app/app/");
    assert_eq!(failures[0].file.as_deref(), Some("src/Main.kt"));
    assert_eq!(failures[0].line, Some(4));
    assert_eq!(failures[1].file, None);
    assert!(failures[1].message.contains("/home/outside.kt:8:1"));
    assert_eq!(failures[2].file, None);
    assert!(failures[2].message.contains("../escape.rs:2:1"));

    let many = (0..60)
        .map(|index| format!("src/test{index}.rs:1:1: error: failure {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(parse_failures(&many, "/workspace/").len(), 50);
}

#[test]
fn trim_log_removes_ansi_timestamps_groups_and_keeps_bounded_tail() {
    let raw = "2026-11-21T10:00:00.1234567Z \u{1b}[31m##[group]Tests\u{1b}[0m\n2026-11-21T10:00:01.1234567Z old\n##[endgroup]\n##[error]failed";
    let trimmed = trim_log(raw, 120);
    assert_eq!(trimmed.text, "Tests\nold\n\n##[error]failed");
    assert!(!trimmed.truncated);

    let cut = trim_log(
        &(0..80)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n"),
        40,
    );
    assert!(cut.truncated);
    assert!(cut.text.ends_with("line 79"));
    assert!(cut.text.contains("[... "));
    assert!(cut.text.chars().count() <= 40);

    let tiny = trim_log(
        &(0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n"),
        32,
    );
    assert!(tiny.truncated);
    assert!(tiny.text.chars().count() <= 32);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The fixture table asserts every parsed field exactly."
)]
fn fixture_logs_have_exact_failures() {
    let cases = [
        ("pytest_pass.log", 0, None, None, None),
        (
            "pytest_fail.log",
            1,
            Some("pytest"),
            Some("tests/test_api.py"),
            None,
        ),
        ("jest_pass.log", 0, None, None, None),
        (
            "jest_fail.log",
            1,
            Some("jest"),
            Some("tests/api.test.ts"),
            Some(27),
        ),
        ("vitest_pass.log", 0, None, None, None),
        (
            "vitest_fail.log",
            1,
            Some("vitest"),
            Some("tests/math.test.ts"),
            Some(9),
        ),
        ("cargo_pass.log", 0, None, None, None),
        (
            "cargo_fail.log",
            1,
            Some("cargo test"),
            Some("src/lib.rs"),
            Some(8),
        ),
        ("gradle_pass.log", 0, None, None, None),
        (
            "gradle_fail.log",
            1,
            Some("gradle"),
            Some("src/test/kotlin/ApiTest.kt"),
            Some(42),
        ),
        (
            "gradle_compile.log",
            1,
            Some("gradle"),
            Some("src/main/kotlin/Main.kt"),
            Some(4),
        ),
    ];
    for (name, count, framework, file, line) in cases {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/logs")
            .join(name);
        let log = std::fs::read_to_string(path).expect("fixture");
        let failures = parse_failures(&log, "/home/runner/work/app/app/");
        assert_eq!(failures.len(), count, "{name}");
        if let Some(framework) = framework {
            assert_eq!(failures[0].framework, framework, "{name}");
            assert_eq!(failures[0].file.as_deref(), file, "{name}");
            assert_eq!(failures[0].line, line, "{name}");
        }
        let expected = match name {
            "pytest_fail.log" => CiFailure {
                framework: "pytest".into(),
                test_name: Some("test_auth".into()),
                file: Some("tests/test_api.py".into()),
                line: None,
                column: None,
                message: "assert response.status_code == 200".into(),
                excerpt: "tests/test_api.py::test_auth FAILED                         [100%]".into(),
            },
            "jest_fail.log" => CiFailure {
                framework: "jest".into(),
                test_name: Some("API › rejects an invalid token".into()),
                file: Some("tests/api.test.ts".into()),
                line: Some(27),
                column: Some(11),
                message: "JavaScript test failed".into(),
                excerpt: "● API › rejects an invalid token".into(),
            },
            "vitest_fail.log" => CiFailure {
                framework: "vitest".into(),
                test_name: Some("math > adds".into()),
                file: Some("tests/math.test.ts".into()),
                line: Some(9),
                column: Some(5),
                message: "JavaScript test failed".into(),
                excerpt: "❯ math > adds".into(),
            },
            "cargo_fail.log" => CiFailure {
                framework: "cargo test".into(),
                test_name: Some("tests::auth".into()),
                file: Some("src/lib.rs".into()),
                line: Some(8),
                column: Some(3),
                message: "assertion failed: left == right".into(),
                excerpt: "---- tests::auth stdout ----\nthread 'tests::auth' panicked at src/lib.rs:8:3:".into(),
            },
            "gradle_fail.log" => CiFailure {
                framework: "gradle".into(),
                test_name: Some("rejects invalid input".into()),
                file: Some("src/test/kotlin/ApiTest.kt".into()),
                line: Some(42),
                column: None,
                message: "Gradle test failed".into(),
                excerpt: "Test > rejects invalid input FAILED".into(),
            },
            "gradle_compile.log" => CiFailure {
                framework: "gradle".into(),
                test_name: None,
                file: Some("src/main/kotlin/Main.kt".into()),
                line: Some(4),
                column: Some(2),
                message: "Type mismatch: inferred type is String but Int was expected".into(),
                excerpt: "e: file:///home/runner/work/app/app/src/main/kotlin/Main.kt:4:2 Type mismatch: inferred type is String but Int was expected".into(),
            },
            _ => continue,
        };
        assert_eq!(failures[0], expected, "{name}");
    }
}

#[test]
fn pytest_traceback_tail_has_location() {
    let failures = parse_failures(
        "tests/test_api.py:14: AssertionError",
        "/home/runner/work/app/app/",
    );
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].framework, "pytest");
    assert_eq!(failures[0].file.as_deref(), Some("tests/test_api.py"));
    assert_eq!(failures[0].line, Some(14));
    assert_eq!(failures[0].message, "AssertionError");
}

#[test]
fn javascript_fail_path_and_generic_compiler_error_have_frameworks_and_locations() {
    let javascript = parse_failures("FAIL tests/api.test.ts", "/workspace/");
    assert_eq!(javascript.len(), 1);
    assert_eq!(javascript[0].framework, "jest");
    assert_eq!(javascript[0].test_name, None);
    assert_eq!(javascript[0].file.as_deref(), Some("tests/api.test.ts"));

    let compiler = parse_failures("src/main.c:2:3: error: expected ';'", "/workspace/");
    assert_eq!(compiler.len(), 1);
    assert_eq!(compiler[0].framework, "generic");
    assert_eq!(compiler[0].file.as_deref(), Some("src/main.c"));
    assert_eq!(compiler[0].line, Some(2));
    assert_eq!(compiler[0].column, Some(3));
    assert_eq!(compiler[0].message, "expected ';'");

    let vitest = parse_failures(
        "Vitest v2.0.0\n ❯ math > adds\n   at adds (tests/math.test.ts:9:5)",
        "/workspace/",
    );
    assert_eq!(vitest.len(), 1);
    assert_eq!(vitest[0].framework, "vitest");
    assert_eq!(vitest[0].file.as_deref(), Some("tests/math.test.ts"));
    assert_eq!(vitest[0].line, Some(9));
}

#[test]
fn template_has_all_dispatch_inputs_and_safe_run_shape() {
    let template = workflow_template();
    for expected in [
        "run-name: tether ${{ inputs.request_id }}",
        "command:",
        "request_id:",
        "setup:",
        "required: true",
        "permissions:",
        "contents: read",
        "runs-on: ubuntu-latest",
        "timeout-minutes: 30",
        "actions/checkout@v4",
        "actions/setup-python@v5",
        "actions/setup-node@v4",
        "dtolnay/rust-toolchain@stable",
        "actions/setup-java@v4",
        "java-version: '17'",
        "TETHER_COMMAND: ${{ inputs.command }}",
    ] {
        assert!(template.contains(expected), "missing {expected}");
    }
    let in_run = template
        .lines()
        .filter(|line| line.trim_start().starts_with("run:"))
        .collect::<Vec<_>>();
    assert_eq!(
        in_run,
        vec![r#"        run: bash -euo pipefail -c "$TETHER_COMMAND""#]
    );
    assert!(
        template
            .lines()
            .any(|line| line == "      - name: Run command")
    );
}
