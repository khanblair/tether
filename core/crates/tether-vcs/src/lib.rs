//! Git and GitHub support.

mod clone;
mod github;
mod oauth;
mod pat;
mod push;

#[cfg(test)]
mod clone_tests;
#[cfg(test)]
mod github_tests;
#[cfg(test)]
mod push_tests;
mod repo;
#[cfg(test)]
mod repo_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod transport_tests;

pub use clone::{CloneError, ClonePhase, CloneProgress, CloneRequest, clone};
pub use github::{
    CheckRun, GithubClient, GithubError, GithubIdentity, HttpClient, HttpClientError, HttpRequest,
    HttpResponse, PullRequestComment, PullRequestDraft, PullRequestInfo, RepoInfo,
    ReqwestHttpClient, parse_github_url, should_clone_shallow,
};
pub use oauth::{DeviceGrant, GithubDeviceAuth, OAuthError, PollState};
pub use pat::{GithubPatAuth, PatError, PatInfo};
pub use push::{
    PushError, PushPhase, PushProgress, PushReport, PushRequest, PushRisk, RefResult, RefUpdate,
    ReqwestTransport, Transport, TransportError, classify_push, push,
};
pub use repo::{
    BranchInfo, CancellationToken, CommitId, CommitInfo, DiffSpec, FileState, Identity, Repo,
    RepoError, StatusEntry, branch_name_for_task, unique_branch_name, validate_branch_name,
};
