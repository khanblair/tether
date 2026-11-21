//! GitHub sign-in and identity views for the platform layer.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::git::{GitRuntime, GitRuntimeError};

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
    pub(crate) user_code: String,
    pub(crate) verification_uri: String,
    pub(crate) interval_seconds: u64,
    pub(crate) expires_at: Instant,
}

#[uniffi::export]
impl GitRuntime {
    /// Sets the OAuth application's public client ID.
    pub fn configure_github(&self, client_id: String) -> Result<(), GitRuntimeError> {
        if client_id.trim().is_empty() {
            return Err(GitRuntimeError::InvalidInput);
        }
        *self
            .oauth_client_id
            .lock()
            .map_err(|_| GitRuntimeError::Unavailable)? = Some(client_id);
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
        *self
            .device_grant
            .lock()
            .map_err(|_| GitRuntimeError::Unavailable)? = Some(DeviceGrantState {
            device_code,
            user_code: user_code.clone(),
            verification_uri: verification_uri.clone(),
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
        let response = self.http_form(
            "https://github.com/login/oauth/access_token",
            &[
                ("client_id", client_id),
                ("device_code", state.device_code.clone()),
                (
                    "grant_type",
                    "urn:ietf:params:oauth:grant-type:device_code".into(),
                ),
            ],
        )?;
        let value: Value =
            serde_json::from_slice(&response).map_err(|_| GitRuntimeError::GithubResponse)?;
        if let Some(token) = value.get("access_token").and_then(Value::as_str) {
            self.save_github_token(token)?;
            self.refresh_identity()?;
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
    /// Returns an error when GitHub rejects the token or it lacks the `repo` scope.
    pub fn github_sign_in_with_token(&self, token: String) -> Result<(), GitRuntimeError> {
        if token.trim().is_empty() {
            return Err(GitRuntimeError::InvalidInput);
        }
        let response = self.http_bearer("https://api.github.com/user", &token)?;
        if response.0 == 401 {
            return Err(GitRuntimeError::GithubCredentials);
        }
        if !(200..300).contains(&response.0) {
            return Err(GitRuntimeError::GithubResponse);
        }
        let scopes = response
            .2
            .get("x-oauth-scopes")
            .map(|text| {
                text.split(',')
                    .map(str::trim)
                    .filter(|scope| !scope.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !scopes.iter().any(|scope| scope == "repo") {
            return Err(GitRuntimeError::MissingScope {
                scope: "repo".into(),
            });
        }
        self.save_github_token(&token)?;
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
            .map_err(|_| GitRuntimeError::Unavailable)
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
        Ok(GithubStatusView {
            signed_in,
            login: identity.as_ref().and_then(|value| value.login.clone()),
            name: identity.as_ref().map(|value| value.name.clone()),
            email: identity.as_ref().map(|value| value.email.clone()),
            scopes: Vec::new(),
        })
    }
}

impl GitRuntime {
    fn save_github_token(&self, token: &str) -> Result<(), GitRuntimeError> {
        self.vault
            .put("github-oauth", token)
            .map_err(|_| GitRuntimeError::Unavailable)
    }

    fn refresh_identity(&self) -> Result<(), GitRuntimeError> {
        let (status, body, _) =
            self.http_bearer("https://api.github.com/user", &self.token_text()?)?;
        if !(200..300).contains(&status) {
            return Err(GitRuntimeError::GithubCredentials);
        }
        let login = field(&body, "login")?;
        let name = body
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(&login)
            .to_owned();
        let email = body
            .get("email")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        self.store()?
            .set_git_identity(&tether_store::GitIdentity {
                login: Some(login),
                name,
                email,
                source: "github".into(),
                updated_at: now_ms(),
            })
            .map_err(|_| GitRuntimeError::Unavailable)
    }

    fn token_text(&self) -> Result<String, GitRuntimeError> {
        self.vault
            .reveal("github-oauth")
            .map_err(|_| GitRuntimeError::Unavailable)?
            .ok_or(GitRuntimeError::NotSignedIn)
    }

    fn http_form(
        &self,
        url: &str,
        fields: &[(String, String)],
    ) -> Result<Vec<u8>, GitRuntimeError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| GitRuntimeError::GithubNetwork)?;
        let response = self
            .runtime
            .block_on(
                client
                    .post(url)
                    .header("accept", "application/json")
                    .form(fields)
                    .send(),
            )
            .map_err(|_| GitRuntimeError::GithubNetwork)?;
        self.runtime
            .block_on(response.bytes())
            .map(|bytes| bytes.to_vec())
            .map_err(|_| GitRuntimeError::GithubNetwork)
    }

    fn http_bearer(
        &self,
        url: &str,
        token: &str,
    ) -> Result<(u16, Value, Option<String>), GitRuntimeError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| GitRuntimeError::GithubNetwork)?;
        let response = self
            .runtime
            .block_on(
                client
                    .get(url)
                    .bearer_auth(token)
                    .header("user-agent", "Tether")
                    .send(),
            )
            .map_err(|_| GitRuntimeError::GithubNetwork)?;
        let status = response.status().as_u16();
        let scopes = response
            .headers()
            .get("x-oauth-scopes")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = self
            .runtime
            .block_on(response.json())
            .map_err(|_| GitRuntimeError::GithubResponse)?;
        Ok((status, body, scopes))
    }
}

fn field(value: &Value, key: &str) -> Result<String, GitRuntimeError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
        .ok_or(GitRuntimeError::GithubResponse)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}
