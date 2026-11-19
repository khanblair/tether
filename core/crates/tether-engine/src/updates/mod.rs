//! Check, download, and verify full-build application updates.

mod download;
mod release;
mod sums;
mod version;

use std::time::Duration;

use reqwest::redirect::Policy;
use reqwest::{Client, ClientBuilder, Url};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

const CHECK_TIMEOUT: Duration = Duration::from_secs(20);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// The maximum APK size accepted by the updater.
pub(crate) const MAX_APK_BYTES: u64 = 256 * 1024 * 1024;

/// Configuration for the GitHub release checked by the updater.
pub struct UpdateConfig {
    /// Base URL of the GitHub API.
    pub api_base: String,
    /// Repository in `owner/name` form.
    pub repo: String,
    /// Prefix used by the release APK asset.
    pub asset_prefix: String,
    #[cfg(test)]
    pub(crate) allow_local_http: bool,
}

impl UpdateConfig {
    /// Returns the production configuration for Tether's full APK releases.
    #[must_use]
    pub fn github() -> Self {
        Self {
            api_base: "https://api.github.com".to_owned(),
            repo: "khanblair/tether".to_owned(),
            asset_prefix: "tether-full-".to_owned(),
            #[cfg(test)]
            allow_local_http: false,
        }
    }
}

/// A downloadable release asset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    /// Exact asset file name.
    pub name: String,
    /// HTTPS browser download URL.
    pub url: String,
    /// Size reported by GitHub in bytes.
    pub size: u64,
}

/// The verified metadata needed to download one release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// Semantic version without the leading `v`.
    pub version: String,
    /// Release tag, including the leading `v`.
    pub tag: String,
    /// Markdown release notes, limited to 8,000 characters.
    pub notes: String,
    /// GitHub publication timestamp.
    pub published_at: String,
    /// Full-build APK asset.
    pub apk: Asset,
    /// SHA-256 checksum asset.
    pub sums: Asset,
}

/// The result of comparing an installed version with the latest release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateCheck {
    /// The installed version is current.
    UpToDate {
        /// Installed version.
        current: String,
        /// Latest release version.
        latest: String,
    },
    /// A newer release is available.
    Available(Release),
}

/// An update operation failed without retaining a URL, query, token, or path.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum UpdateError {
    /// The network request failed.
    #[error("network error")]
    Network,
    /// GitHub did not provide a usable release or asset.
    #[error("update unavailable")]
    Unavailable,
    /// A response or input did not meet the update contract.
    #[error("invalid update data")]
    Invalid,
    /// The downloaded bytes did not match the published digest.
    #[error("update checksum failed")]
    Checksum,
    /// The APK exceeds the updater size limit.
    #[error("update is too large")]
    TooLarge,
    /// Local storage could not be read or written.
    #[error("update storage failed")]
    Storage,
    /// The caller cancelled the operation.
    #[error("update cancelled")]
    Cancelled,
}

/// Checks GitHub for a newer full-build APK release.
///
/// # Errors
/// Returns [`UpdateError`] when the GitHub response is unavailable, malformed, or unsafe.
pub async fn check(
    config: &UpdateConfig,
    current_version: &str,
) -> Result<UpdateCheck, UpdateError> {
    check_with_timeout(config, current_version, CHECK_TIMEOUT).await
}

/// Checks for an update with a caller-selected timeout.
pub(crate) async fn check_with_timeout(
    config: &UpdateConfig,
    current_version: &str,
    timeout: Duration,
) -> Result<UpdateCheck, UpdateError> {
    let current = version::parse_current(current_version)?;
    let client = client(config).map_err(|_| UpdateError::Network)?;
    tokio::time::timeout(timeout, release::check(config, current, &client))
        .await
        .map_err(|_| UpdateError::Network)?
}

/// Downloads, verifies, and atomically saves a release APK.
///
/// # Errors
/// Returns [`UpdateError`] when the release, network stream, storage, checksum, or cancellation
/// state fails the update contract.
pub async fn download(
    config: &UpdateConfig,
    release: &Release,
    dir: &std::path::Path,
    progress: &(dyn Fn(u64, u64) + Sync),
    cancel: CancellationToken,
) -> Result<std::path::PathBuf, UpdateError> {
    let client = client(config).map_err(|_| UpdateError::Network)?;
    let request = download::Request {
        config,
        release,
        dir,
        progress,
        cancel,
        client: &client,
    };
    Box::pin(download::run(&request)).await
}

pub(crate) fn client(config: &UpdateConfig) -> Result<Client, reqwest::Error> {
    let local_http = local_http_allowed(config);
    let redirect_local = local_http;
    let builder = ClientBuilder::new()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .redirect(Policy::custom(move |attempt| {
            if attempt.previous().len() > 5 {
                return attempt.stop();
            }
            let allowed = is_allowed_redirect(attempt.url())
                || (redirect_local && is_local_http(attempt.url()));
            if allowed {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }));
    if local_http {
        builder.https_only(false).build()
    } else {
        builder.https_only(true).build()
    }
}

pub(crate) fn is_allowed_redirect(url: &Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    host == "github.com"
        || host == "api.github.com"
        || host.ends_with(".github.com")
        || host.ends_with(".githubusercontent.com")
}

pub(crate) fn valid_asset_url(config: &UpdateConfig, value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    is_allowed_redirect(&url) || (local_http_allowed(config) && is_local_http(&url))
}

fn is_local_http(url: &Url) -> bool {
    url.scheme() == "http" && url.host_str() == Some("127.0.0.1")
}

#[cfg(test)]
fn local_http_allowed(config: &UpdateConfig) -> bool {
    config.allow_local_http
}

#[cfg(not(test))]
fn local_http_allowed(_config: &UpdateConfig) -> bool {
    false
}

#[cfg(test)]
mod cached_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_limits;
#[cfg(test)]
mod tests_network;
#[cfg(test)]
mod tests_progress;
#[cfg(test)]
mod tests_release;
#[cfg(test)]
mod tests_sums;
#[cfg(test)]
mod tests_version;
