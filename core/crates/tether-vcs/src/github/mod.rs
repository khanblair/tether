//! GitHub REST API support.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tether_secrets::Secret;
use thiserror::Error;

mod client;
mod helpers;
pub use helpers::{parse_github_url, should_clone_shallow};

const API_URL: &str = "https://api.github.com";
const MAX_BODY: usize = 8 * 1024 * 1024;

fn redirect_action(
    cross_host: bool,
    attempt: reqwest::redirect::Attempt,
) -> reqwest::redirect::Action {
    if cross_host {
        attempt.stop()
    } else {
        attempt.follow()
    }
}
fn add_headers(
    mut request: reqwest::RequestBuilder,
    headers: BTreeMap<String, String>,
) -> reqwest::RequestBuilder {
    for (key, value) in headers {
        request = request.header(key, value);
    }
    request
}
fn add_body(request: reqwest::RequestBuilder, body: Option<Vec<u8>>) -> reqwest::RequestBuilder {
    match body {
        Some(body) => request.body(body),
        None => request,
    }
}
fn header_pair(
    pair: (&reqwest::header::HeaderName, &reqwest::header::HeaderValue),
) -> Option<(String, String)> {
    let (key, value) = pair;
    value
        .to_str()
        .ok()
        .map(|value| (key.to_string(), value.to_owned()))
}
fn ensure_body_limit(body: &[u8]) -> Result<(), HttpClientError> {
    if body.len() > MAX_BODY {
        Err(HttpClientError)
    } else {
        Ok(())
    }
}

/// An HTTP request passed to a testable client.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// HTTP method.
    pub method: String,
    /// Absolute HTTPS URL.
    pub url: String,
    /// Request headers.
    pub headers: BTreeMap<String, String>,
    /// Optional request body.
    pub body: Option<Vec<u8>>,
}

/// An HTTP response returned by a testable client.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// Status code.
    pub status: u16,
    /// Response headers.
    pub headers: BTreeMap<String, String>,
    /// Response bytes.
    pub body: Vec<u8>,
}

/// A transport failure with no credential data.
#[derive(Debug, Error)]
#[error("HTTP transport failed")]
pub struct HttpClientError;

/// The HTTP seam used by [`GithubClient`].
pub trait HttpClient: Send + Sync {
    /// Sends one request.
    fn send(
        &self,
        request: HttpRequest,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpClientError>> + Send + '_>>;
}

/// A production HTTP client backed by reqwest.
#[derive(Debug, Clone)]
pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    /// Creates a client with same-host-only redirects and bounded timeouts.
    ///
    /// # Errors
    /// Returns an error when reqwest cannot build the client.
    pub fn new() -> Result<Self, HttpClientError> {
        let policy = reqwest::redirect::Policy::custom(|attempt| {
            let previous = attempt.previous().last().and_then(reqwest::Url::host_str);
            let next = attempt.url().host_str();
            redirect_action(previous.is_some() && previous != next, attempt)
        });
        let client = reqwest::Client::builder()
            .redirect(policy)
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|_| HttpClientError)?;
        Ok(Self { client })
    }
}

impl HttpClient for ReqwestHttpClient {
    /// Sends a request and rejects bodies over eight MiB.
    fn send(
        &self,
        request: HttpRequest,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpClientError>> + Send + '_>> {
        Box::pin(async move {
            let method = reqwest::Method::from_bytes(request.method.as_bytes())
                .map_err(|_| HttpClientError)?;
            let call = add_body(
                add_headers(self.client.request(method, request.url), request.headers),
                request.body,
            );
            let response = call.send().await.map_err(|_| HttpClientError)?;
            let status = response.status().as_u16();
            let headers = response.headers().iter().filter_map(header_pair).collect();
            let body = response
                .bytes()
                .await
                .map_err(|_| HttpClientError)?
                .to_vec();
            ensure_body_limit(&body)?;
            Ok(HttpResponse {
                status,
                headers,
                body,
            })
        })
    }
}

/// A safe GitHub error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GithubError {
    /// Authentication failed.
    #[error("GitHub credentials were rejected")]
    BadCredentials,
    /// The request is rate limited.
    #[error("GitHub rate limit exceeded until {reset_at}")]
    RateLimited {
        /// Reset Unix time.
        reset_at: i64,
    },
    /// The account cannot access the resource.
    #[error("GitHub access was forbidden: {message}")]
    Forbidden {
        /// GitHub's safe message.
        message: String,
    },
    /// The resource was absent or inaccessible.
    #[error("GitHub resource was not found or is inaccessible")]
    NotFoundOrNoAccess,
    /// Validation failed.
    #[error("GitHub request was invalid: {messages:?}")]
    Validation {
        /// Field messages.
        messages: Vec<String>,
    },
    /// A retryable server or network failure.
    #[error("GitHub request failed and can be retried")]
    Retryable,
    /// GitHub returned an invalid response.
    #[error("GitHub response was invalid or too large")]
    InvalidResponse,
    /// The request could not be formed.
    #[error("GitHub request URL is invalid")]
    InvalidRequest,
}

/// The authenticated GitHub identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubIdentity {
    /// Login.
    pub login: String,
    /// Numeric id.
    pub id: u64,
    /// Display name.
    pub name: String,
    /// Primary verified email.
    pub email: String,
}

/// Repository metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoInfo {
    /// Default branch.
    pub default_branch: String,
    /// Whether private.
    pub private: bool,
    /// Size in KiB.
    pub size_kb: u64,
    /// HTTPS clone URL.
    pub clone_url: String,
    /// Permission names and flags.
    pub permissions: BTreeMap<String, bool>,
}

/// Pull request input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestDraft {
    /// Head branch.
    pub head: String,
    /// Base branch.
    pub base: String,
    /// Title.
    pub title: String,
    /// Body.
    pub body: String,
    /// Whether draft.
    pub draft: bool,
}

/// Pull request data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestInfo {
    /// Number.
    pub number: u64,
    /// HTML URL.
    pub url: String,
    /// Title.
    pub title: String,
    /// Body.
    pub body: String,
    /// State.
    pub state: String,
    /// Head SHA.
    pub head_sha: String,
    /// Head branch name.
    pub head_ref: String,
    /// Base branch.
    pub base: String,
}

/// A check run summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckRun {
    /// Check name.
    pub name: String,
    /// Status.
    pub status: String,
    /// Conclusion.
    pub conclusion: Option<String>,
    /// Details URL.
    pub details_url: Option<String>,
}

/// A merged issue or review comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestComment {
    /// Author login.
    pub author: String,
    /// Comment body.
    pub body: String,
    /// Optional path.
    pub path: Option<String>,
    /// Optional one-based line.
    pub line: Option<u64>,
}

/// A GitHub pull request client.
pub struct GithubClient {
    pub(crate) http: Arc<dyn HttpClient>,
    pub(crate) token: Secret,
    pub(crate) base_url: String,
}
