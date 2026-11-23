use tether_ci::CiFailure;

/// A safe CI readiness summary.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CiStatusView {
    /// Whether sign-in and a GitHub remote are available.
    pub ready: bool,
    /// Whether the workflow file exists.
    pub workflow_present: bool,
    /// Whether the workflow is current.
    pub workflow_current: bool,
    /// Why CI is unavailable.
    pub reason: String,
}

/// A workflow proposal shown before the user writes it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WorkflowProposalView {
    /// Destination path.
    pub path: String,
    /// Complete workflow content.
    pub content: String,
    /// Unified diff.
    pub diff: String,
    /// Whether the file is new.
    pub is_new: bool,
}

/// One run in the CI history list.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CiRunView {
    /// Numeric run id.
    pub id: u64,
    /// GitHub run number.
    pub number: u64,
    /// Command title.
    pub title: String,
    /// Branch.
    pub branch: String,
    /// Run status.
    pub status: String,
    /// Run conclusion.
    pub conclusion: String,
    /// Start time.
    pub started_at: String,
    /// Duration in seconds.
    pub duration_seconds: u64,
    /// Run page.
    pub html_url: String,
}

/// One parsed failure shown with a CI log.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CiFailureView {
    /// Framework name.
    pub framework: String,
    /// Test name.
    pub test_name: Option<String>,
    /// Repository-relative file.
    pub file: Option<String>,
    /// One-based line.
    pub line: Option<u32>,
    /// One-based column.
    pub column: Option<u32>,
    /// Failure message.
    pub message: String,
    /// Short excerpt.
    pub excerpt: String,
}

/// A trimmed CI log and parsed failures.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CiLogView {
    /// Trimmed log.
    pub text: String,
    /// Parsed failures.
    pub failures: Vec<CiFailureView>,
}

/// Errors from CI runtime calls.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CiRuntimeError {
    /// The project or repository is absent.
    #[error("project repository is unavailable")]
    Repository,
    /// GitHub sign-in is absent.
    #[error("GitHub is not signed in")]
    NotSignedIn,
    /// The project has no GitHub remote.
    #[error("project does not have a GitHub remote")]
    NoRemote,
    /// Storage or runtime setup failed.
    #[error("CI runtime is unavailable")]
    Unavailable,
    /// GitHub returned an error.
    #[error("GitHub Actions request failed: {detail}")]
    Github {
        /// Safe error detail.
        detail: String,
    },
}

impl From<CiFailure> for CiFailureView {
    fn from(failure: CiFailure) -> Self {
        Self {
            framework: failure.framework,
            test_name: failure.test_name,
            file: failure.file,
            line: failure.line,
            column: failure.column,
            message: failure.message,
            excerpt: failure.excerpt,
        }
    }
}
