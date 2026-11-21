//! UniFFI runtime for local Git repositories and clone operations.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_engine::{
    GitIdentityReader, GitTools, GitToolsConfig, GithubClientFactory, ToolHandler, TransportFactory,
};
use tether_secrets::{Secret, SecretRef};
use tether_store::{GitIdentity, Store};
use tether_types::ProjectId;
use tether_vcs::{
    BranchInfo, CloneProgress, CloneRequest, CommitInfo, DiffSpec, Identity, Repo, RepoError,
    ReqwestHttpClient, ReqwestTransport, StatusEntry, Transport, parse_github_url,
    should_clone_shallow,
};

use crate::SecretVault;
use crate::github::{DeviceGrantState, GithubRepoView};

/// A choice of clone history depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ShallowChoiceView {
    /// Select shallow or full from repository size.
    Auto,
    /// Clone with one history layer.
    Shallow,
    /// Clone complete history.
    Full,
}

/// One compact Git status row.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitStatusView {
    /// Repository-relative path.
    pub path: String,
    /// Index state, or empty when unchanged.
    pub index_state: String,
    /// Worktree state, or empty when unchanged.
    pub worktree_state: String,
}

/// One commit summary.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitCommitView {
    /// Full commit id.
    pub id: String,
    /// First message line.
    pub subject: String,
    /// Author name.
    pub author_name: String,
    /// Author email.
    pub author_email: String,
    /// Commit time in Unix seconds.
    pub committer_time: i64,
}

/// One local branch.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitBranchView {
    /// Branch name.
    pub name: String,
    /// Commit id, or empty for an unborn branch.
    pub commit: Option<String>,
}

/// A diff request for a project repository.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GitDiffSpecView {
    /// Compare the index with `HEAD`.
    pub staged: bool,
    /// Optional repository-relative path.
    pub path: Option<String>,
    /// Older revision, paired with `to`.
    pub from: Option<String>,
    /// Newer revision, paired with `from`.
    pub to: Option<String>,
}

/// Receives clone progress on a platform thread.
#[uniffi::export(with_foreign)]
pub trait CloneProgressView: Send + Sync {
    /// Reports a clone phase and optional byte counts.
    fn on_progress(&self, phase: String, received: u64, total: Option<u64>);
}

/// A Git or GitHub runtime failure.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum GitRuntimeError {
    /// A required project or repository was absent.
    #[error("project or repository not found")]
    NotFound,
    /// The input was invalid.
    #[error("invalid Git input")]
    InvalidInput,
    /// Storage or local runtime setup failed.
    #[error("Git runtime unavailable")]
    Unavailable,
    /// GitHub sign-in was not configured.
    #[error("GitHub sign-in is not configured")]
    GithubNotConfigured,
    /// No device grant is active.
    #[error("GitHub device sign-in is not active")]
    NoDeviceGrant,
    /// GitHub rejected credentials.
    #[error("GitHub credentials were rejected")]
    GithubCredentials,
    /// GitHub returned a malformed response.
    #[error("GitHub response was invalid")]
    GithubResponse,
    /// GitHub could not be reached.
    #[error("GitHub network request failed")]
    GithubNetwork,
    /// The token is missing a required scope.
    #[error("GitHub token is missing scope {scope}")]
    MissingScope { scope: String },
    /// The Git repository operation failed.
    #[error("Git repository operation failed: {detail}")]
    Repository { detail: String },
}

/// Owns GitHub credentials, project metadata, and repository operations.
#[derive(uniffi::Object)]
pub struct GitRuntime {
    pub(crate) database_path: String,
    pub(crate) workspaces_root: String,
    pub(crate) vault: Arc<SecretVault>,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
    pub(crate) oauth_client_id: Mutex<Option<String>>,
    pub(crate) device_grant: Mutex<Option<DeviceGrantState>>,
}

#[uniffi::export]
impl GitRuntime {
    pub(crate) fn tool_handler(
        runtime: Arc<Self>,
        remote_url: Option<String>,
        default_branch: Option<String>,
    ) -> Arc<dyn ToolHandler> {
        Arc::new(GitTools::new(GitToolsConfig {
            remote_url,
            default_branch,
            identity: Arc::new(RuntimeIdentity(Arc::clone(&runtime))),
            transport: Arc::new(RuntimeTransport(Arc::clone(&runtime))),
            github: Arc::new(RuntimeGithub(runtime)),
            gates: Vec::new(),
        }))
    }

    /// Opens the Git runtime over the app database, workspace root, and secret vault.
    ///
    /// # Errors
    /// Returns an error when the database or blocking runtime cannot start.
    #[uniffi::constructor]
    pub fn new(
        database_path: &str,
        workspaces_root: &str,
        vault: Arc<SecretVault>,
    ) -> Result<Arc<Self>, GitRuntimeError> {
        Store::open(Path::new(database_path)).map_err(|_| GitRuntimeError::Unavailable)?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|_| GitRuntimeError::Unavailable)?;
        Ok(Arc::new(Self {
            database_path: database_path.into(),
            workspaces_root: workspaces_root.into(),
            vault,
            runtime: Arc::new(runtime),
            oauth_client_id: Mutex::new(None),
            device_grant: Mutex::new(None),
        }))
    }

    /// Sets a manual Git commit identity.
    ///
    /// # Errors
    /// Returns an error when either field is empty or contains a forbidden character.
    pub fn set_git_identity(&self, name: String, email: String) -> Result<(), GitRuntimeError> {
        validate_identity_field(&name)?;
        validate_identity_field(&email)?;
        self.store()?
            .set_git_identity(&GitIdentity {
                login: None,
                name,
                email,
                source: "manual".into(),
                updated_at: now_ms(),
            })
            .map_err(|_| GitRuntimeError::Unavailable)
    }

    /// Reads repository status for one project.
    ///
    /// # Errors
    /// Returns an error when the project or repository cannot be read.
    pub fn git_status(&self, project_id: String) -> Result<Vec<GitStatusView>, GitRuntimeError> {
        let repo = self.repo(&project_id)?;
        repo.status()
            .map(|rows| rows.into_iter().map(status_view).collect())
            .map_err(repo_error)
    }

    /// Reads a bounded repository diff for one project.
    ///
    /// # Errors
    /// Returns an error when the project or revision cannot be read.
    pub fn git_diff(
        &self,
        project_id: String,
        spec: GitDiffSpecView,
    ) -> Result<String, GitRuntimeError> {
        let repo = self.repo(&project_id)?;
        let diff = match (spec.from, spec.to) {
            (Some(from), Some(to)) => repo.diff(DiffSpec::Revisions { old: from, new: to }),
            (None, None) if spec.staged => repo.diff(DiffSpec::Staged),
            (None, None) => repo.diff(DiffSpec::Worktree),
            _ => return Err(GitRuntimeError::InvalidInput),
        }?;
        Ok(cap_diff(diff))
    }

    /// Reads recent commits for one project.
    ///
    /// # Errors
    /// Returns an error when the project or history cannot be read.
    pub fn git_log(
        &self,
        project_id: String,
        limit: u32,
    ) -> Result<Vec<GitCommitView>, GitRuntimeError> {
        if !(1..=50).contains(&limit) {
            return Err(GitRuntimeError::InvalidInput);
        }
        self.repo(&project_id)?
            .log(limit as usize, None)
            .map(|rows| rows.into_iter().map(commit_view).collect())
            .map_err(repo_error)
    }

    /// Lists local branches for one project.
    ///
    /// # Errors
    /// Returns an error when the project or repository cannot be read.
    pub fn git_branches(&self, project_id: String) -> Result<Vec<GitBranchView>, GitRuntimeError> {
        self.repo(&project_id)?
            .branches()
            .map(|rows| rows.into_iter().map(branch_view).collect())
            .map_err(repo_error)
    }

    /// Returns the current branch for one project.
    ///
    /// # Errors
    /// Returns an error when the project or repository cannot be read.
    pub fn current_branch(&self, project_id: String) -> Result<Option<String>, GitRuntimeError> {
        self.repo(&project_id)?.current_branch().map_err(repo_error)
    }

    /// Initializes a repository in an empty project folder.
    ///
    /// # Errors
    /// Returns an error when the project is absent, non-empty, or the branch is invalid.
    pub fn git_init(
        &self,
        project_id: String,
        default_branch: String,
    ) -> Result<(), GitRuntimeError> {
        let path = self.project_root(&project_id)?;
        Repo::init(path, &default_branch)
            .map(|_| ())
            .map_err(repo_error)
    }

    /// Lists repositories for the authenticated GitHub account.
    ///
    /// # Errors
    /// Returns an error when GitHub is not signed in or the request fails.
    pub fn list_github_repos(&self, page: u32) -> Result<Vec<GithubRepoView>, GitRuntimeError> {
        if page == 0 {
            return Err(GitRuntimeError::InvalidInput);
        }
        let client = RuntimeGithub(Arc::new(self.clone_for_factory()?))
            .client()
            .map_err(|_| GitRuntimeError::GithubNetwork)?;
        self.runtime
            .block_on(client.list_user_repos(page, 30))
            .map(|repos| {
                repos
                    .into_iter()
                    .map(|repo| GithubRepoView {
                        clone_url: repo.clone_url,
                        default_branch: repo.default_branch,
                        private: repo.private,
                        size_kb: repo.size_kb,
                    })
                    .collect()
            })
            .map_err(|_| GitRuntimeError::GithubResponse)
    }

    /// Clones a repository into a new project workspace.
    ///
    /// # Errors
    /// Returns an error when metadata, storage, or cloning fails.
    pub fn clone_repository(
        &self,
        url: String,
        name: Option<String>,
        shallow: ShallowChoiceView,
    ) -> Result<String, GitRuntimeError> {
        let (default_branch, depth) = self.clone_choice(&url, shallow)?;
        let project_name = name.unwrap_or_else(|| {
            url.rsplit('/')
                .next()
                .unwrap_or("project")
                .trim_end_matches(".git")
                .to_owned()
        });
        let mut store = self.store()?;
        let project = store
            .create_project(&project_name, now_ms())
            .map_err(|_| GitRuntimeError::Unavailable)?;
        let destination = Path::new(&self.workspaces_root).join(&project.path);
        let result = tether_vcs::clone(
            CloneRequest {
                url: url.clone(),
                dest: destination.clone(),
                depth,
                branch: Some(default_branch.clone()),
                credentials: self.token().ok(),
            },
            &SilentCloneProgress,
            &tether_vcs::CancellationToken::new(),
        );
        if result.is_err() {
            drop(std::fs::remove_dir_all(&destination));
            drop(store.delete_project(&project.id));
            return Err(GitRuntimeError::Repository {
                detail: "clone failed".into(),
            });
        }
        store
            .set_project_remote(&project.id, &url, &default_branch)
            .map_err(|_| GitRuntimeError::Unavailable)?;
        Ok(project.id.to_string())
    }
}

impl GitRuntime {
    fn clone_choice(
        &self,
        url: &str,
        choice: ShallowChoiceView,
    ) -> Result<(String, Option<u32>), GitRuntimeError> {
        if url.starts_with("file://") {
            return Ok((
                "main".into(),
                matches!(choice, ShallowChoiceView::Shallow).then_some(1),
            ));
        }
        let (owner, repo) = parse_github_url(url).ok_or(GitRuntimeError::InvalidInput)?;
        let client = RuntimeGithub(Arc::new(self.clone_for_factory()?))
            .client()
            .map_err(|_| GitRuntimeError::GithubNetwork)?;
        let info = self
            .runtime
            .block_on(client.get_repo(&owner, &repo))
            .map_err(|_| GitRuntimeError::GithubResponse)?;
        let use_shallow = match choice {
            ShallowChoiceView::Auto => should_clone_shallow(info.size_kb),
            ShallowChoiceView::Shallow => true,
            ShallowChoiceView::Full => false,
        };
        Ok((info.default_branch, use_shallow.then_some(1)))
    }

    fn clone_for_factory(&self) -> Result<Self, GitRuntimeError> {
        Ok(Self {
            database_path: self.database_path.clone(),
            workspaces_root: self.workspaces_root.clone(),
            vault: Arc::clone(&self.vault),
            runtime: Arc::clone(&self.runtime),
            oauth_client_id: Mutex::new(
                self.oauth_client_id
                    .lock()
                    .map_err(|_| GitRuntimeError::Unavailable)?
                    .clone(),
            ),
            device_grant: Mutex::new(None),
        })
    }

    pub(crate) fn store(&self) -> Result<Store, GitRuntimeError> {
        Store::open(Path::new(&self.database_path)).map_err(|_| GitRuntimeError::Unavailable)
    }

    fn project_root(&self, project_id: &str) -> Result<PathBuf, GitRuntimeError> {
        let project = self
            .store()?
            .load_project(&ProjectId::new(project_id.to_owned()))
            .map_err(|_| GitRuntimeError::Unavailable)?
            .ok_or(GitRuntimeError::NotFound)?;
        Ok(Path::new(&self.workspaces_root).join(project.path))
    }

    fn repo(&self, project_id: &str) -> Result<Repo, GitRuntimeError> {
        Repo::open(self.project_root(project_id)?).map_err(repo_error)
    }

    pub(crate) fn token(&self) -> Result<Secret, String> {
        let reference = SecretRef::new("github-oauth")
            .map_err(|_| "GitHub token reference is invalid".to_owned())?;
        self.vault
            .store
            .get(&reference)
            .map_err(|_| "GitHub token is unavailable".to_owned())?
            .ok_or_else(|| "GitHub is not signed in".to_owned())
    }
}

pub(crate) struct RuntimeIdentity(Arc<GitRuntime>);
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

pub(crate) struct RuntimeTransport(Arc<GitRuntime>);
impl TransportFactory for RuntimeTransport {
    fn transport(&self) -> Result<Box<dyn Transport>, String> {
        Ok(Box::new(ReqwestTransport::new(self.0.token()).map_err(
            |_| "GitHub transport is unavailable".to_owned(),
        )?))
    }
}

pub(crate) struct RuntimeGithub(Arc<GitRuntime>);
impl GithubClientFactory for RuntimeGithub {
    fn client(&self) -> Result<tether_vcs::GithubClient, String> {
        let http = Arc::new(
            ReqwestHttpClient::new().map_err(|_| "GitHub HTTP is unavailable".to_owned())?,
        );
        Ok(tether_vcs::GithubClient::new(http, self.0.token()?))
    }
}

fn validate_identity_field(value: &str) -> Result<(), GitRuntimeError> {
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

fn status_view(status: StatusEntry) -> GitStatusView {
    GitStatusView {
        path: status.path,
        index_state: format!("{:?}", status.index_state),
        worktree_state: format!("{:?}", status.worktree_state),
    }
}

fn commit_view(commit: CommitInfo) -> GitCommitView {
    GitCommitView {
        id: commit.id,
        subject: commit.subject,
        author_name: commit.author_name,
        author_email: commit.author_email,
        committer_time: commit.committer_time,
    }
}

fn branch_view(branch: BranchInfo) -> GitBranchView {
    GitBranchView {
        name: branch.name,
        commit: branch.commit,
    }
}

fn repo_error(error: RepoError) -> GitRuntimeError {
    GitRuntimeError::Repository {
        detail: error.to_string(),
    }
}

fn cap_diff(mut diff: String) -> String {
    const LIMIT: usize = 64 * 1024;
    if diff.len() <= LIMIT {
        return diff;
    }
    diff.truncate(LIMIT);
    diff.push_str("\n[diff truncated at 64 KiB]\n");
    diff
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

struct SilentCloneProgress;
impl CloneProgress for SilentCloneProgress {
    fn update(&self, _phase: tether_vcs::ClonePhase, _received: u64, _total: Option<u64>) {}
}
