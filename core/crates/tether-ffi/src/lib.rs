//! UniFFI surface of the core, built as `libtether_core.so`.

mod callbacks;
mod files;
mod memory;
mod projects;
mod provider_settings;
mod secrets;
mod sessions;
mod skills;
mod updater;
mod vault;
mod views;

use std::sync::Arc;

use tether_engine::Core;

pub use callbacks::EventSink;
pub use files::{
    DirListingView, FileEntryView, FilePreviewView, FileTextHitView, FilesRuntime,
    FilesRuntimeError,
};
pub use memory::{
    MemoryActionView, MemoryResultView, MemoryRuntime, MemoryRuntimeError, MemoryTargetView,
};
pub use projects::{ProjectRuntime, ProjectRuntimeError};
pub use provider_settings::{ProviderSettings, ProviderSettingsError, ProviderStatusView};
pub use secrets::{SecretVault, SecretVaultError, SecretsPlatform, SecretsPlatformFailure};
pub use sessions::{SessionRunner, SessionRuntime, SessionRuntimeError};
pub use skills::{SkillDetailView, SkillView, SkillsRuntime, SkillsRuntimeError};
pub use updater::{DownloadProgress, ReleaseView, UpdateCheckView, Updater, UpdaterError};
pub use vault::{VaultLinkView, VaultLinksView, VaultNoteView, VaultRuntime, VaultRuntimeError};
pub use views::{
    CheckpointView, ConnectionStatusView, CoreErrorView, CoreEventView, ErrorKindView,
    ExecutorView, MessagePartView, MessageView, ModelRefView, ModelView, PermissionModeView,
    ProjectView, ProviderView, RewindScopeView, RoleView, SessionRecordView, SessionStatusView,
    SessionUsageView, SupportView, ToolCallStatusView, TranscriptHitView, UsageSnapshotView,
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
