//! Repository cloning through gitoxide.

use std::fs;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use tether_secrets::Secret;
use thiserror::Error;

use crate::{CancellationToken, Repo};

/// A clone progress phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClonePhase {
    /// Resolving the remote.
    Resolving,
    /// Receiving pack data.
    Receiving,
    /// Checking out files.
    CheckingOut,
}

/// Receives throttled clone progress updates.
pub trait CloneProgress: Send + Sync {
    /// Reports a phase and optional byte counts.
    fn update(&self, phase: ClonePhase, received: u64, total: Option<u64>);
}

/// A clone request.
#[derive(Debug)]
pub struct CloneRequest {
    /// Remote URL.
    pub url: String,
    /// Destination folder.
    pub dest: PathBuf,
    /// Optional shallow depth.
    pub depth: Option<u32>,
    /// Optional remote branch to check out.
    pub branch: Option<String>,
    /// Optional GitHub token.
    pub credentials: Option<Secret>,
}

/// Clone failures. Credentials are never stored in this error.
#[derive(Debug, Error)]
pub enum CloneError {
    /// The URL scheme or URL authority is not permitted.
    #[error("clone URL is invalid")]
    InvalidUrl,
    /// The destination is not empty.
    #[error("clone destination is not empty")]
    DestinationNotEmpty,
    /// The transfer was cancelled.
    #[error("clone cancelled")]
    Cancelled,
    /// The clone failed.
    #[error("clone failed")]
    Failed,
}

/// Clones a repository into an atomic temporary sibling and returns the opened result.
///
/// # Errors
/// Returns an error for unsupported URLs, non-empty destinations, cancellation, or transfer failures.
pub fn clone(
    request: CloneRequest,
    progress: &dyn CloneProgress,
    cancel: &CancellationToken,
) -> Result<Repo, CloneError> {
    let url = validate_url(&request.url)?;
    validate_destination(&request.dest)?;
    if cancel.is_cancelled() {
        return Err(CloneError::Cancelled);
    }
    progress.update(ClonePhase::Resolving, 0, None);
    let temp = temporary_destination(&request.dest);
    let atomic = cancel.atomic();
    let result = clone_inner(&url, &request, &temp, progress, &atomic);
    match result {
        Ok(()) if !cancel.is_cancelled() => {
            fs::rename(&temp, &request.dest).map_err(|_| CloneError::Failed)?;
            progress.update(ClonePhase::CheckingOut, 1, Some(1));
            Repo::open(request.dest).map_err(|_| CloneError::Failed)
        }
        Ok(()) | Err(CloneError::Cancelled) => {
            remove_temp(&temp);
            Err(CloneError::Cancelled)
        }
        Err(error) => {
            remove_temp(&temp);
            Err(error)
        }
    }
}

fn clone_inner(
    url: &str,
    request: &CloneRequest,
    temp: &Path,
    progress: &dyn CloneProgress,
    cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), CloneError> {
    let mut prep = gix::clone::PrepareFetch::new(
        url,
        temp,
        gix::create::Kind::WithWorktree,
        gix::create::Options {
            destination_must_be_empty: Some(true),
            ..Default::default()
        },
        gix::open::Options::default(),
    )
    .map_err(|_| CloneError::Failed)?;
    if let Some(depth) = request.depth {
        let depth = NonZeroU32::new(depth).ok_or(CloneError::InvalidUrl)?;
        prep = prep.with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(depth));
    }
    if let Some(branch) = &request.branch {
        prep = prep
            .with_ref_name(Some(branch.as_str()))
            .map_err(|_| CloneError::Failed)?;
    }
    if let Some(secret) = &request.credentials {
        let text = secret
            .expose_text()
            .map_err(|_| CloneError::Failed)?
            .to_owned();
        prep = prep.configure_connection(move |connection| {
            let text = text.clone();
            connection.set_credentials(move |action| credential_action(&action, &text));
            Ok(())
        });
    }
    progress.update(ClonePhase::Receiving, 0, None);
    let mut checkout = prep
        .fetch_then_checkout(gix::progress::Discard, cancel)
        .map_err(|_| clone_error(cancel))?
        .0;
    if cancel.load(Ordering::Acquire) {
        return Err(CloneError::Cancelled);
    }
    progress.update(ClonePhase::CheckingOut, 1, Some(1));
    let _ = checkout
        .main_worktree(gix::progress::Discard, cancel)
        .map_err(|_| clone_error(cancel))?;
    Ok(())
}

fn clone_error(cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>) -> CloneError {
    if cancel.load(Ordering::Acquire) {
        CloneError::Cancelled
    } else {
        CloneError::Failed
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "gix credentials requires a fallible callback"
)]
fn credential_action(
    action: &gix::credentials::helper::Action,
    token: &str,
) -> gix::Result<Option<gix::credentials::protocol::Outcome>> {
    match action {
        gix::credentials::helper::Action::Get(context) => {
            Ok(Some(gix::credentials::protocol::Outcome {
                identity: gix::sec::identity::Account {
                    username: "x-access-token".to_owned(),
                    password: token.to_owned(),
                    oauth_refresh_token: None,
                },
                next: context.clone().into(),
            }))
        }
        _ => Ok(None),
    }
}

fn validate_url(url: &str) -> Result<String, CloneError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| CloneError::InvalidUrl)?;
    if parsed.username() != "" || parsed.password().is_some() {
        return Err(CloneError::InvalidUrl);
    }
    match parsed.scheme() {
        "https" | "file" => Ok(parsed.to_string()),
        _ => Err(CloneError::InvalidUrl),
    }
}
fn validate_destination(dest: &Path) -> Result<(), CloneError> {
    if dest.exists()
        && fs::read_dir(dest)
            .map_err(|_| CloneError::DestinationNotEmpty)?
            .next()
            .is_some()
    {
        return Err(CloneError::DestinationNotEmpty);
    }
    Ok(())
}
fn temporary_destination(dest: &Path) -> PathBuf {
    let name = dest
        .file_name()
        .map(|v| v.to_string_lossy())
        .unwrap_or_default();
    dest.with_file_name(format!(".{name}.tether-clone-{}", std::process::id()))
}
fn remove_temp(path: &Path) {
    if fs::remove_dir_all(path).is_err() {}
}
