//! GitHub sign-in and identity views for the platform layer.

use std::time::{Duration, Instant};

use serde_json::Value;
use tether_vcs::{HttpRequest, HttpResponse};

use crate::git::GitRuntime;
use crate::git_support::now_ms;
use crate::git_types::GitRuntimeError;
use crate::github_token_info::inspect_token;
/// The public part of a GitHub device grant.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DeviceGrantView {
    /// Code shown to the user.
    pub user_code: String,
    /// Browser URL where the code is entered.
    pub verification_uri: String,
    /// Minimum polling interval in seconds.
    pub interval_seconds: u64,
    /// Seconds until the grant expires.
    pub expires_in_seconds: u64,
}
/// One GitHub sign-in poll result.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SignInStateView {
    /// The user has not approved the grant yet.
    Pending,
    /// GitHub asked the client to poll less often.
    SlowDown,
    /// Sign-in succeeded.
    Authorized(String),
    /// The user denied access.
    Denied,
    /// The grant expired.
    Expired,
}

/// The kind of GitHub personal access token in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum GithubTokenKindView {
    /// A classic personal access token.
    Classic,
    /// A fine-grained personal access token.
    FineGrained,
}

/// The saved GitHub account state.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GithubStatusView {
    /// Whether a token is stored.
    pub signed_in: bool,
    /// GitHub login, if known.
    pub login: Option<String>,
    /// GitHub display name, if known.
    pub name: Option<String>,
    /// GitHub email, if known.
    pub email: Option<String>,
    /// Scopes reported by the last token validation.
    pub scopes: Vec<String>,
    /// The kind of the saved token, if one is known.
    pub token_kind: Option<GithubTokenKindView>,
    /// Whether the last repository reach probe succeeded.
    pub can_read_repos: bool,
}

/// One repository shown in the clone picker.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GithubRepoView {
    /// Repository clone URL.
    pub clone_url: String,
    /// Repository default branch.
    pub default_branch: String,
    /// Whether the repository is private.
    pub private: bool,
    /// Repository size in KiB.
    pub size_kb: u64,
}

pub(crate) struct DeviceGrantState {
    pub(crate) device_code: String,
    pub(crate) interval_seconds: u64,
    pub(crate) expires_at: Instant,
}

#[uniffi::export]
impl GitRuntime {
    /// Sets the OAuth application's public client ID.
    ///
    /// # Errors
    /// Returns an error when the client ID is empty or the config lock is poisoned.
    pub fn configure_github(&self, client_id: &str) -> Result<(), GitRuntimeError> {
        if client_id.trim().is_empty() {
            return Err(GitRuntimeError::InvalidInput);
        }
        *self
            .oauth_client_id
            .lock()
            .map_err(|_| GitRuntimeError::Unavailable)? = Some(client_id.to_owned());
        Ok(())
    }

    /// Starts the GitHub OAuth device flow.
    ///
    /// # Errors
    /// Returns an error when the client is not configured or GitHub rejects the request.
    pub fn github_sign_in_start(&self) -> Result<DeviceGrantView, GitRuntimeError> {
        let client_id = self
            .oauth_client_id
            .lock()
            .map_err(|_| GitRuntimeError::Unavailable)?
            .clone()
            .ok_or(GitRuntimeError::GithubNotConfigured)?;
        let response = self.http_form(
            "https://github.com/login/device/code",
            &[("client_id", client_id), ("scope", "repo".into())],
        )?;
        let value: Value =
            serde_json::from_slice(&response).map_err(|_| GitRuntimeError::GithubResponse)?;
        let device_code = field(&value, "device_code")?;
        let user_code = field(&value, "user_code")?;
        let verification_uri = field(&value, "verification_uri")?;
        let expires_in = value
            .get("expires_in")
            .and_then(Value::as_u64)
            .ok_or(GitRuntimeError::GithubResponse)?;
        let interval_seconds = value
            .get("interval")
            .and_then(Value::as_u64)
            .unwrap_or(5)
            .max(5);
        if !(1..=900).contains(&expires_in) {
            return Err(GitRuntimeError::GithubResponse);
        }
        *self
            .device_grant
            .lock()
            .map_err(|_| GitRuntimeError::Unavailable)? = Some(DeviceGrantState {
            device_code,
            interval_seconds,
            expires_at: Instant::now() + Duration::from_secs(expires_in),
        });
        Ok(DeviceGrantView {
            user_code,
            verification_uri,
            interval_seconds,
            expires_in_seconds: expires_in,
        })
    }

    /// Polls the current GitHub OAuth device grant once.
    ///
    /// # Errors
    /// Returns an error when there is no grant or GitHub returns an unusable response.
    pub fn github_sign_in_poll(&self) -> Result<SignInStateView, GitRuntimeError> {
        let mut grant = self
            .device_grant
            .lock()
            .map_err(|_| GitRuntimeError::Unavailable)?;
        let state = grant.as_mut().ok_or(GitRuntimeError::NoDeviceGrant)?;
        if Instant::now() >= state.expires_at {
            *grant = None;
            return Ok(SignInStateView::Expired);
        }
        let client_id = self
            .oauth_client_id
            .lock()
            .map_err(|_| GitRuntimeError::Unavailable)?
            .clone()
            .ok_or(GitRuntimeError::GithubNotConfigured)?;
        let value = self.request_device_token(client_id, &state.device_code)?;
        if let Some(token) = value.get("access_token").and_then(Value::as_str) {
            self.github_sign_in_with_token(token)?;
            let login = self.github_status()?.login.unwrap_or_default();
            *grant = None;
            return Ok(SignInStateView::Authorized(login));
        }
        match value.get("error").and_then(Value::as_str) {
            Some("authorization_pending") => Ok(SignInStateView::Pending),
            Some("slow_down") => {
                state.interval_seconds = state.interval_seconds.saturating_add(5);
                Ok(SignInStateView::SlowDown)
            }
            Some("access_denied") => {
                *grant = None;
                Ok(SignInStateView::Denied)
            }
            Some("expired_token") => {
                *grant = None;
                Ok(SignInStateView::Expired)
            }
            _ => Err(GitRuntimeError::GithubResponse),
        }
    }

    /// Validates and stores a GitHub personal access token.
    ///
    /// # Errors
    /// Returns an error when GitHub rejects the token or its response is unusable.
    pub fn github_sign_in_with_token(&self, token: &str) -> Result<(), GitRuntimeError> {
        if token.trim().is_empty() {
            return Err(GitRuntimeError::InvalidInput);
        }
        let info = inspect_token(self, token)?;
        self.save_github_token(token)?;
        self.save_github_token_info(&info)?;
        self.refresh_identity()
    }

    /// Removes the GitHub token and cached GitHub identity.
    ///
    /// # Errors
    /// Returns an error when storage cannot be updated.
    pub fn github_sign_out(&self) -> Result<(), GitRuntimeError> {
        self.vault
            .delete("github-oauth")
            .map_err(|_| GitRuntimeError::Unavailable)?;
        self.store()?
            .clear_git_identity()
            .map_err(|_| GitRuntimeError::Unavailable)?;
        self.clear_github_token_info()
    }

    /// Reads the saved GitHub account and Git identity state.
    ///
    /// # Errors
    /// Returns an error when storage cannot be read.
    pub fn github_status(&self) -> Result<GithubStatusView, GitRuntimeError> {
        let signed_in = self
            .vault
            .has("github-oauth")
            .map_err(|_| GitRuntimeError::Unavailable)?;
        let identity = self
            .store()?
            .git_identity()
            .map_err(|_| GitRuntimeError::Unavailable)?;
        let info = self.github_token_info()?;
        Ok(GithubStatusView {
            signed_in,
            login: identity.as_ref().and_then(|value| value.login.clone()),
            name: identity.as_ref().map(|value| value.name.clone()),
            email: identity.as_ref().map(|value| value.email.clone()),
            scopes: info
                .as_ref()
                .map_or_else(Vec::new, |value| value.scopes.clone()),
            token_kind: info.as_ref().and_then(|value| match value.kind.as_str() {
                "classic" => Some(GithubTokenKindView::Classic),
                "fine_grained" => Some(GithubTokenKindView::FineGrained),
                _ => None,
            }),
            can_read_repos: info.is_some_and(|value| value.can_read_repos),
        })
    }
}

impl GitRuntime {
    fn request_device_token(
        &self,
        client_id: String,
        device_code: &str,
    ) -> Result<Value, GitRuntimeError> {
        let response = self.http_form(
            "https://github.com/login/oauth/access_token",
            &[
                ("client_id", client_id),
                ("device_code", device_code.to_owned()),
                (
                    "grant_type",
                    "urn:ietf:params:oauth:grant-type:device_code".into(),
                ),
            ],
        )?;
        serde_json::from_slice(&response).map_err(|_| GitRuntimeError::GithubResponse)
    }

    fn save_github_token(&self, token: &str) -> Result<(), GitRuntimeError> {
        self.vault
            .put("github-oauth", token)
            .map_err(|_| GitRuntimeError::Unavailable)
    }

    fn refresh_identity(&self) -> Result<(), GitRuntimeError> {
        let client = self.github_client()?;
        let identity = self
            .runtime
            .block_on(client.current_user())
            .map_err(|_| GitRuntimeError::GithubResponse)?;
        self.store()?
            .set_git_identity(&tether_store::GitIdentity {
                login: Some(identity.login),
                name: identity.name,
                email: identity.email,
                source: "github".into(),
                updated_at: now_ms(),
            })
            .map_err(|_| GitRuntimeError::Unavailable)
    }

    fn http_form(&self, url: &str, fields: &[(&str, String)]) -> Result<Vec<u8>, GitRuntimeError> {
        let body = fields
            .iter()
            .map(|(key, value)| format!("{}={}", form_escape(key), form_escape(value)))
            .collect::<Vec<_>>()
            .join("&")
            .into_bytes();
        let response = self.http_request(HttpRequest {
            method: "POST".into(),
            url: url.into(),
            headers: [
                ("accept".into(), "application/json".into()),
                (
                    "content-type".into(),
                    "application/x-www-form-urlencoded".into(),
                ),
            ]
            .into_iter()
            .collect(),
            body: Some(body),
        })?;
        Ok(response.body)
    }

    pub(crate) fn http_bearer(
        &self,
        url: &str,
        token: &str,
    ) -> Result<(u16, Value, Option<String>), GitRuntimeError> {
        let response = self.http_request(HttpRequest {
            method: "GET".into(),
            url: url.into(),
            headers: [
                ("authorization".into(), format!("Bearer {token}")),
                ("user-agent".into(), "Tether".into()),
            ]
            .into_iter()
            .collect(),
            body: None,
        })?;
        let body = serde_json::from_slice(&response.body).unwrap_or(Value::Null);
        Ok((
            response.status,
            body,
            response.headers.get("x-oauth-scopes").cloned(),
        ))
    }

    fn http_request(&self, request: HttpRequest) -> Result<HttpResponse, GitRuntimeError> {
        let http = self
            .http
            .lock()
            .map_err(|_| GitRuntimeError::Unavailable)?
            .clone();
        self.runtime
            .block_on(http.send(request))
            .map_err(|_| GitRuntimeError::GithubNetwork)
    }
}

fn form_escape(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            other => format!("%{other:02X}").chars().collect(),
        })
        .collect()
}

fn field(value: &Value, key: &str) -> Result<String, GitRuntimeError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
        .ok_or(GitRuntimeError::GithubResponse)
}
