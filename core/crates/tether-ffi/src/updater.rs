//! UniFFI API for checking and downloading full-build updates.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tether_engine::updates::{self, Release, UpdateCheck, UpdateConfig, UpdateError};
use tokio::runtime::Builder;
use tokio_util::sync::CancellationToken;

/// Release metadata displayed by the Android About screen.
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct ReleaseView {
    /// Version without the leading `v`.
    pub version: String,
    /// Release tag including the leading `v`.
    pub tag: String,
    /// Markdown release notes.
    pub notes: String,
    /// Publication timestamp.
    pub published_at: String,
    /// APK file name.
    pub apk_name: String,
    /// APK size in bytes.
    pub apk_size_bytes: u64,
}

/// Result of comparing the installed app with the latest release.
#[derive(Debug, PartialEq, Eq, uniffi::Enum)]
pub enum UpdateCheckView {
    /// No newer release is available.
    UpToDate {
        /// Installed version.
        current: String,
        /// Latest release version.
        latest: String,
    },
    /// A newer release is available.
    Available {
        /// Release metadata.
        release: ReleaseView,
    },
}

/// Errors exposed by the updater without local paths or remote URL details.
#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum UpdaterError {
    /// The network request failed.
    #[error("network error")]
    Network,
    /// No usable release was found.
    #[error("update unavailable")]
    Unavailable,
    /// The update data was invalid.
    #[error("invalid update data")]
    Invalid,
    /// The downloaded APK did not match its published digest.
    #[error("update checksum failed")]
    Checksum,
    /// The APK exceeds the size limit.
    #[error("update is too large")]
    TooLarge,
    /// Local storage failed.
    #[error("update storage failed")]
    Storage,
    /// The operation was cancelled.
    #[error("update cancelled")]
    Cancelled,
    /// No matching release has been checked.
    #[error("no release is available")]
    NoRelease,
}

impl From<UpdateError> for UpdaterError {
    fn from(error: UpdateError) -> Self {
        match error {
            UpdateError::Network => Self::Network,
            UpdateError::Unavailable => Self::Unavailable,
            UpdateError::Invalid => Self::Invalid,
            UpdateError::Checksum => Self::Checksum,
            UpdateError::TooLarge => Self::TooLarge,
            UpdateError::Storage => Self::Storage,
            UpdateError::Cancelled => Self::Cancelled,
        }
    }
}

/// Receives whole-percent download progress from a background updater call.
#[uniffi::export(with_foreign)]
pub trait DownloadProgress: Send + Sync {
    /// Receives downloaded and total byte counts.
    fn on_progress(&self, downloaded: u64, total: u64);
}

struct DownloadRequest<'a> {
    release: &'a Release,
    dir: &'a Path,
    progress: &'a (dyn Fn(u64, u64) + Sync),
    cancel: CancellationToken,
}

trait Checker: Send + Sync {
    fn check(&self, current_version: &str) -> Result<UpdateCheck, UpdateError>;
    fn download(&self, request: DownloadRequest<'_>) -> Result<PathBuf, UpdateError>;
}

struct EngineChecker;

impl Checker for EngineChecker {
    fn check(&self, current_version: &str) -> Result<UpdateCheck, UpdateError> {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| UpdateError::Network)?;
        runtime.block_on(updates::check(&UpdateConfig::github(), current_version))
    }

    fn download(&self, request: DownloadRequest<'_>) -> Result<PathBuf, UpdateError> {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| UpdateError::Storage)?;
        runtime.block_on(updates::download(
            &UpdateConfig::github(),
            request.release,
            request.dir,
            request.progress,
            request.cancel,
        ))
    }
}

/// The updater object held by the Android app.
#[derive(uniffi::Object)]
pub struct Updater {
    download_dir: PathBuf,
    checker: Arc<dyn Checker>,
    release: Mutex<Option<Release>>,
    active_cancel: Mutex<Option<Arc<CancellationToken>>>,
}

#[uniffi::export]
impl Updater {
    /// Creates an updater for `download_dir`.
    #[uniffi::constructor]
    #[must_use]
    pub fn new(download_dir: String) -> Arc<Self> {
        Arc::new(Self::with_checker(
            PathBuf::from(download_dir),
            Arc::new(EngineChecker),
        ))
    }

    /// Checks GitHub for a release. Kotlin must call this off the main thread.
    ///
    /// # Errors
    /// Returns an [`UpdaterError`] when the release check fails.
    #[allow(clippy::needless_pass_by_value)]
    pub fn check(&self, current_version: String) -> Result<UpdateCheckView, UpdaterError> {
        let result = self
            .checker
            .check(&current_version)
            .map_err(UpdaterError::from)?;
        if let UpdateCheck::Available(release) = &result {
            let mut cached = self.release.lock().map_err(|_| UpdaterError::Storage)?;
            *cached = Some(release.clone());
        }
        Ok(UpdateCheckView::from(result))
    }

    /// Downloads a checked release. Kotlin must call this off the main thread.
    ///
    /// # Errors
    /// Returns [`UpdaterError::NoRelease`] when `version` was not returned by the latest
    /// available check, or another [`UpdaterError`] when downloading fails.
    #[allow(clippy::needless_pass_by_value)]
    pub fn download(
        &self,
        version: String,
        progress: Arc<dyn DownloadProgress>,
    ) -> Result<String, UpdaterError> {
        let release = self.cached_release(&version)?;
        let token = Arc::new(CancellationToken::new());
        {
            let mut active = self
                .active_cancel
                .lock()
                .map_err(|_| UpdaterError::Storage)?;
            *active = Some(token.clone());
        }
        let callback = |downloaded, total| progress.on_progress(downloaded, total);
        let result = self
            .checker
            .download(DownloadRequest {
                release: &release,
                dir: &self.download_dir,
                progress: &callback,
                cancel: (*token).clone(),
            })
            .map_err(UpdaterError::from);
        self.clear_cancel(&token)?;
        let path = result?;
        absolute_path(&path)
    }

    /// Cancels the download currently in flight, if any.
    pub fn cancel(&self) {
        if let Ok(active) = self.active_cancel.lock()
            && let Some(token) = active.as_ref()
        {
            token.cancel();
        }
    }
}

impl Updater {
    fn with_checker(download_dir: PathBuf, checker: Arc<dyn Checker>) -> Self {
        Self {
            download_dir,
            checker,
            release: Mutex::new(None),
            active_cancel: Mutex::new(None),
        }
    }

    fn cached_release(&self, version: &str) -> Result<Release, UpdaterError> {
        let release = self.release.lock().map_err(|_| UpdaterError::Storage)?;
        release
            .as_ref()
            .filter(|release| release.version == version)
            .cloned()
            .ok_or(UpdaterError::NoRelease)
    }

    fn clear_cancel(&self, token: &Arc<CancellationToken>) -> Result<(), UpdaterError> {
        let mut active = self
            .active_cancel
            .lock()
            .map_err(|_| UpdaterError::Storage)?;
        if active
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, token))
        {
            *active = None;
        }
        Ok(())
    }
}

fn absolute_path(path: &Path) -> Result<String, UpdaterError> {
    std::path::absolute(path)
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|_| UpdaterError::Storage)
}

impl From<UpdateCheck> for UpdateCheckView {
    fn from(check: UpdateCheck) -> Self {
        match check {
            UpdateCheck::UpToDate { current, latest } => Self::UpToDate { current, latest },
            UpdateCheck::Available(release) => Self::Available {
                release: ReleaseView::from(release),
            },
        }
    }
}

impl From<Release> for ReleaseView {
    fn from(release: Release) -> Self {
        Self {
            version: release.version,
            tag: release.tag,
            notes: release.notes,
            published_at: release.published_at,
            apk_name: release.apk.name,
            apk_size_bytes: release.apk.size,
        }
    }
}

#[cfg(test)]
mod tests;
