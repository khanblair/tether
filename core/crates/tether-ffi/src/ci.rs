//! UniFFI runtime for GitHub Actions runs and log views.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tether_ci::{CiClient, CiError, RunSummary, propose_workflow, workflow_is_current};
use tether_engine::{CiLogResult, CiLogsRequest, ToolHandler};
use tether_secrets::Secret;
use tether_store::{ProjectRecord, Store, project_dir};
use tether_types::ProjectId;
use tether_vcs::parse_github_url;

#[path = "ci_service.rs"]
mod ci_service;
#[path = "ci_types.rs"]
mod ci_types;
use crate::git::GitRuntime;
pub use ci_types::{
    CiFailureView, CiLogView, CiRunView, CiRuntimeError, CiStatusView, WorkflowProposalView,
};

/// Owns CI credentials, project metadata, and the shared CI service.
#[derive(uniffi::Object)]
pub struct CiRuntime {
    pub(crate) database_path: String,
    pub(crate) workspaces_root: String,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
    pub(crate) github: Arc<GitRuntime>,
}

impl CiRuntime {
    /// Creates a CI handler for a project with a GitHub remote.
    pub(crate) fn tool_handler(
        runtime: Arc<Self>,
        project_id: ProjectId,
        branch: String,
    ) -> Arc<dyn ToolHandler> {
        Arc::new(tether_engine::CiTools::new(runtime, project_id, branch))
    }

    /// Returns an executor that uses this runtime's CI service.
    #[must_use]
    pub fn executor(
        self: &Arc<Self>,
        _project_id: &str,
    ) -> Arc<dyn tether_engine::executors::CiExecutor> {
        self.clone()
    }

    pub(crate) fn token(&self) -> Result<Secret, CiRuntimeError> {
        self.github.token().map_err(|_| CiRuntimeError::NotSignedIn)
    }

    pub(crate) fn store(&self) -> Result<Store, CiRuntimeError> {
        Store::open(Path::new(&self.database_path)).map_err(|_| CiRuntimeError::Unavailable)
    }
    pub(crate) fn project(&self, id: &ProjectId) -> Result<ProjectRecord, CiRuntimeError> {
        self.store()?
            .load_project(id)
            .map_err(|_| CiRuntimeError::Unavailable)?
            .ok_or(CiRuntimeError::Repository)
    }
    pub(crate) fn root(&self, project: &ProjectRecord) -> PathBuf {
        project_dir(Path::new(&self.workspaces_root), project)
    }
    pub(crate) fn workflow_path(&self, project: &ProjectRecord) -> PathBuf {
        self.root(project).join(".github/workflows/tether.yml")
    }
    pub(crate) fn workflow_content(&self, project: &ProjectRecord) -> Option<String> {
        let path = self.workflow_path(project);
        let metadata = fs::symlink_metadata(&path).ok()?;
        metadata
            .file_type()
            .is_file()
            .then(|| fs::read_to_string(path).ok())?
    }
    pub(crate) fn client(
        &self,
        project: &ProjectRecord,
    ) -> Result<(CiClient, String, String), CiRuntimeError> {
        let remote = self
            .store()?
            .project_remote(&project.id)
            .map_err(|_| CiRuntimeError::Unavailable)?
            .ok_or(CiRuntimeError::NoRemote)?;
        let (owner, repo) = remote
            .remote_url
            .as_deref()
            .and_then(parse_github_url)
            .ok_or(CiRuntimeError::NoRemote)?;
        let http = Arc::new(
            tether_vcs::ReqwestHttpClient::new().map_err(|_| CiRuntimeError::Unavailable)?,
        );
        Ok((
            CiClient::for_repository(http, self.token()?, &owner, &repo),
            owner,
            repo,
        ))
    }
}

#[uniffi::export]
impl CiRuntime {
    /// Opens the CI runtime over the app database, workspaces, and vault.
    ///
    /// # Errors
    /// Returns an error when the database or runtime cannot start.
    #[uniffi::constructor]
    pub fn new(
        database_path: &str,
        workspaces_root: &str,
        vault: Arc<crate::SecretVault>,
    ) -> Result<Arc<Self>, CiRuntimeError> {
        Store::open(Path::new(database_path)).map_err(|_| CiRuntimeError::Unavailable)?;
        let github = GitRuntime::new(database_path, workspaces_root, vault)
            .map_err(|_| CiRuntimeError::Unavailable)?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|_| CiRuntimeError::Unavailable)?;
        Ok(Arc::new(Self {
            database_path: database_path.into(),
            workspaces_root: workspaces_root.into(),
            runtime: Arc::new(runtime),
            github,
        }))
    }

    /// Reads CI readiness and workflow state for one project.
    ///
    /// # Errors
    /// Returns an error when project storage cannot be read.
    pub fn ci_status(&self, project_id: &str) -> Result<CiStatusView, CiRuntimeError> {
        let Ok(project) = self.project(&ProjectId::new(project_id)) else {
            return Ok(CiStatusView {
                ready: false,
                workflow_present: false,
                workflow_current: false,
                reason: "project repository is unavailable".into(),
            });
        };
        let signed_in = self.token().is_ok();
        let github_remote = self
            .store()?
            .project_remote(&project.id)
            .map_err(|_| CiRuntimeError::Unavailable)?
            .and_then(|remote| remote.remote_url)
            .and_then(|url| parse_github_url(&url))
            .is_some();
        let content = self.workflow_content(&project);
        let workflow_current = workflow_is_current(content.as_deref());
        let stage = if !signed_in {
            CiStatusStage::SignedOut
        } else if !github_remote {
            CiStatusStage::NoRemote
        } else if content.is_none() {
            CiStatusStage::MissingWorkflow
        } else if !workflow_current {
            CiStatusStage::OutdatedWorkflow
        } else {
            CiStatusStage::Ready
        };
        let reason = status_reason(&stage);
        Ok(CiStatusView {
            ready: signed_in && github_remote,
            workflow_present: content.is_some(),
            workflow_current,
            reason: reason.into(),
        })
    }

    /// Returns the workflow proposal for one project.
    ///
    /// # Errors
    /// Returns an error when project storage cannot be read.
    pub fn workflow_proposal(
        &self,
        project_id: &str,
    ) -> Result<Option<WorkflowProposalView>, CiRuntimeError> {
        let project = self.project(&ProjectId::new(project_id))?;
        Ok(workflow_view(self.workflow_content(&project).as_deref()))
    }

    /// Lists recent workflow runs for one project.
    ///
    /// # Errors
    /// Returns an error when project storage or GitHub is unavailable.
    pub fn list_ci_runs(
        &self,
        project_id: &str,
        limit: u32,
    ) -> Result<Vec<CiRunView>, CiRuntimeError> {
        let project = self.project(&ProjectId::new(project_id))?;
        let (client, owner, repo) = self.client(&project)?;
        self.runtime
            .block_on(client.list_runs(&owner, &repo, ci_limit(limit)))
            .map(|runs| runs.into_iter().map(run_view).collect())
            .map_err(|error| ci_error(&error))
    }

    /// Reads and parses logs for one run.
    ///
    /// # Errors
    /// Returns an error when project storage or GitHub is unavailable.
    pub fn ci_run_logs(&self, project_id: &str, run_id: u64) -> Result<CiLogView, CiRuntimeError> {
        let request = CiLogsRequest {
            run_id: Some(run_id),
            job: None,
            project_id: ProjectId::new(project_id),
            cancel: tokio_util::sync::CancellationToken::new(),
        };
        self.runtime
            .block_on(ci_service::logs(self, request))
            .map(log_view)
            .map_err(|error| CiRuntimeError::Github {
                detail: error.to_string(),
            })
    }

    /// Cancels one workflow run.
    ///
    /// # Errors
    /// Returns an error when project storage or GitHub is unavailable.
    pub fn cancel_ci_run(&self, project_id: &str, run_id: u64) -> Result<(), CiRuntimeError> {
        let project = self.project(&ProjectId::new(project_id))?;
        let (client, _, _) = self.client(&project)?;
        self.runtime
            .block_on(client.cancel_run(run_id))
            .map_err(|error| ci_error(&error))
    }
}

fn run_view(run: RunSummary) -> CiRunView {
    CiRunView {
        id: run.id,
        number: run.number,
        title: run.title,
        branch: run.branch,
        status: run.status,
        conclusion: run.conclusion.unwrap_or_default(),
        started_at: run.started_at.unwrap_or_default(),
        duration_seconds: run.duration_seconds.unwrap_or_default(),
        html_url: run.html_url,
    }
}
fn log_view(result: CiLogResult) -> CiLogView {
    CiLogView {
        text: result.text,
        failures: result.failures.into_iter().map(Into::into).collect(),
    }
}
fn ci_error(error: &CiError) -> CiRuntimeError {
    CiRuntimeError::Github {
        detail: error.to_string(),
    }
}

fn ci_limit(limit: u32) -> usize {
    limit.clamp(1, 20) as usize
}

enum CiStatusStage {
    SignedOut,
    NoRemote,
    MissingWorkflow,
    OutdatedWorkflow,
    Ready,
}

fn status_reason(stage: &CiStatusStage) -> &'static str {
    match stage {
        CiStatusStage::SignedOut => "GitHub is not signed in",
        CiStatusStage::NoRemote => "project does not have a GitHub remote",
        CiStatusStage::MissingWorkflow => "workflow is missing",
        CiStatusStage::OutdatedWorkflow => "workflow is outdated",
        CiStatusStage::Ready => "",
    }
}

fn workflow_view(existing: Option<&str>) -> Option<WorkflowProposalView> {
    propose_workflow(existing).map(|proposal| WorkflowProposalView {
        path: proposal.path,
        content: proposal.content,
        diff: proposal.diff,
        is_new: proposal.is_new,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_status_reports_each_missing_precondition_in_order() {
        assert_eq!(
            status_reason(&CiStatusStage::SignedOut),
            "GitHub is not signed in"
        );
        assert_eq!(
            status_reason(&CiStatusStage::NoRemote),
            "project does not have a GitHub remote"
        );
        assert_eq!(
            status_reason(&CiStatusStage::MissingWorkflow),
            "workflow is missing"
        );
        assert_eq!(
            status_reason(&CiStatusStage::OutdatedWorkflow),
            "workflow is outdated"
        );
        assert_eq!(status_reason(&CiStatusStage::Ready), "");
    }

    #[test]
    fn ci_run_limit_is_clamped_to_one_through_twenty() {
        assert_eq!(ci_limit(0), 1);
        assert_eq!(ci_limit(1), 1);
        assert_eq!(ci_limit(20), 20);
        assert_eq!(ci_limit(99), 20);
    }

    #[test]
    fn workflow_proposal_view_keeps_exact_new_file_content() {
        let proposal = workflow_view(None).expect("proposal");
        assert_eq!(proposal.path, ".github/workflows/tether.yml");
        assert!(proposal.is_new);
        assert_eq!(proposal.content, tether_ci::workflow_template());
        assert!(proposal.diff.contains("+name: tether"));
    }
}
