use std::sync::Arc;

use serde_json::Value;
use tether_vcs::{
    GithubError, PullRequestDraft, PushRequest, RefUpdate, Repo, classify_push, parse_github_url,
    push,
};
use tokio_util::sync::CancellationToken;

use super::{GithubClientFactory, TransportFactory, optional_bool, optional_string, string};
use crate::tool_loop::PreparedCall;

pub(super) async fn execute(
    call: &PreparedCall,
    root: &std::path::Path,
    remote_url: Option<&str>,
    default_branch: Option<&str>,
    transport: &Arc<dyn TransportFactory>,
    github: &Arc<dyn GithubClientFactory>,
    cancel: CancellationToken,
) -> Result<String, String> {
    match call.tool.as_str() {
        "git_push" => push_call(call, root, remote_url, default_branch, transport, cancel),
        "pr_create" => create_pr(call, root, remote_url, default_branch, github).await,
        "pr_status" => status_pr(call, root, remote_url, github).await,
        _ => Err("unknown git tool".into()),
    }
}

fn push_call(
    call: &PreparedCall,
    root: &std::path::Path,
    remote_url: Option<&str>,
    default_branch: Option<&str>,
    transport: &Arc<dyn TransportFactory>,
    cancel: CancellationToken,
) -> Result<String, String> {
    let remote_url = remote_url.ok_or("project has no GitHub remote")?;
    let repo = Repo::open(root).map_err(|error| error.to_string())?;
    let branch = optional_string(&call.payload, "branch")?
        .map(str::to_owned)
        .or(repo.current_branch().map_err(|error| error.to_string())?)
        .ok_or("current branch is unavailable")?;
    let force = optional_bool(&call.payload, "force", false)?;
    let delete = optional_bool(&call.payload, "delete", false)?;
    let request = PushRequest {
        remote_url: remote_url.to_owned(),
        updates: vec![RefUpdate {
            src: (!delete).then(|| format!("refs/heads/{branch}")),
            dst: format!("refs/heads/{branch}"),
            force,
        }],
    };
    let _risks = classify_push(&repo, &request, default_branch.unwrap_or("main"));
    let transport = transport.transport()?;
    let vcs_cancel = tether_vcs::CancellationToken::new();
    if cancel.is_cancelled() {
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

async fn create_pr(
    call: &PreparedCall,
    root: &std::path::Path,
    remote_url: Option<&str>,
    default_branch: Option<&str>,
    github: &Arc<dyn GithubClientFactory>,
) -> Result<String, String> {
    let (owner, repo_name) = parse_remote(remote_url)?;
    let repo = Repo::open(root).map_err(|error| error.to_string())?;
    let head = repo
        .current_branch()
        .map_err(|error| error.to_string())?
        .ok_or("current branch is unavailable")?;
    let base = optional_string(&call.payload, "base")?
        .unwrap_or(default_branch.unwrap_or("main"))
        .to_owned();
    let title = string(&call.payload, "title")?.to_owned();
    let body = optional_string(&call.payload, "body")?
        .unwrap_or_default()
        .to_owned();
    let draft = optional_bool(&call.payload, "draft", false)?;
    let client = github.client()?;
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

async fn status_pr(
    call: &PreparedCall,
    root: &std::path::Path,
    remote_url: Option<&str>,
    github: &Arc<dyn GithubClientFactory>,
) -> Result<String, String> {
    let (owner, repo_name) = parse_remote(remote_url)?;
    let repo = Repo::open(root).map_err(|error| error.to_string())?;
    let client = github.client()?;
    let number = if let Some(number) = call.payload.get("number").and_then(Value::as_u64) {
        number
    } else {
        let branch = repo
            .current_branch()
            .map_err(|error| error.to_string())?
            .ok_or("current branch is unavailable")?;
        client
            .list_pull_requests(&owner, &repo_name, "open")
            .await
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|pr| pr.head_sha == branch || pr.base == branch)
            .map(|pr| pr.number)
            .ok_or("pull request not found for the current branch")?
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
    Ok(lines.join("\n"))
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

struct NoProgress;

impl tether_vcs::PushProgress for NoProgress {
    fn update(&self, _phase: tether_vcs::PushPhase, _current: u64, _total: Option<u64>) {}
}
