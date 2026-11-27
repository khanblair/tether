//! UniFFI surface of the core, built as `libtether_core.so`.

mod approvals;
mod archive;
mod callbacks;
mod ci;
mod commands;
mod config;
mod context;
mod executors;
mod files;
mod files_bytes;
mod git;
mod git_support;
mod git_types;
mod github;
mod github_token_info;
mod knowledge;
mod mcp;
mod mcp_types;
mod memory;
mod network;
mod notify;
mod project_location;
mod projects;
mod provider_settings;
mod quality;
mod questions;
mod resume;
mod secrets;
mod session_control;
mod sessions;
mod skills;
mod skills_install;
mod updater;
mod usage;
mod vault;
mod vault_archive;
mod vault_manifest;
mod vault_sync;
mod views;

use std::sync::Arc;

use tether_engine::Core;

pub use callbacks::{EventSink, NetworkPlatform, NotifyPlatform, RunServicePlatform};
pub use ci::{
    CiFailureView, CiLogView, CiRunView, CiRuntime, CiRuntimeError, CiStatusView,
    WorkflowProposalView,
};
pub use commands::{CommandResolutionView, CommandRuntimeError, CommandView, SendOutcomeView};
pub use config::{ConfigIssueView, EffectiveConfigView};
pub use executors::{
    ExecutorAvailabilityView, ExecutorPathsView, ExecutorProgress, ExecutorRuntimeError,
    RuntimePackView,
};
pub use files::{
    DirListingView, FileEntryView, FilePreviewView, FileTextHitView, FilesRuntime,
    FilesRuntimeError,
};
pub use files_bytes::FileBytesView;
pub use git::GitRuntime;
pub use git_types::{
    CloneProgressView, GitBranchView, GitCommitView, GitDiffSpecView, GitRuntimeError,
    GitStatusView, ShallowChoiceView,
};
pub use github::{
    DeviceGrantView, GithubRepoView, GithubStatusView, GithubTokenKindView, SignInStateView,
};
pub use mcp::McpRuntime;
pub use mcp_types::{McpHeaderInput, McpRuntimeError, McpServerView, McpToolView};
pub use memory::{
    MemoryActionView, MemoryResultView, MemoryRuntime, MemoryRuntimeError, MemoryTargetView,
};
pub use projects::{ProjectRuntime, ProjectRuntimeError};
pub use provider_settings::{ProviderSettings, ProviderSettingsError, ProviderStatusView};
pub use quality::{
    QualityConfigView, QualityRuleView, QualityRuntime, QualityRuntimeError, ThresholdView,
};
pub use secrets::{SecretVault, SecretVaultError, SecretsPlatform, SecretsPlatformFailure};
pub use sessions::{
    SessionRunner, SessionRuntime, SessionRuntimeError, recover_interrupted_sessions,
};
pub use skills::{SkillDetailView, SkillView, SkillsRuntime, SkillsRuntimeError};
pub use skills_install::{
    InstalledSkillView, QuarantineFileView, QuarantineView, ScanFindingView, ScanReportView,
    ScanSeverityView, ScanVerdictView, SkillScopeView, SkillStateView, UpkeepActionView,
};
pub use updater::{DownloadProgress, ReleaseView, UpdateCheckView, Updater, UpdaterError};
pub use usage::{UsageRuntime, UsageRuntimeError};
pub use vault::{
    ImportMode, VaultFolderPlatform, VaultFolderPlatformError, VaultImportReportView,
    VaultLinkView, VaultLinksView, VaultNoteView, VaultRuntime, VaultRuntimeError,
    VaultSyncReportView,
};
pub use views::{
    BudgetScopeView, BudgetView, CheckpointView, CompactionView, ConnectionStatusView,
    ContextBreakdownView, ContextSectionView, CoreErrorView, CoreEventView, DayUsageView,
    ErrorKindView, ExecutorView, InterruptionView, MessagePartView, MessageView, ModelRefView,
    ModelUsageView, ModelView, PendingApprovalView, PendingQuestionView, PermissionModeView,
    PermissionRuleView, PlanView, ProjectUsageView, ProjectView, PromptSectionView, ProviderView,
    QualityCountsView, QualityFindingRecordView, ReasoningEffortView, RewindScopeView, RoleView,
    SessionRecordView, SessionStatusView, SessionUsageView, SupportView, TodoItemView,
    TodoStatusView, ToolCallStatusView, TranscriptHitView, UsageSnapshotView, UsageSummaryView,
    UsageTotalsView, WorkspaceLocationView,
};

uniffi::setup_scaffolding!();

/// Returns the core version. It equals the app version.
#[must_use]
#[uniffi::export]
pub fn version() -> String {
    tether_engine::VERSION.to_owned()
}

/// The object the platform layer holds for the life of the process.
#[derive(uniffi::Object)]
pub struct TetherCore {
    core: Core,
}

#[uniffi::export]
impl TetherCore {
    /// Starts the core. `sink` receives every event, starting with `Ready`.
    #[uniffi::constructor]
    pub fn new(sink: Arc<dyn EventSink>) -> Arc<Self> {
        let core = Core::new(Arc::new(callbacks::SinkAdapter(sink)));
        Arc::new(Self { core })
    }

    /// Asks the core to emit a `Test` event. P0 uses this to prove the event path.
    pub fn emit_test_event(&self, message: String) {
        self.core.emit_test_event(message);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_matches_package_version() {
        assert_eq!(super::version(), env!("CARGO_PKG_VERSION"));
    }
}
