//! Views, the error enum, and the progress callback for the Git runtime.

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
    /// No GitHub token is saved.
    #[error("GitHub is not signed in")]
    NotSignedIn,
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
