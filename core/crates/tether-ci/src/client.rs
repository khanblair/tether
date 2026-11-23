use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tether_secrets::Secret;
use tether_vcs::{HttpClient, HttpRequest, HttpResponse};
use thiserror::Error;
mod support;
use support::{JobPage, RunPage, encode, encode_base64, map_response};

const API_URL: &str = "https://api.github.com";
const WORKFLOW: &str = "tether.yml";
const MAX_LOG_BYTES: usize = 8 * 1024 * 1024;

/// Inputs sent to the workflow dispatch endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchInputs {
    /// Shell command.
    pub command: String,
    /// Request identifier included in the run title.
    pub request_id: String,
    /// Optional runtime setup.
    pub setup: Option<String>,
}
/// A GitHub Actions workflow run reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRef {
    /// Numeric run id.
    pub id: u64,
    /// Display title.
    pub display_title: String,
    /// Branch.
    pub branch: String,
    /// Run status.
    pub status: String,
    /// Run conclusion.
    pub conclusion: Option<String>,
    /// Creation time as an RFC3339 string.
    pub created_at: String,
    /// Run page.
    pub html_url: String,
}
/// A GitHub Actions job returned for a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRef {
    /// Numeric job id.
    pub id: u64,
    /// Job name.
    pub name: String,
    /// GitHub job status.
    pub status: String,
    /// GitHub job conclusion.
    pub conclusion: Option<String>,
    /// Job steps.
    pub steps: Vec<JobStep>,
}
/// One step in a workflow job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobStep {
    /// Step name.
    pub name: String,
    /// GitHub step status.
    pub status: String,
    /// GitHub step conclusion.
    pub conclusion: Option<String>,
}
/// A bounded summary for the CI runs screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSummary {
    /// Numeric run id.
    pub id: u64,
    /// GitHub run number.
    pub number: u64,
    /// Command title when it can be read from the display title.
    pub title: String,
    /// Branch.
    pub branch: String,
    /// GitHub status.
    pub status: String,
    /// GitHub conclusion.
    pub conclusion: Option<String>,
    /// Start time.
    pub started_at: Option<String>,
    /// Duration in seconds.
    pub duration_seconds: Option<u64>,
    /// Run page.
    pub html_url: String,
}
/// A workflow is absent from the repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowMissing;
impl fmt::Display for WorkflowMissing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GitHub Actions workflow is missing")
    }
}
impl std::error::Error for WorkflowMissing {}
/// Errors returned by GitHub Actions operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CiError {
    /// Credentials were rejected.
    #[error("GitHub credentials were rejected")]
    BadCredentials,
    /// The request was forbidden.
    #[error("GitHub access was forbidden: {message}")]
    Forbidden {
        /// Safe message returned by GitHub.
        message: String,
    },
    /// The request is rate limited.
    #[error("GitHub rate limit exceeded until {reset_at}")]
    RateLimited {
        /// Unix reset time.
        reset_at: i64,
    },
    /// A requested resource was not found.
    #[error("GitHub Actions resource was not found")]
    NotFound,
    /// The workflow file is not installed.
    #[error(transparent)]
    WorkflowMissing(#[from] WorkflowMissing),
    /// GitHub rejected request fields.
    #[error("GitHub request was invalid: {messages:?}")]
    Validation {
        /// Safe validation messages.
        messages: Vec<String>,
    },
    /// The transport or GitHub server failed.
    #[error("GitHub Actions request failed and can be retried")]
    Retryable,
    /// GitHub returned malformed data.
    #[error("GitHub Actions response was invalid")]
    InvalidResponse,
    /// The request could not be formed.
    #[error("GitHub Actions request is invalid")]
    InvalidRequest,
    /// The run was cancelled by the caller.
    #[error("CI run cancelled")]
    Cancelled,
}
/// A GitHub Actions client with a redacted token.
pub struct CiClient {
    http: Arc<dyn HttpClient>,
    token: Secret,
    repository: Option<(String, String)>,
}
impl fmt::Debug for CiClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CiClient")
    }
}
impl CiClient {
    /// Creates a client over the supplied HTTP seam.
    #[must_use]
    pub fn new(http: Arc<dyn HttpClient>, token: Secret) -> Self {
        Self {
            http,
            token,
            repository: None,
        }
    }
    /// Creates a client bound to one repository for run and job endpoints.
    #[must_use]
    pub fn for_repository(
        http: Arc<dyn HttpClient>,
        token: Secret,
        owner: impl Into<String>,
        repo: impl Into<String>,
    ) -> Self {
        Self {
            http,
            token,
            repository: Some((owner.into(), repo.into())),
        }
    }
    /// Dispatches the Tether workflow on a ref.
    ///
    /// # Errors
    /// Returns a mapped GitHub or transport error.
    pub async fn dispatch(
        &self,
        owner: &str,
        repo: &str,
        git_ref: &str,
        inputs: DispatchInputs,
    ) -> Result<(), CiError> {
        let body = serde_json::json!({"ref": git_ref, "inputs": {"command": inputs.command, "request_id": inputs.request_id, "setup": inputs.setup.unwrap_or_default()}});
        let response = self
            .request(
                "POST",
                &format!("/repos/{owner}/{repo}/actions/workflows/{WORKFLOW}/dispatches"),
                Some(body),
            )
            .await?;
        (response.status == 204)
            .then_some(())
            .ok_or(CiError::InvalidResponse)
    }
    /// Finds a freshly created dispatch run for a request id.
    ///
    /// # Errors
    /// Returns a mapped GitHub or transport error.
    #[expect(
        clippy::too_many_arguments,
        reason = "The GitHub lookup contract keeps repository and request identity explicit."
    )]
    pub async fn find_run(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        request_id: &str,
        not_before: SystemTime,
    ) -> Result<Option<RunRef>, CiError> {
        let path = format!(
            "/repos/{owner}/{repo}/actions/workflows/{WORKFLOW}/runs?event=workflow_dispatch&branch={}&per_page=20",
            encode(branch)
        );
        let response = self.request("GET", &path, None).await?;
        let page: RunPage =
            serde_json::from_slice(&response.body).map_err(|_| CiError::InvalidResponse)?;
        let not_before = not_before
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        Ok(page
            .workflow_runs
            .into_iter()
            .find(|run| {
                run.display_title.contains(request_id)
                    && support::parse_rfc3339(&run.created_at)
                        .is_some_and(|created| created >= not_before)
            })
            .map(RunRef::from))
    }
    /// Reads one workflow run by its numeric id.
    ///
    /// # Errors
    /// Returns a mapped GitHub or transport error.
    pub async fn get_run(&self, run_id: u64) -> Result<RunRef, CiError> {
        let response = self
            .request(
                "GET",
                &self.repo_path(&format!("/actions/runs/{run_id}")),
                None,
            )
            .await?;
        serde_json::from_slice::<support::ApiRun>(&response.body)
            .map(RunRef::from)
            .map_err(|_| CiError::InvalidResponse)
    }
    /// Lists jobs and their steps for a workflow run.
    ///
    /// # Errors
    /// Returns a mapped GitHub or transport error.
    pub async fn run_jobs(&self, run_id: u64) -> Result<Vec<JobRef>, CiError> {
        let response = self
            .request(
                "GET",
                &self.repo_path(&format!("/actions/runs/{run_id}/jobs?per_page=100")),
                None,
            )
            .await?;
        let page: JobPage =
            serde_json::from_slice(&response.body).map_err(|_| CiError::InvalidResponse)?;
        Ok(page.jobs.into_iter().map(JobRef::from).collect())
    }
    /// Cancels a workflow run.
    ///
    /// # Errors
    /// Returns a mapped GitHub or transport error.
    pub async fn cancel_run(&self, run_id: u64) -> Result<(), CiError> {
        self.request(
            "POST",
            &self.repo_path(&format!("/actions/runs/{run_id}/cancel")),
            None,
        )
        .await
        .map(|_| ())
    }
    /// Lists the newest workflow runs, capped at twenty.
    ///
    /// # Errors
    /// Returns a mapped GitHub or transport error.
    pub async fn list_runs(
        &self,
        owner: &str,
        repo: &str,
        limit: usize,
    ) -> Result<Vec<RunSummary>, CiError> {
        let limit = limit.clamp(1, 20);
        let response = self
            .request(
                "GET",
                &format!(
                    "/repos/{owner}/{repo}/actions/workflows/{WORKFLOW}/runs?per_page={limit}"
                ),
                None,
            )
            .await?;
        let page: RunPage =
            serde_json::from_slice(&response.body).map_err(|_| CiError::InvalidResponse)?;
        let mut summaries = page
            .workflow_runs
            .into_iter()
            .map(RunSummary::from)
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| right.started_at.cmp(&left.started_at));
        summaries.truncate(limit);
        Ok(summaries)
    }
    /// Downloads a job log, following a signed redirect without credentials.
    ///
    /// # Errors
    /// Returns a mapped GitHub or transport error.
    pub async fn job_log(&self, job_id: u64) -> Result<String, CiError> {
        let response = self
            .request(
                "GET",
                &self.repo_path(&format!("/actions/jobs/{job_id}/logs")),
                None,
            )
            .await?;
        if response.status != 302 {
            return bounded_log(response.body, &self.token);
        }
        let location = response
            .headers
            .get("location")
            .ok_or(CiError::InvalidResponse)?;
        let redirected = self
            .http
            .send(HttpRequest {
                method: "GET".into(),
                url: location.clone(),
                headers: BTreeMap::new(),
                body: None,
            })
            .await
            .map_err(|_| CiError::Retryable)?;
        bounded_log(redirected.body, &self.token)
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<HttpResponse, CiError> {
        let token = self
            .token
            .expose_text()
            .map_err(|_| CiError::InvalidRequest)?;
        let mut headers = BTreeMap::from([
            ("accept".into(), "application/vnd.github+json".into()),
            ("user-agent".into(), "Tether".into()),
        ]);
        headers.insert(
            "authorization".into(),
            format!(
                "Basic {}",
                encode_base64(format!("x-access-token:{token}").as_bytes())
            ),
        );
        let body = body
            .map(|value| serde_json::to_vec(&value).map_err(|_| CiError::InvalidRequest))
            .transpose()?;
        let response = self
            .http
            .send(HttpRequest {
                method: method.into(),
                url: format!("{API_URL}{path}"),
                headers,
                body,
            })
            .await
            .map_err(|_| CiError::Retryable)?;
        map_response(&response, token)?;
        Ok(response)
    }

    fn repo_path(&self, suffix: &str) -> String {
        self.repository.as_ref().map_or_else(
            || suffix.into(),
            |(owner, repo)| format!("/repos/{owner}/{repo}{suffix}"),
        )
    }
}

fn bounded_log(body: Vec<u8>, token: &Secret) -> Result<String, CiError> {
    if body.len() > MAX_LOG_BYTES {
        return Err(CiError::InvalidResponse);
    }
    let text = String::from_utf8(body).map_err(|_| CiError::InvalidResponse)?;
    let token = token.expose_text().map_err(|_| CiError::InvalidRequest)?;
    Ok(if token.is_empty() {
        text
    } else {
        text.replace(token, "[redacted]")
    })
}
