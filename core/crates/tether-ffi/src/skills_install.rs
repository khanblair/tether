//! Skill quarantine, installation, and prune-only upkeep.

use std::sync::atomic::AtomicU64;
use std::time::{SystemTime, UNIX_EPOCH};

use tether_workspace::archive::ExtractLimits;

mod api;
mod download;
mod github_url;
mod preview;
mod quarantine;
mod scope;
mod tree;
mod upkeep;
mod views;

pub use views::{
    InstalledSkillView, QuarantineFileView, QuarantineView, ScanFindingView, ScanReportView,
    ScanSeverityView, ScanVerdictView, SkillScopeView, SkillStateView, UpkeepActionView,
};

const BODY_CAP: usize = 5 * 1024 * 1024;
const QUARANTINE_TTL_MS: i64 = 24 * 86_400_000;
const LIMITS: ExtractLimits = ExtractLimits {
    max_entries: 100,
    max_total_bytes: 1 << 20,
    max_file_bytes: 256 << 10,
};
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn system_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
use std::{path::PathBuf, sync::Arc};

#[cfg(test)]
use download::SkillDownloader;

#[cfg(test)]
use crate::skills::{SkillsRuntime, SkillsRuntimeError};

#[cfg(test)]
impl SkillsRuntime {
    fn with_clock(root: PathBuf, clock: Arc<dyn Fn() -> i64 + Send + Sync>) -> Arc<Self> {
        Arc::new(Self { root, clock })
    }
}

#[cfg(test)]
#[path = "skills_install_tests.rs"]
mod tests;
