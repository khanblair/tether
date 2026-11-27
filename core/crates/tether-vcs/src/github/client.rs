use super::helpers::{
    base64_encode, issue_comments, map_status, number, parse_created_repo, parse_pr, parse_repo,
    review_comments, string,
};
use super::{
    API_URL, BTreeMap, CheckRun, CreatedRepository, GithubClient, GithubError, GithubIdentity,
    HttpClient, HttpRequest, MAX_BODY, PullRequestComment, PullRequestDraft, PullRequestInfo,
    RepoInfo, Secret,
};
use std::sync::Arc;
impl GithubClient {
    /// Creates a client using GitHub's production API.
    #[must_use]
    pub fn new(http: Arc<dyn HttpClient>, token: Secret) -> Self {
        Self {
            http,
            token,
            base_url: API_URL.to_owned(),
        }
    }

    /// Creates a client against an HTTPS test endpoint.
    ///
    /// # Errors
    /// Returns an error when `base_url` is not HTTPS.
    pub fn with_base_url(
        http: Arc<dyn HttpClient>,
        token: Secret,
        base_url: &str,
    ) -> Result<Self, GithubError> {
        let parsed = reqwest::Url::parse(base_url).map_err(|_| GithubError::InvalidRequest)?;
        if parsed.scheme() != "https" {
            return Err(GithubError::InvalidRequest);
        }
        Ok(Self {
            http,
            token,
            base_url: base_url.trim_end_matches('/').to_owned(),
        })
    }

    /// Gets the authenticated user and resolves a primary email when needed.
    ///
    /// # Errors
    /// Returns a typed GitHub error.
    pub async fn current_user(&self) -> Result<GithubIdentity, GithubError> {
        let user = self.get("/user").await?;
        let login = string(&user, "login")?;
        let id = number(&user, "id")?;
        let name = user
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
            .unwrap_or(&login)
            .to_owned();
        let email = match user
            .get("email")
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
        {
            Some(email) => email.to_owned(),
            None => self
                .get_primary_email()
                .await
                .unwrap_or_else(|| format!("{id}+{login}@users.noreply.github.com")),
        };
        Ok(GithubIdentity {
            login,
            id,
            name,
            email,
        })
    }

    /// Gets repository metadata.
    ///
    /// # Errors
    /// Returns a typed GitHub error.
    pub async fn get_repo(&self, owner: &str, repo: &str) -> Result<RepoInfo, GithubError> {
        parse_repo(&self.get(&format!("/repos/{owner}/{repo}")).await?)
    }

    /// Lists repositories for the authenticated user.
    ///
    /// # Errors
    /// Returns a typed GitHub error.
    pub async fn list_user_repos(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<RepoInfo>, GithubError> {
        let value = self
            .get(&format!("/user/repos?page={page}&per_page={per_page}"))
            .await?;
        value
            .as_array()
            .ok_or(GithubError::InvalidResponse)?
            .iter()
            .map(parse_repo)
            .collect()
    }

    /// Creates a repository for the authenticated user.
    ///
    /// # Errors
    /// Returns a typed GitHub error.
    pub async fn create_repository(
        &self,
        name: &str,
        private: bool,
        description: Option<&str>,
    ) -> Result<CreatedRepository, GithubError> {
        let body = serde_json::json!({
            "name": name,
            "private": private,
            "description": description.unwrap_or_default(),
        });
        parse_created_repo(&self.post("/user/repos", body).await?)
    }

    /// Creates a pull request.
    ///
    /// # Errors
    /// Returns a typed GitHub error.
    pub async fn create_pull_request(
        &self,
        owner: &str,
        repo: &str,
        draft: PullRequestDraft,
    ) -> Result<PullRequestInfo, GithubError> {
        let body = serde_json::json!({"head": draft.head, "base": draft.base, "title": draft.title, "body": draft.body, "draft": draft.draft});
        parse_pr(
            &self
                .post(&format!("/repos/{owner}/{repo}/pulls"), body)
                .await?,
        )
    }

    /// Gets a pull request.
    ///
    /// # Errors
    /// Returns a typed GitHub error.
    pub async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Result<PullRequestInfo, GithubError> {
        parse_pr(
            &self
                .get(&format!("/repos/{owner}/{repo}/pulls/{number}"))
                .await?,
        )
    }

    /// Lists pull requests in a state.
    ///
    /// # Errors
    /// Returns a typed GitHub error.
    pub async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
    ) -> Result<Vec<PullRequestInfo>, GithubError> {
        let value = self
            .get(&format!(
                "/repos/{owner}/{repo}/pulls?state={state}&per_page=100"
            ))
            .await?;
        value
            .as_array()
            .ok_or(GithubError::InvalidResponse)?
            .iter()
            .map(parse_pr)
            .collect()
    }

    /// Lists check runs for a commit.
    ///
    /// # Errors
    /// Returns a typed GitHub error.
    pub async fn pull_request_checks(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<Vec<CheckRun>, GithubError> {
        let value = self
            .get(&format!("/repos/{owner}/{repo}/commits/{sha}/check-runs"))
            .await?;
        let runs = value
            .get("check_runs")
            .and_then(|v| v.as_array())
            .ok_or(GithubError::InvalidResponse)?;
        runs.iter()
            .map(|v| {
                Ok(CheckRun {
                    name: string(v, "name")?,
                    status: string(v, "status")?,
                    conclusion: v
                        .get("conclusion")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned),
                    details_url: v
                        .get("details_url")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned),
                })
            })
            .collect()
    }

    /// Merges issue and review comments in oldest-first order.
    ///
    /// # Errors
    /// Returns a typed GitHub error.
    pub async fn pull_request_comments(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Result<Vec<PullRequestComment>, GithubError> {
        let issue = self
            .get(&format!("/repos/{owner}/{repo}/issues/{number}/comments"))
            .await?;
        let review = self
            .get(&format!("/repos/{owner}/{repo}/pulls/{number}/comments"))
            .await?;
        let mut out = issue_comments(&issue)?;
        out.extend(review_comments(&review)?);
        Ok(out)
    }

    async fn get(&self, path: &str) -> Result<serde_json::Value, GithubError> {
        self.request("GET", path, None).await
    }
    async fn post(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, GithubError> {
        self.request("POST", path, Some(body)).await
    }
    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, GithubError> {
        let token = self
            .token
            .expose_text()
            .map_err(|_| GithubError::InvalidRequest)?;
        let mut headers = BTreeMap::from([
            (
                String::from("accept"),
                String::from("application/vnd.github+json"),
            ),
            (String::from("user-agent"), String::from("Tether")),
        ]);
        headers.insert(
            String::from("authorization"),
            format!(
                "Basic {}",
                base64_encode(format!("x-access-token:{token}").as_bytes())
            ),
        );
        let body = body
            .map(|value| serde_json::to_vec(&value).map_err(|_| GithubError::InvalidRequest))
            .transpose()?;
        let response = self
            .http
            .send(HttpRequest {
                method: method.to_owned(),
                url: format!("{}{}", self.base_url, path),
                headers,
                body,
            })
            .await
            .map_err(|_| GithubError::Retryable)?;
        map_status(&response)?;
        if response.body.len() > MAX_BODY {
            return Err(GithubError::InvalidResponse);
        }
        serde_json::from_slice(&response.body).map_err(|_| GithubError::InvalidResponse)
    }
    async fn get_primary_email(&self) -> Option<String> {
        let value = self.get("/user/emails").await.ok()?;
        value
            .as_array()?
            .iter()
            .find(|v| {
                v.get("primary").and_then(serde_json::Value::as_bool) == Some(true)
                    && v.get("verified").and_then(serde_json::Value::as_bool) == Some(true)
            })
            .and_then(|v| v.get("email"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    }
}
