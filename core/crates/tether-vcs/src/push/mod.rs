//! Smart HTTP receive-pack support.

use std::collections::{BTreeMap, BTreeSet};

use tether_secrets::Secret;
use thiserror::Error;

mod http;

use crate::repo::validate_branch_name;
use crate::{CancellationToken, Repo};

const ZERO: &str = "0000000000000000000000000000000000000000";

/// A push transport seam used by tests and by the engine.
pub trait Transport: Send + Sync {
    /// Gets smart-HTTP advertisement bytes.
    ///
    /// # Errors
    /// Returns a transport error when the request fails.
    fn get(&self, url: &str, headers: BTreeMap<String, String>) -> Result<Vec<u8>, TransportError>;
    /// Posts receive-pack bytes.
    ///
    /// # Errors
    /// Returns a transport error when the request fails.
    fn post(&self, url: &str, content_type: &str, body: Vec<u8>)
    -> Result<Vec<u8>, TransportError>;
}

/// A transport failure without credential data.
#[derive(Debug, Error)]
#[error("push transport failed")]
pub struct TransportError;

/// A production reqwest transport for GitHub smart HTTP.
pub struct ReqwestTransport {
    token: Secret,
}

impl std::fmt::Debug for ReqwestTransport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ReqwestTransport")
    }
}

impl ReqwestTransport {
    /// Creates a transport with a token held outside request URLs.
    ///
    /// # Errors
    /// Returns an error when reqwest cannot build its client.
    pub fn new(token: Secret) -> Result<Self, TransportError> {
        let _ = token.expose_text().map_err(|_| TransportError)?;
        Ok(Self { token })
    }
}

impl Transport for ReqwestTransport {
    /// Gets a receive-pack advertisement.
    fn get(&self, url: &str, headers: BTreeMap<String, String>) -> Result<Vec<u8>, TransportError> {
        http::send(http::Request {
            url,
            token: &self.token,
            headers,
            post: None,
        })
    }

    /// Posts receive-pack data.
    fn post(
        &self,
        url: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, TransportError> {
        http::send(http::Request {
            url,
            token: &self.token,
            headers: BTreeMap::new(),
            post: Some((content_type, body)),
        })
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        output.push(ALPHABET[(first >> 2) as usize] as char);
        output.push(ALPHABET[((first & 0x03) << 4 | second >> 4) as usize] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[((second & 0x0f) << 2 | third >> 6) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[(third & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    output
}

/// A requested remote ref update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefUpdate {
    /// Local source ref or object name. `None` deletes the destination.
    pub src: Option<String>,
    /// Full remote ref name.
    pub dst: String,
    /// Allow a non-fast-forward update.
    pub force: bool,
}

/// A push request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRequest {
    /// Remote smart-HTTP URL without query parameters.
    pub remote_url: String,
    /// Requested ref updates.
    pub updates: Vec<RefUpdate>,
}

/// Progress phases emitted by a push.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushPhase {
    /// Resolving remote refs.
    Advertising,
    /// Constructing the pack.
    Packing,
    /// Uploading the request.
    Sending,
    /// Reading the remote report.
    Finishing,
}

/// Receives throttled push progress.
pub trait PushProgress: Send + Sync {
    /// Reports a phase and byte counts.
    fn update(&self, phase: PushPhase, current: u64, total: Option<u64>);
}

/// The result of one remote ref update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefResult {
    /// Remote ref name.
    pub dst: String,
    /// Whether the remote accepted the update.
    pub accepted: bool,
}

/// A completed push report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushReport {
    /// Per-ref results.
    pub updates: Vec<RefResult>,
}

/// A risk used by the engine approval guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PushRisk {
    /// The default branch is changed.
    DefaultBranch,
    /// The update is forced.
    Force,
    /// A remote ref is deleted.
    DeleteRemoteRef,
    /// A new remote branch is created.
    NewBranch,
    /// The update is a fast-forward.
    FastForward,
}

/// A typed push failure.
#[derive(Debug, Error)]
pub enum PushError {
    /// The remote transport failed.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// A request was malformed or a ref was forbidden.
    #[error("push request is invalid: {0}")]
    InvalidRequest(String),
    /// The source ref was not found locally.
    #[error("push source ref was not found")]
    SourceNotFound,
    /// The update would not fast-forward.
    #[error("push is not a fast-forward")]
    NonFastForward,
    /// The remote rejected an update.
    #[error("remote rejected {dst}: {reason}")]
    Rejected {
        /// Rejected remote ref.
        dst: String,
        /// Safe remote reason.
        reason: String,
    },
    /// The operation was cancelled.
    #[error("push cancelled")]
    Cancelled,
    /// The protocol response was invalid.
    #[error("push protocol response is invalid")]
    Protocol,
}

/// Pushes local objects through smart HTTP receive-pack.
///
/// # Errors
/// Returns a typed transport, validation, ancestry, cancellation, or remote rejection error.
pub fn push(
    repo: &Repo,
    request: &PushRequest,
    transport: &dyn Transport,
    progress: &dyn PushProgress,
    cancel: &CancellationToken,
) -> Result<PushReport, PushError> {
    if cancel.is_cancelled() {
        return Err(PushError::Cancelled);
    }
    let advertised = advertise(request, transport, progress)?;
    preflight(repo, request, &advertised, cancel)?;
    let body = build_request(repo, request, &advertised, progress, cancel)?;
    if cancel.is_cancelled() {
        return Err(PushError::Cancelled);
    }
    progress.update(
        PushPhase::Sending,
        body.len() as u64,
        Some(body.len() as u64),
    );
    let response = if body.is_empty() {
        Vec::new()
    } else {
        transport.post(
            &format!("{}/git-receive-pack", receive_url(&request.remote_url)),
            "application/x-git-receive-pack-request",
            body,
        )?
    };
    parse_report(&response, request)
}

/// Classifies push updates without contacting the network.
#[must_use]
pub fn classify_push(repo: &Repo, request: &PushRequest, default_branch: &str) -> Vec<PushRisk> {
    let mut risks = BTreeSet::new();
    for update in &request.updates {
        if update.dst == format!("refs/heads/{default_branch}") {
            risks.insert(PushRisk::DefaultBranch);
        }
        if update.force {
            risks.insert(PushRisk::Force);
        }
        if update.src.is_none() {
            risks.insert(PushRisk::DeleteRemoteRef);
        } else if repo_ref_exists(repo, update.dst.as_str()).is_err() {
            risks.insert(PushRisk::NewBranch);
        } else {
            risks.insert(PushRisk::FastForward);
        }
    }
    risks.into_iter().collect()
}

mod protocol;
use protocol::{advertise, build_request, parse_report, preflight, receive_url, repo_ref_exists};
