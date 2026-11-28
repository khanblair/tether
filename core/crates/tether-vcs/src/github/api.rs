use std::collections::BTreeMap;

use serde_json::Value;
use tether_secrets::Redactor;

use super::helpers::base64_encode;
use super::{GithubClient, GithubError, HttpRequest, HttpResponse, Secret};

const MAX_REQUEST_BODY: usize = 100 * 1024;

/// A raw response from a GitHub API call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubApiResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response body as redacted text.
    pub body: String,
    /// The next-page path and query from GitHub's Link header.
    pub next: Option<String>,
    /// Remaining GitHub API requests, when the header is present and numeric.
    pub rate_limit_remaining: Option<u64>,
}

impl GithubClient {
    pub(super) async fn send_request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<HttpResponse, GithubError> {
        let headers = auth_headers(&self.token)?;
        let body = body.as_ref().map(serialize_body).transpose()?;
        self.http
            .send(HttpRequest {
                method: method.to_owned(),
                url: format!("{}{}", self.base_url, path),
                headers,
                body,
            })
            .await
            .map_err(|_| GithubError::Retryable)
    }

    /// Calls GitHub's API with the saved account token.
    ///
    /// # Errors
    /// Returns a safe error when the request cannot be built or sent.
    pub async fn api(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<GithubApiResponse, GithubError> {
        let response = self.send_request(method, path, body).await?;
        let mut redactor = Redactor::default();
        redactor.register(&self.token);
        let body = redactor.redact(&String::from_utf8_lossy(&response.body));
        let next = next_link(&response).map(|value| redactor.redact(&value));
        Ok(GithubApiResponse {
            status: response.status,
            body,
            next,
            rate_limit_remaining: header(&response, "x-ratelimit-remaining")
                .and_then(|value| value.parse().ok()),
        })
    }
}

fn serialize_body(value: &Value) -> Result<Vec<u8>, GithubError> {
    let body = serde_json::to_vec(&value).map_err(|_| GithubError::InvalidRequest)?;
    if body.len() > MAX_REQUEST_BODY {
        Err(GithubError::InvalidRequest)
    } else {
        Ok(body)
    }
}

fn auth_headers(token: &Secret) -> Result<BTreeMap<String, String>, GithubError> {
    let token = token
        .expose_text()
        .map_err(|_| GithubError::InvalidRequest)?;
    Ok(BTreeMap::from([
        (
            String::from("accept"),
            String::from("application/vnd.github+json"),
        ),
        (String::from("user-agent"), String::from("Tether")),
        (
            String::from("authorization"),
            format!(
                "Basic {}",
                base64_encode(format!("x-access-token:{token}").as_bytes())
            ),
        ),
    ]))
}

fn header<'a>(response: &'a HttpResponse, name: &str) -> Option<&'a str> {
    response
        .headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn next_link(response: &HttpResponse) -> Option<String> {
    let link = header(response, "link")?;
    link.split(',').find_map(next_link_target)
}

fn next_link_target(link: &str) -> Option<String> {
    let (target, parameters) = link.split_once('>')?;
    if !parameters.contains("rel=\"next\"") && !parameters.contains("rel=next") {
        return None;
    }
    let url = target.strip_prefix('<')?;
    let parsed = reqwest::Url::parse(url).ok()?;
    if parsed.host_str() != Some("api.github.com") {
        return None;
    }
    let mut path = parsed.path().to_owned();
    if let Some(query) = parsed.query() {
        path.push('?');
        path.push_str(query);
    }
    Some(path)
}
