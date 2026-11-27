//! Factories and helpers that connect the Git runtime to the engine's Git tools.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tether_engine::{GitIdentityReader, GitRemoteWriter, GithubClientFactory, TransportFactory};
use tether_vcs::{
    BranchInfo, CommitInfo, Identity, RepoError, ReqwestTransport, StatusEntry, Transport,
};

use crate::git::GitRuntime;
use crate::git_types::{GitBranchView, GitCommitView, GitRuntimeError, GitStatusView};

pub(crate) struct RuntimeIdentity(pub(crate) Arc<GitRuntime>);
impl GitIdentityReader for RuntimeIdentity {
    fn identity(&self) -> Result<Option<Identity>, String> {
        self.0
            .store()
            .map_err(|_| "identity is unavailable".to_owned())?
            .git_identity()
            .map_err(|_| "identity is unavailable".to_owned())
            .map(|value| {
                value.map(|identity| Identity {
                    name: identity.name,
                    email: identity.email,
                })
            })
    }
}

pub(crate) struct RuntimeTransport(pub(crate) Arc<GitRuntime>);
impl TransportFactory for RuntimeTransport {
    fn transport(&self) -> Result<Box<dyn Transport>, String> {
        let token = self.0.token()?;
        let transport = ReqwestTransport::new(token)
            .map_err(|_| "GitHub transport is unavailable".to_owned())?;
        Ok(Box::new(transport))
    }
}

pub(crate) struct RuntimeGithub(pub(crate) Arc<GitRuntime>);
impl GithubClientFactory for RuntimeGithub {
    fn client(&self) -> Result<tether_vcs::GithubClient, String> {
        self.0.github_client().map_err(|error| error.to_string())
    }
}

pub(crate) struct RuntimeRemote {
    pub(crate) runtime: Arc<GitRuntime>,
    pub(crate) project_id: tether_types::ProjectId,
}

impl GitRemoteWriter for RuntimeRemote {
    fn set_remote(&self, remote_url: &str, default_branch: &str) -> Result<(), String> {
        self.runtime
            .store()
            .map_err(|_| "project store is unavailable".to_owned())?
            .set_project_remote(&self.project_id, remote_url, default_branch)
            .map_err(|_| "project remote could not be saved".to_owned())
    }
}

pub(crate) fn validate_identity_field(value: &str) -> Result<(), GitRuntimeError> {
    if value.trim().is_empty()
        || value
            .chars()
            .any(|ch| ch == '<' || ch == '>' || ch == '\n' || ch == '\r')
    {
        Err(GitRuntimeError::InvalidInput)
    } else {
        Ok(())
    }
}

pub(crate) fn status_view(status: StatusEntry) -> GitStatusView {
    GitStatusView {
        path: status.path,
        index_state: status
            .index_state
            .map_or_else(String::new, |state| format!("{state:?}")),
        worktree_state: status
            .worktree_state
            .map_or_else(String::new, |state| format!("{state:?}")),
    }
}

pub(crate) fn commit_view(commit: CommitInfo) -> GitCommitView {
    GitCommitView {
        id: commit.id,
        subject: commit.subject,
        author_name: commit.author_name,
        author_email: commit.author_email,
        committer_time: commit.committer_time,
    }
}

pub(crate) fn branch_view(branch: BranchInfo) -> GitBranchView {
    GitBranchView {
        name: branch.name,
        commit: branch.commit,
    }
}

pub(crate) fn repo_error(error: &RepoError) -> GitRuntimeError {
    GitRuntimeError::Repository {
        detail: error.to_string(),
    }
}

pub(crate) fn cap_diff(mut diff: String) -> String {
    const LIMIT: usize = 64 * 1024;
    if diff.len() <= LIMIT {
        return diff;
    }
    let mut end = LIMIT;
    while !diff.is_char_boundary(end) {
        end -= 1;
    }
    diff.truncate(end);
    diff.push_str("\n[diff truncated at 64 KiB]\n");
    diff
}

pub(crate) fn filter_diff_path(text: &str, path: &str) -> String {
    let marker = format!("diff --git a/{path} b/{path}");
    let mut output = String::new();
    for chunk in text.split("diff --git ") {
        if chunk.starts_with(marker.trim_start_matches("diff --git ")) {
            output.push_str("diff --git ");
            output.push_str(chunk);
        }
    }
    cap_diff(output)
}

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}
