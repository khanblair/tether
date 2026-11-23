use std::sync::Arc;

use serde_json::Value;
use tether_vcs::{
    CheckRun, GithubError, PullRequestComment, PullRequestDraft, PullRequestInfo, PushRequest,
    RefUpdate, Repo, classify_push, parse_github_url, push,
};
use tokio_util::sync::CancellationToken;

use super::{GithubClientFactory, TransportFactory, optional_bool, optional_string, string};
use crate::tool_loop::PreparedCall;

/// Project settings and factories that the remote actions need.
pub(super) struct RemoteSetup<'a> {
    /// Workspace root that holds the repository.
    pub(super) root: &'a std::path::Path,
    /// Saved project remote URL, if any.
    pub(super) remote_url: Option<&'a str>,
    /// Saved project default branch, if any.
    pub(super) default_branch: Option<&'a str>,
    /// Creates push transports.
    pub(super) transport: &'a Arc<dyn TransportFactory>,
    /// Creates GitHub clients.
    pub(super) github: &'a Arc<dyn GithubClientFactory>,
}

pub(super) async fn execute(
    call: &PreparedCall,
    setup: &RemoteSetup<'_>,
    cancel: &CancellationToken,
) -> Result<String, String> {
    match call.tool.as_str() {
        "git_push" => push_blocking(call, setup, cancel).await,
        "pr_create" => create_pr(call, setup).await,
        "pr_status" => status_pr(call, setup).await,
        _ => Err("unknown git tool".into()),
    }
}

/// Everything a push needs, owned so it can move to a blocking thread.
struct PushJob {
    call: PreparedCall,
    root: std::path::PathBuf,
    remote_url: Option<String>,
    default_branch: Option<String>,
    transport: Arc<dyn TransportFactory>,
    cancel: CancellationToken,
}

/// Runs the push on a blocking thread: the production transport blocks on its own runtime,
/// which must not happen on a thread that drives async tasks.
async fn push_blocking(
    call: &PreparedCall,
    setup: &RemoteSetup<'_>,
    cancel: &CancellationToken,
) -> Result<String, String> {
    let job = PushJob {
        call: call.clone(),
        root: setup.root.to_owned(),
        remote_url: setup.remote_url.map(str::to_owned),
        default_branch: setup.default_branch.map(str::to_owned),
        transport: Arc::clone(setup.transport),
        cancel: cancel.clone(),
    };
    tokio::task::spawn_blocking(move || push_call(&job))
        .await
        .map_err(|_| "push stopped".to_owned())?
}

fn push_call(job: &PushJob) -> Result<String, String> {
    let remote_url = job
        .remote_url
        .as_deref()
        .ok_or("project has no GitHub remote")?;
    let repo = Repo::open(&job.root).map_err(|error| error.to_string())?;
    // Preparation resolved and froze the branch, so this is the branch the user approved.
    let branch = string(&job.call.payload, "branch")?.to_owned();
    let force = optional_bool(&job.call.payload, "force", false)?;
    let delete = optional_bool(&job.call.payload, "delete", false)?;
    let request = PushRequest {
        remote_url: remote_url.to_owned(),
        updates: vec![RefUpdate {
            src: (!delete).then(|| format!("refs/heads/{branch}")),
            dst: format!("refs/heads/{branch}"),
            force,
        }],
    };
    let _risks = classify_push(
        &repo,
        &request,
        job.default_branch.as_deref().unwrap_or("main"),
    );
    let transport = job.transport.transport()?;
    let vcs_cancel = tether_vcs::CancellationToken::new();
    if job.cancel.is_cancelled() {
        vcs_cancel.cancel();
    }
    let report = push(
        &repo,
        &request,
        transport.as_ref(),
        &NoProgress,
        &vcs_cancel,
    )
    .map_err(|error| error.to_string())?;
    Ok(report
        .updates
        .into_iter()
        .map(|update| {
            format!(
                "{}: {}",
                update.dst,
                if update.accepted {
                    "accepted"
                } else {
                    "rejected"
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

async fn create_pr(call: &PreparedCall, setup: &RemoteSetup<'_>) -> Result<String, String> {
    let (owner, repo_name) = parse_remote(setup.remote_url)?;
    // Preparation resolved and froze the head branch that the approval named.
    let head = string(&call.payload, "head")?.to_owned();
    let base = optional_string(&call.payload, "base")?
        .unwrap_or(setup.default_branch.unwrap_or("main"))
        .to_owned();
    let title = string(&call.payload, "title")?.to_owned();
    let body = optional_string(&call.payload, "body")?
        .unwrap_or_default()
        .to_owned();
    let draft = optional_bool(&call.payload, "draft", false)?;
    let client = setup.github.client()?;
    let pr = client
        .create_pull_request(
            &owner,
            &repo_name,
            PullRequestDraft {
                head,
                base,
                title,
                body,
                draft,
            },
        )
        .await
        .map_err(create_error)?;
    Ok(format!("PR #{}: {}", pr.number, pr.url))
}

async fn status_pr(call: &PreparedCall, setup: &RemoteSetup<'_>) -> Result<String, String> {
    let (owner, repo_name) = parse_remote(setup.remote_url)?;
    let repo = Repo::open(setup.root).map_err(|error| error.to_string())?;
    let client = setup.github.client()?;
    let number = if let Some(number) = call.payload.get("number").and_then(Value::as_u64) {
        number
    } else {
        let branch = current_branch(&repo)?;
        let open = client
            .list_pull_requests(&owner, &repo_name, "open")
            .await
            .map_err(|error| error.to_string())?;
        find_pr_number(&open, &branch)?
    };
    let pr = client
        .get_pull_request(&owner, &repo_name, number)
        .await
        .map_err(|error| error.to_string())?;
    let checks = client
        .pull_request_checks(&owner, &repo_name, &pr.head_sha)
        .await
        .map_err(|error| error.to_string())?;
    let comments = client
        .pull_request_comments(&owner, &repo_name, number)
        .await
        .map_err(|error| error.to_string())?;
    Ok(status_text(&pr, checks, comments))
}

/// Finds the number of the open pull request whose head branch is `branch`.
pub(super) fn find_pr_number(open: &[PullRequestInfo], branch: &str) -> Result<u64, String> {
    open.iter()
        .find(|pr| pr.head_ref == branch)
        .map(|pr| pr.number)
        .ok_or_else(|| "pull request not found for the current branch".to_owned())
}

/// Formats the pull request header, its checks, and its first comments.
fn status_text(
    pr: &PullRequestInfo,
    checks: Vec<CheckRun>,
    comments: Vec<PullRequestComment>,
) -> String {
    let mut lines = vec![format!("PR #{}: {}", pr.number, pr.url)];
    lines.extend(checks.into_iter().map(|check| {
        format!(
            "check {}: {} ({})",
            check.name,
            check.status,
            check.conclusion.unwrap_or_else(|| "pending".into())
        )
    }));
    lines.extend(comments.into_iter().take(30).map(|comment| {
        format!(
            "comment {}: {}{}",
            comment.author,
            comment.body.chars().take(500).collect::<String>(),
            comment.path.map_or(String::new(), |path| format!(
                " at {path}:{}",
                comment.line.unwrap_or_default()
            ))
        )
    }));
    lines.join("\n")
}

fn current_branch(repo: &Repo) -> Result<String, String> {
    repo.current_branch()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "current branch is unavailable".to_owned())
}

fn parse_remote(remote_url: Option<&str>) -> Result<(String, String), String> {
    remote_url
        .and_then(parse_github_url)
        .ok_or_else(|| "project does not have a GitHub remote".to_owned())
}

fn create_error(error: GithubError) -> String {
    match error {
        GithubError::Validation { messages }
            if messages
                .iter()
                .any(|message| message.contains("head") && message.contains("does not exist")) =>
        {
            "Push the branch first".into()
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
pub(super) fn test_status_text(
    pr: &PullRequestInfo,
    checks: Vec<CheckRun>,
    comments: Vec<PullRequestComment>,
) -> String {
    status_text(pr, checks, comments)
}

#[cfg(test)]
pub(super) fn test_create_error(error: GithubError) -> String {
    create_error(error)
}

struct NoProgress;

impl tether_vcs::PushProgress for NoProgress {
    fn update(&self, _phase: tether_vcs::PushPhase, _current: u64, _total: Option<u64>) {}
}
