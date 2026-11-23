use super::{CiRuntime, CiRuntimeError};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tether_ci::{
    CiClient, CiRunState, DispatchInputs, PollConfig, Sleeper, parse_failures, poll_run,
    propose_workflow, trim_log, workflow_is_current,
};
use tether_engine::executors::{CiExecutor, CiRunRequest, ExecutorError, RunReport, RunStatus};
use tether_engine::{CiLogResult, CiLogsRequest, CiRunService};
use tether_secrets::Secret;
use tether_vcs::{
    PushRequest, RefUpdate, Repo, ReqwestHttpClient, ReqwestTransport, parse_github_url, push,
};
use tokio_util::sync::CancellationToken;
impl CiRunService for CiRuntime {
    fn run<'a>(
        &'a self,
        request: CiRunRequest,
        setup: Option<String>,
        _sink: Option<&'a dyn tether_engine::EventSink>,
    ) -> tether_engine::BoxFuture<'a, Result<RunReport, ExecutorError>> {
        Box::pin(async move { self.run_ci(request, setup).await })
    }
    fn logs(
        &self,
        request: CiLogsRequest,
    ) -> tether_engine::BoxFuture<'_, Result<CiLogResult, String>> {
        Box::pin(async move { logs(self, request).await.map_err(|error| error.to_string()) })
    }
}
impl CiExecutor for CiRuntime {
    fn run<'a>(
        &'a self,
        request: CiRunRequest,
        _sink: Option<&'a dyn tether_engine::EventSink>,
    ) -> tether_engine::BoxFuture<'a, Result<RunReport, ExecutorError>> {
        Box::pin(async move { self.run_ci(request, None).await })
    }
}

impl CiRuntime {
    async fn run_ci(
        &self,
        request: CiRunRequest,
        setup: Option<String>,
    ) -> Result<RunReport, ExecutorError> {
        let started = Instant::now();
        let prepared = prepare_run(self, &request)?;
        let PreparedRun {
            root,
            repo,
            branch,
            remote_url,
            owner,
            repo_name,
        } = prepared;
        push_branch(
            repo,
            remote_url,
            branch.clone(),
            self.token().map_err(|error| failed(error.to_string()))?,
        )
        .await?;
        let http =
            Arc::new(ReqwestHttpClient::new().map_err(|_| failed("GitHub HTTP is unavailable"))?);
        let client = CiClient::for_repository(
            http,
            self.token().map_err(|error| failed(error.to_string()))?,
            &owner,
            &repo_name,
        );
        let outcome = dispatch_and_poll(
            CiTarget {
                client: &client,
                owner: &owner,
                repo: &repo_name,
                branch: &branch,
            },
            request.command,
            setup,
            request.cancel,
        )
        .await?;
        let (trimmed, failures) = collect_logs(&client, &outcome, &root).await?;
        let duration = started.elapsed();
        let output = format_result(&outcome, &failures, &trimmed.text, duration);
        let status = run_status(outcome.state);
        Ok(RunReport {
            status,
            output,
            duration,
            output_truncated: trimmed.truncated,
        })
    }
}
struct PreparedRun {
    root: PathBuf,
    repo: Repo,
    branch: String,
    remote_url: String,
    owner: String,
    repo_name: String,
}
fn prepare_run(runtime: &CiRuntime, request: &CiRunRequest) -> Result<PreparedRun, ExecutorError> {
    let project = runtime
        .project(&request.project_id)
        .map_err(|error| failed(error.to_string()))?;
    let root = runtime.root(&project);
    let (repo, branch) = clean_repo(&root)?;
    let remote_url = github_remote(runtime, &project)?;
    let (owner, repo_name) = parse_github_url(&remote_url)
        .ok_or_else(|| failed("project does not have a GitHub remote"))?;
    runtime.token().map_err(|error| failed(error.to_string()))?;
    ensure_workflow(runtime, &project)?;
    Ok(PreparedRun {
        root,
        repo,
        branch,
        remote_url,
        owner,
        repo_name,
    })
}
fn clean_repo(root: &Path) -> Result<(Repo, String), ExecutorError> {
    let repo = Repo::open(root).map_err(|error| failed(error.to_string()))?;
    if !repo
        .status()
        .map_err(|error| failed(error.to_string()))?
        .is_empty()
    {
        return Err(failed("Commit your changes first"));
    }
    let branch = repo
        .current_branch()
        .map_err(|error| failed(error.to_string()))?
        .ok_or_else(|| failed("current branch is unavailable"))?;
    Ok((repo, branch))
}
fn github_remote(
    runtime: &CiRuntime,
    project: &tether_store::ProjectRecord,
) -> Result<String, ExecutorError> {
    runtime
        .store()
        .map_err(|error| failed(error.to_string()))?
        .project_remote(&project.id)
        .map_err(|_| failed("project does not have a GitHub remote"))?
        .and_then(|remote| remote.remote_url)
        .ok_or_else(|| failed("project does not have a GitHub remote"))
}
fn ensure_workflow(
    runtime: &CiRuntime,
    project: &tether_store::ProjectRecord,
) -> Result<(), ExecutorError> {
    let workflow = runtime.workflow_content(project);
    if workflow_is_current(workflow.as_deref()) {
        return Ok(());
    }
    let proposal = propose_workflow(workflow.as_deref())
        .ok_or_else(|| failed("workflow proposal unavailable"))?;
    Err(failed(format!(
        "Workflow is missing or outdated. Add it with write_file, commit it, and call ci_run again.\n{}",
        proposal.content
    )))
}
async fn push_branch(
    repo: Repo,
    remote_url: String,
    branch: String,
    token: Secret,
) -> Result<(), ExecutorError> {
    let transport =
        ReqwestTransport::new(token).map_err(|_| failed("GitHub transport is unavailable"))?;
    let push_request = PushRequest {
        remote_url,
        updates: vec![RefUpdate {
            src: Some(format!("refs/heads/{branch}")),
            dst: format!("refs/heads/{branch}"),
            force: false,
        }],
    };
    tokio::task::spawn_blocking(move || {
        push(
            &repo,
            &push_request,
            &transport,
            &SilentProgress,
            &tether_vcs::CancellationToken::new(),
        )
    })
    .await
    .map_err(|_| failed("push stopped"))?
    .map_err(|error| failed(error.to_string()))
    .map(|_| ())
}
struct CiTarget<'a> {
    client: &'a CiClient,
    owner: &'a str,
    repo: &'a str,
    branch: &'a str,
}
async fn dispatch_and_poll(
    target: CiTarget<'_>,
    command: String,
    setup: Option<String>,
    cancel: CancellationToken,
) -> Result<tether_ci::CiOutcome, ExecutorError> {
    let request_id = format!(
        "tether-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    );
    let not_before = SystemTime::now();
    target
        .client
        .dispatch(
            target.owner,
            target.repo,
            target.branch,
            DispatchInputs {
                command,
                request_id: request_id.clone(),
                setup,
            },
        )
        .await
        .map_err(|error| failed(error.to_string()))?;
    let run = find_run_with_retry(&target, &request_id, not_before).await?;
    poll_run(
        target.client,
        run,
        PollConfig::default(),
        &TokioSleeper,
        cancel,
        |_| {},
    )
    .await
    .map_err(|error| failed(error.to_string()))
}
async fn find_run_with_retry(
    target: &CiTarget<'_>,
    request_id: &str,
    not_before: SystemTime,
) -> Result<tether_ci::RunRef, ExecutorError> {
    for _ in 0..60 {
        let found = target
            .client
            .find_run(
                target.owner,
                target.repo,
                target.branch,
                request_id,
                not_before,
            )
            .await
            .map_err(|error| failed(error.to_string()))?;
        let Some(run) = found else {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        };
        return Ok(run);
    }
    Err(failed("GitHub Actions run was not found"))
}
async fn collect_logs(
    client: &CiClient,
    outcome: &tether_ci::CiOutcome,
    root: &Path,
) -> Result<(tether_ci::TrimmedLog, Vec<tether_ci::CiFailure>), ExecutorError> {
    let jobs = client
        .run_jobs(outcome.run.id)
        .await
        .map_err(|error| failed(error.to_string()))?;
    let failed_jobs = jobs
        .iter()
        .filter(|job| job.conclusion.as_deref() != Some("success"))
        .collect::<Vec<_>>();
    let jobs_to_read = if failed_jobs.is_empty() {
        jobs.iter().collect::<Vec<_>>()
    } else {
        failed_jobs
    };
    let mut raw = String::new();
    for job in jobs_to_read {
        let Ok(log) = client.job_log(job.id).await else {
            continue;
        };
        raw.push_str(&log);
        raw.push('\n');
    }
    let trimmed = trim_log(&raw, 12_000);
    let failures = parse_failures(&trimmed.text, &root.to_string_lossy());
    Ok((trimmed, failures))
}
pub(crate) async fn logs(
    runtime: &CiRuntime,
    request: CiLogsRequest,
) -> Result<CiLogResult, CiRuntimeError> {
    let project = runtime.project(&request.project_id)?;
    let (client, owner, repo) = runtime.client(&project)?;
    let run_id = if let Some(id) = request.run_id {
        id
    } else {
        let branch = Repo::open(runtime.root(&project))
            .map_err(|_| CiRuntimeError::Repository)?
            .current_branch()
            .map_err(|_| CiRuntimeError::Repository)?
            .ok_or(CiRuntimeError::Repository)?;
        client
            .list_runs(&owner, &repo, 20)
            .await
            .map_err(|error| map_error(&error))?
            .into_iter()
            .find(|run| run.branch == branch)
            .map(|run| run.id)
            .ok_or(CiRuntimeError::Repository)?
    };
    let jobs = client
        .run_jobs(run_id)
        .await
        .map_err(|error| map_error(&error))?;
    let mut raw = String::new();
    for job in jobs
        .into_iter()
        .filter(|job| request.job.as_deref().is_none_or(|name| name == job.name))
    {
        raw.push_str(
            &client
                .job_log(job.id)
                .await
                .map_err(|error| map_error(&error))?,
        );
        raw.push('\n');
    }
    let trimmed = trim_log(&raw, 12_000);
    Ok(CiLogResult {
        failures: parse_failures(&trimmed.text, &runtime.root(&project).to_string_lossy()),
        text: trimmed.text,
    })
}
struct SilentProgress;
impl tether_vcs::PushProgress for SilentProgress {
    fn update(&self, _phase: tether_vcs::PushPhase, _current: u64, _total: Option<u64>) {}
}
struct TokioSleeper;
impl Sleeper for TokioSleeper {
    fn sleep<'a>(
        &'a self,
        duration: Duration,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(tokio::time::sleep(duration))
    }
}
fn failed(error: impl Into<String>) -> ExecutorError {
    ExecutorError::Failed(error.into())
}
fn map_error(error: &tether_ci::CiError) -> CiRuntimeError {
    CiRuntimeError::Github {
        detail: error.to_string(),
    }
}
fn format_result(
    outcome: &tether_ci::CiOutcome,
    failures: &[tether_ci::CiFailure],
    log: &str,
    duration: Duration,
) -> String {
    let conclusion = match outcome.state {
        CiRunState::Succeeded => "success",
        CiRunState::Cancelled => "cancelled",
        CiRunState::TimedOut => "timed_out",
        _ => "failure",
    };
    let mut lines = vec![
        format!("run: {}", outcome.run.html_url),
        format!("conclusion: {conclusion}"),
        format!("duration: {}s", duration.as_secs()),
    ];
    lines.extend(failures.iter().map(|failure| {
        format!(
            "{}:{} {}",
            failure.file.as_deref().unwrap_or("unknown"),
            failure.line.unwrap_or_default(),
            failure.message
        )
    }));
    lines.push(log.into());
    lines.join("\n")
}

fn run_status(state: CiRunState) -> RunStatus {
    match state {
        CiRunState::Succeeded => RunStatus::Exited(0),
        CiRunState::Cancelled => RunStatus::Cancelled,
        CiRunState::TimedOut => RunStatus::TimedOut,
        _ => RunStatus::Exited(1),
    }
}
