//! UniFFI runtime for local Git repositories and clone operations.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tether_engine::{GitTools, GitToolsConfig, GithubClientFactory, ToolHandler};
use tether_secrets::{Secret, SecretRef};
use tether_store::{GitIdentity, Store};
use tether_types::ProjectId;
use tether_vcs::{CloneRequest, DiffSpec, Repo, parse_github_url, should_clone_shallow};

use crate::SecretVault;
use crate::git_support::{
    RuntimeGithub, RuntimeIdentity, RuntimeTransport, SilentCloneProgress, branch_view, cap_diff,
    commit_view, now_ms, repo_error, status_view, validate_identity_field,
};
use crate::git_types::{
    GitBranchView, GitCommitView, GitDiffSpecView, GitRuntimeError, GitStatusView,
    ShallowChoiceView,
};
use crate::github::{DeviceGrantState, GithubRepoView};

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
}

#[uniffi::export]
impl GitRuntime {
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
    pub fn git_status(&self, project_id: &str) -> Result<Vec<GitStatusView>, GitRuntimeError> {
        let repo = self.repo(project_id)?;
        repo.status()
            .map(|rows| rows.into_iter().map(status_view).collect())
            .map_err(|error| repo_error(&error))
    }

    /// Reads a bounded repository diff for one project.
    ///
    /// # Errors
    /// Returns an error when the project or revision cannot be read.
    pub fn git_diff(
        &self,
        project_id: &str,
        spec: GitDiffSpecView,
    ) -> Result<String, GitRuntimeError> {
        let repo = self.repo(project_id)?;
        let diff = match (spec.from, spec.to) {
            (Some(from), Some(to)) => repo.diff(DiffSpec::Revisions { old: from, new: to }),
            (None, None) if spec.staged => repo.diff(DiffSpec::Staged),
            (None, None) => repo.diff(DiffSpec::Worktree),
            _ => return Err(GitRuntimeError::InvalidInput),
        }
        .map_err(|error| repo_error(&error))?;
        Ok(cap_diff(diff))
    }

    /// Reads recent commits for one project.
    ///
    /// # Errors
    /// Returns an error when the project or history cannot be read.
    pub fn git_log(
        &self,
        project_id: &str,
        limit: u32,
    ) -> Result<Vec<GitCommitView>, GitRuntimeError> {
        if !(1..=50).contains(&limit) {
            return Err(GitRuntimeError::InvalidInput);
        }
        self.repo(project_id)?
            .log(limit as usize, None)
            .map(|rows| rows.into_iter().map(commit_view).collect())
            .map_err(|error| repo_error(&error))
    }

    /// Lists local branches for one project.
    ///
    /// # Errors
    /// Returns an error when the project or repository cannot be read.
    pub fn git_branches(&self, project_id: &str) -> Result<Vec<GitBranchView>, GitRuntimeError> {
        self.repo(project_id)?
            .branches()
            .map(|rows| rows.into_iter().map(branch_view).collect())
            .map_err(|error| repo_error(&error))
    }

    /// Returns the current branch for one project.
    ///
    /// # Errors
    /// Returns an error when the project or repository cannot be read.
    pub fn current_branch(&self, project_id: &str) -> Result<Option<String>, GitRuntimeError> {
        self.repo(project_id)?
            .current_branch()
            .map_err(|error| repo_error(&error))
    }

    /// Initializes a repository in an empty project folder.
    ///
    /// # Errors
    /// Returns an error when the project is absent, non-empty, or the branch is invalid.
    pub fn git_init(&self, project_id: &str, default_branch: &str) -> Result<(), GitRuntimeError> {
        let path = self.project_root(project_id)?;
        Repo::init(path, default_branch)
            .map(|_| ())
            .map_err(|error| repo_error(&error))
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
        url: &str,
        name: Option<String>,
        shallow: ShallowChoiceView,
    ) -> Result<String, GitRuntimeError> {
        let (default_branch, depth) = self.clone_choice(url, shallow)?;
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
                url: url.to_owned(),
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
            .set_project_remote(&project.id, url, &default_branch)
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
        Repo::open(self.project_root(project_id)?).map_err(|error| repo_error(&error))
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
