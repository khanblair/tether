//! GitHub OAuth device flow. The token is wrapped by the existing secret store.

use std::time::{Duration, Instant};

use reqwest::header::ACCEPT;
use serde_json::Value;
use tether_secrets::{Secret, SecretRef, SecretStore};
use thiserror::Error;

const DEVICE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const RESPONSE_LIMIT: usize = 16 * 1024;

/// The public code shown to the user and the private device code used for polling.
pub struct DeviceGrant {
    /// Code to type in the browser.
    pub user_code: String,
    /// Browser URL where the user approves access.
    pub verification_uri: String,
    /// Minimum seconds between polls.
    pub interval_seconds: u64,
    /// When the grant expires.
    pub expires_at: Instant,
    device_code: String,
}

/// One poll result. The access token is stored before `Authorized` is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollState {
    /// Approval is still pending.
    Pending,
    /// GitHub asked for slower polling; the grant interval increases by five seconds.
    SlowDown,
    /// The token was saved in wrapped storage.
    Authorized,
}

/// Authentication failure. Token and device codes never appear in messages.
#[derive(Debug, Error)]
pub enum OAuthError {
    /// The OAuth app identifier is absent.
    #[error("GitHub OAuth client ID is missing")]
    MissingClientId,
    /// The network request failed.
    #[error("GitHub authentication request failed")]
    Network,
    /// GitHub sent a malformed or oversized response.
    #[error("GitHub authentication response was invalid")]
    InvalidResponse,
    /// The device code expired.
    #[error("GitHub device code expired")]
    Expired,
    /// The user denied the request.
    #[error("GitHub authorization was denied")]
    Denied,
    /// Wrapped token storage failed.
    #[error("GitHub token could not be stored")]
    Storage,
}

/// Runs the GitHub device flow with a configured OAuth app client ID.
pub struct GithubDeviceAuth {
    client: reqwest::Client,
    client_id: String,
    scope: String,
    secrets: SecretStore,
}

impl GithubDeviceAuth {
    /// Creates a flow with HTTPS redirects disabled and a short request timeout.
    ///
    /// # Errors
    /// Returns a missing client ID or HTTP client setup failure.
    pub fn new(client_id: &str, scope: &str, secrets: SecretStore) -> Result<Self, OAuthError> {
        if client_id.trim().is_empty() {
            return Err(OAuthError::MissingClientId);
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| OAuthError::Network)?;
        Ok(Self {
            client,
            client_id: client_id.to_owned(),
            scope: scope.to_owned(),
            secrets,
        })
    }

    /// Requests a new device code and user-facing approval URL.
    ///
    /// # Errors
    /// Returns network or protocol failures.
    pub async fn start(&self) -> Result<DeviceGrant, OAuthError> {
        let response = self
            .client
            .post(DEVICE_URL)
            .header(ACCEPT, "application/json")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("scope", self.scope.as_str()),
            ])
            .send()
            .await
            .map_err(|_| OAuthError::Network)?;
        let body = response.bytes().await.map_err(|_| OAuthError::Network)?;
        if body.len() > RESPONSE_LIMIT {
            return Err(OAuthError::InvalidResponse);
        }
        parse_start(&body)
    }

    /// Polls once. The caller waits at least `interval_seconds` between calls.
    ///
    /// # Errors
    /// Returns expiry, denial, network, protocol, or wrapped storage failures.
    pub async fn poll(&self, grant: &mut DeviceGrant) -> Result<PollState, OAuthError> {
        if Instant::now() >= grant.expires_at {
            return Err(OAuthError::Expired);
        }
        let response = self
            .client
            .post(TOKEN_URL)
            .header(ACCEPT, "application/json")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("device_code", grant.device_code.as_str()),
                ("grant_type", GRANT_TYPE),
            ])
            .send()
            .await
            .map_err(|_| OAuthError::Network)?;
        let body = response.bytes().await.map_err(|_| OAuthError::Network)?;
        if body.len() > RESPONSE_LIMIT {
            return Err(OAuthError::InvalidResponse);
        }
        match parse_poll(&body)? {
            TokenPoll::Pending => Ok(PollState::Pending),
            TokenPoll::SlowDown => {
                grant.interval_seconds = grant.interval_seconds.saturating_add(5);
                Ok(PollState::SlowDown)
            }
            TokenPoll::Token(token) => {
                let reference = token_ref();
                self.secrets
                    .put(&reference, &Secret::new(token.into_bytes()))
                    .map_err(|_| OAuthError::Storage)?;
                Ok(PollState::Authorized)
            }
        }
    }

    /// Reads the wrapped token for a GitHub request, without returning it to the UI.
    ///
    /// # Errors
    /// Returns wrapped storage failures.
    pub fn token(&self) -> Result<Option<Secret>, OAuthError> {
        self.secrets
            .get(&token_ref())
            .map_err(|_| OAuthError::Storage)
    }
}

pub(crate) fn token_ref() -> SecretRef {
    SecretRef::new("github-oauth").expect("fixed reference")
}

fn parse_start(bytes: &[u8]) -> Result<DeviceGrant, OAuthError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| OAuthError::InvalidResponse)?;
    let field = |name| {
        value
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(OAuthError::InvalidResponse)
    };
    let device_code = field("device_code")?;
    let user_code = field("user_code")?;
    let verification_uri = field("verification_uri")?;
    if device_code.is_empty()
        || user_code.is_empty()
        || verification_uri != "https://github.com/login/device"
    {
        return Err(OAuthError::InvalidResponse);
    }
    let expires_in = value
        .get("expires_in")
        .and_then(Value::as_u64)
        .filter(|seconds| (1..=900).contains(seconds))
        .ok_or(OAuthError::InvalidResponse)?;
    let interval_seconds = value
        .get("interval")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .max(5);
    Ok(DeviceGrant {
        user_code,
        verification_uri,
        interval_seconds,
        expires_at: Instant::now() + Duration::from_secs(expires_in),
        device_code,
    })
}

enum TokenPoll {
    Pending,
    SlowDown,
    Token(String),
}

fn parse_poll(bytes: &[u8]) -> Result<TokenPoll, OAuthError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| OAuthError::InvalidResponse)?;
    if let Some(token) = value.get("access_token").and_then(Value::as_str) {
        if token.is_empty() || value.get("token_type").and_then(Value::as_str) != Some("bearer") {
            return Err(OAuthError::InvalidResponse);
        }
        return Ok(TokenPoll::Token(token.to_owned()));
    }
    match value.get("error").and_then(Value::as_str) {
        Some("authorization_pending") => Ok(TokenPoll::Pending),
        Some("slow_down") => Ok(TokenPoll::SlowDown),
        Some("expired_token") => Err(OAuthError::Expired),
        Some("access_denied") => Err(OAuthError::Denied),
        _ => Err(OAuthError::InvalidResponse),
    }
}

#[cfg(test)]
mod tests;
