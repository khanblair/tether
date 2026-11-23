//! Platform-facing skill install, quarantine, and upkeep entry points.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use tether_skills::plan_upkeep;

use super::download::{GithubDownloader, SkillDownloader};
use super::github_url::parse_github_url;
use super::scope::valid_name;
use super::views::{
    InstalledSkillView, QuarantineView, ScanVerdictView, SkillScopeView, UpkeepActionView,
};
use crate::skills::{SkillsRuntime, SkillsRuntimeError};

#[uniffi::export]
impl SkillsRuntime {
    /// Previews a GitHub skill in quarantine without installing it.
    ///
    /// # Errors
    /// Returns a typed URL, download, archive, scan, or storage error.
    pub fn skill_install_preview_url(
        &self,
        url: String,
    ) -> Result<QuarantineView, SkillsRuntimeError> {
        let _ = parse_github_url(&url)?;
        let downloader: Arc<dyn SkillDownloader> = Arc::new(GithubDownloader);
        self.preview_with(url.clone(), downloader, url)
    }

    /// Previews a local ZIP skill in quarantine without installing it.
    ///
    /// # Errors
    /// Returns a typed archive, scan, or storage error.
    #[expect(clippy::needless_pass_by_value, reason = "UniFFI passes owned values")]
    pub fn skill_install_preview_zip(
        &self,
        zip_path: String,
    ) -> Result<QuarantineView, SkillsRuntimeError> {
        let path = PathBuf::from(&zip_path);
        if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(SkillsRuntimeError::InvalidPath);
        }
        let bytes = fs::read(path).map_err(|_| SkillsRuntimeError::Download)?;
        if bytes.len() > super::BODY_CAP {
            return Err(SkillsRuntimeError::TooLarge);
        }
        self.preview_bytes(&bytes, "Local ZIP".into())
    }

    /// Approves one clean quarantined skill and moves it without changing its files.
    ///
    /// # Errors
    /// Returns a typed quarantine, trust, collision, or storage error.
    #[expect(clippy::needless_pass_by_value, reason = "UniFFI passes owned values")]
    pub fn approve_skill_install(
        &self,
        id: String,
        scope: SkillScopeView,
    ) -> Result<InstalledSkillView, SkillsRuntimeError> {
        let record = self.record(&id)?;
        if record.scan.verdict == ScanVerdictView::Blocked {
            return Err(SkillsRuntimeError::Blocked);
        }
        let target_root = self.scope_root(&scope)?;
        let target = target_root.join(&record.name);
        if fs::symlink_metadata(&target).is_ok() {
            return Err(SkillsRuntimeError::AlreadyInstalled);
        }
        fs::create_dir_all(&target_root).map_err(|_| SkillsRuntimeError::Unavailable)?;
        fs::rename(&record.skill_dir, &target).map_err(|_| SkillsRuntimeError::Unavailable)?;
        drop(fs::remove_dir_all(&record.id_dir));
        drop(tether_skills::SkillsCatalog::discover(
            tether_skills::DiscoverInput {
                project_root: None,
                project_trusted: false,
                user_dir: &target_root,
                bundled: &[],
            },
        ));
        Ok(InstalledSkillView {
            name: record.name,
            scope,
        })
    }

    /// Removes one quarantined skill.
    ///
    /// # Errors
    /// Returns an invalid id or storage error.
    #[expect(clippy::needless_pass_by_value, reason = "UniFFI passes owned values")]
    pub fn discard_quarantine(&self, id: String) -> Result<(), SkillsRuntimeError> {
        let path = self.quarantine_dir(&id)?;
        fs::remove_dir_all(path).map_err(|_| SkillsRuntimeError::Unavailable)
    }

    /// Runs file-backed stale and archive transitions for user skills.
    ///
    /// # Errors
    /// Returns a storage error when a state transition cannot be applied.
    pub fn run_skill_upkeep(
        &self,
        now_ms: i64,
    ) -> Result<Vec<UpkeepActionView>, SkillsRuntimeError> {
        self.expire_quarantine(now_ms)?;
        let usages = self.usages()?;
        plan_upkeep(&usages, now_ms)
            .into_iter()
            .map(|action| self.apply_action(action))
            .collect()
    }

    /// Restores an archived user skill.
    ///
    /// # Errors
    /// Returns a missing, collision, invalid-name, or storage error.
    #[expect(clippy::needless_pass_by_value, reason = "UniFFI passes owned values")]
    pub fn restore_skill(&self, name: String) -> Result<(), SkillsRuntimeError> {
        if !valid_name(&name) {
            return Err(SkillsRuntimeError::InvalidPath);
        }
        let source = self.root.join(".archive").join(&name);
        let target = self.root.join(&name);
        if fs::symlink_metadata(&source).is_err() {
            return Err(SkillsRuntimeError::NotFound);
        }
        if fs::symlink_metadata(&target).is_ok() {
            return Err(SkillsRuntimeError::AlreadyInstalled);
        }
        fs::rename(source, target).map_err(|_| SkillsRuntimeError::Unavailable)
    }
}
