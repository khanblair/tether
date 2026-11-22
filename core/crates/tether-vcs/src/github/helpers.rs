use super::{GithubError, HttpResponse, PullRequestComment, PullRequestInfo, RepoInfo};

/// Returns whether a repository should use a shallow clone.
#[must_use]
pub fn should_clone_shallow(size_kb: u64) -> bool {
    size_kb > 200 * 1024
}

/// Parses an HTTPS GitHub repository URL.
#[must_use]
pub fn parse_github_url(url: &str) -> Option<(String, String)> {
    let parsed = reqwest::Url::parse(url).ok()?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return None;
    }
    let parts: Vec<_> = parsed.path_segments()?.filter(|v| !v.is_empty()).collect();
    if parts.len() != 2 {
        return None;
    }
    let repo = parts[1].strip_suffix(".git").unwrap_or(parts[1]);
    (!parts[0].is_empty() && !repo.is_empty()).then(|| (parts[0].to_owned(), repo.to_owned()))
}

pub(super) fn map_status(response: &HttpResponse) -> Result<(), GithubError> {
    match response.status {
        200..=299 => Ok(()),
        401 => Err(GithubError::BadCredentials),
        403 if response
            .headers
            .get("x-ratelimit-remaining")
            .map(String::as_str)
            == Some("0") =>
        {
            Err(GithubError::RateLimited {
                reset_at: response
                    .headers
                    .get("x-ratelimit-reset")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_default(),
            })
        }
        403 => Err(GithubError::Forbidden {
            message: github_message(response),
        }),
        404 => Err(GithubError::NotFoundOrNoAccess),
        422 => Err(GithubError::Validation {
            messages: validation_messages(response),
        }),
        500..=599 => Err(GithubError::Retryable),
        _ => Err(GithubError::InvalidResponse),
    }
}
pub(super) fn github_message(response: &HttpResponse) -> String {
    serde_json::from_slice::<serde_json::Value>(&response.body)
        .ok()
        .and_then(|v| v.get("message").and_then(|v| v.as_str()).map(str::to_owned))
        .unwrap_or_else(|| String::from("request denied"))
}
pub(super) fn validation_messages(response: &HttpResponse) -> Vec<String> {
    serde_json::from_slice::<serde_json::Value>(&response.body)
        .ok()
        .and_then(|v| {
            v.get("errors").and_then(|v| v.as_array()).map(|a| {
                a.iter()
                    .filter_map(|v| v.get("message").and_then(|v| v.as_str()).map(str::to_owned))
                    .collect()
            })
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| vec![github_message(response)])
}
pub(super) fn string(value: &serde_json::Value, key: &str) -> Result<String, GithubError> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or(GithubError::InvalidResponse)
}
pub(super) fn number(value: &serde_json::Value, key: &str) -> Result<u64, GithubError> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .ok_or(GithubError::InvalidResponse)
}
pub(super) fn parse_repo(value: &serde_json::Value) -> Result<RepoInfo, GithubError> {
    let permissions = value
        .get("permissions")
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_bool().map(|v| (k.clone(), v)))
                .collect()
        })
        .unwrap_or_default();
    Ok(RepoInfo {
        default_branch: string(value, "default_branch")?,
        private: value
            .get("private")
            .and_then(serde_json::Value::as_bool)
            .ok_or(GithubError::InvalidResponse)?,
        size_kb: number(value, "size")?,
        clone_url: string(value, "clone_url")?,
        permissions,
    })
}
pub(super) fn parse_pr(v: &serde_json::Value) -> Result<PullRequestInfo, GithubError> {
    Ok(PullRequestInfo {
        number: number(v, "number")?,
        url: string(v, "html_url")?,
        title: string(v, "title")?,
        body: v
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned(),
        state: string(v, "state")?,
        head_sha: v
            .get("head")
            .and_then(|v| v.get("sha"))
            .and_then(|v| v.as_str())
            .ok_or(GithubError::InvalidResponse)?
            .to_owned(),
        head_ref: v
            .get("head")
            .and_then(|v| v.get("ref"))
            .and_then(|v| v.as_str())
            .ok_or(GithubError::InvalidResponse)?
            .to_owned(),
        base: v
            .get("base")
            .and_then(|v| v.get("ref"))
            .and_then(|v| v.as_str())
            .ok_or(GithubError::InvalidResponse)?
            .to_owned(),
    })
}
pub(super) fn issue_comments(
    v: &serde_json::Value,
) -> Result<Vec<PullRequestComment>, GithubError> {
    v.as_array().ok_or(GithubError::InvalidResponse).map(|a| {
        a.iter()
            .map(|v| PullRequestComment {
                author: v
                    .get("user")
                    .and_then(|v| v.get("login"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                body: v
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                path: None,
                line: None,
            })
            .collect()
    })
}
pub(super) fn review_comments(
    v: &serde_json::Value,
) -> Result<Vec<PullRequestComment>, GithubError> {
    v.as_array().ok_or(GithubError::InvalidResponse).map(|a| {
        a.iter()
            .map(|v| PullRequestComment {
                author: v
                    .get("user")
                    .and_then(|v| v.get("login"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                body: v
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                path: v.get("path").and_then(|v| v.as_str()).map(str::to_owned),
                line: v.get("line").and_then(serde_json::Value::as_u64),
            })
            .collect()
    })
}
pub(super) fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        out.push(TABLE[(a >> 2) as usize] as char);
        out.push(TABLE[((a & 3) << 4 | b >> 4) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((b & 15) << 2 | c >> 6) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
