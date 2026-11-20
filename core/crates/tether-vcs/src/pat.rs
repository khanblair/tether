//! GitHub personal access token validation before wrapped storage.

use std::time::Duration;

use reqwest::header::{ACCEPT, HeaderMap, USER_AGENT};
use serde_json::Value;
use tether_secrets::{Secret, SecretStore};
use thiserror::Error;

use crate::oauth::token_ref;

const USER_URL: &str = "https://api.github.com/user";
const RESPONSE_LIMIT: usize = 32 * 1024;

/// A validated account and the scopes GitHub reported, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatInfo {
    /// GitHub login of the token owner.
    pub login: String,
    /// Classic OAuth scopes. `None` means GitHub did not report scopes.
    pub scopes: Option<Vec<String>>,
}

/// PAT validation failed. The token is never included in an error.
#[derive(Debug, Error)]
pub enum PatError {
    /// Token text is empty or not UTF-8.
    #[error("GitHub token is empty or invalid text")]
    InvalidInput,
    /// GitHub rejected the token.
    #[error("GitHub rejected the token")]
    Rejected,
    /// The request or response failed.
    #[error("GitHub token check failed")]
    Network,
    /// GitHub sent an invalid response.
    #[error("GitHub account response was invalid")]
    InvalidResponse,
    /// The validated token could not be wrapped.
    #[error("GitHub token could not be stored")]
    Storage,
}

/// Validates a PAT with GitHub and stores it in the same wrapped slot as OAuth.
pub struct GithubPatAuth {
    client: reqwest::Client,
    secrets: SecretStore,
}

impl GithubPatAuth {
    /// Creates the validator with redirects disabled.
    ///
    /// # Errors
    /// Returns a client setup failure.
    pub fn new(secrets: SecretStore) -> Result<Self, PatError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| PatError::Network)?;
        Ok(Self { client, secrets })
    }

    /// Checks `/user` and stores the token only after a valid account response.
    ///
    /// # Errors
    /// Returns invalid input, rejection, network, protocol, or storage errors.
    pub async fn validate_and_store(&self, token: Secret) -> Result<PatInfo, PatError> {
        let value = token.expose_text().map_err(|_| PatError::InvalidInput)?;
        if value.trim().is_empty() {
            return Err(PatError::InvalidInput);
        }
        let response = self
            .client
            .get(USER_URL)
            .header(ACCEPT, "application/vnd.github+json")
            .header(USER_AGENT, "Tether")
            .bearer_auth(value)
            .send()
            .await
            .map_err(|_| PatError::Network)?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(PatError::Rejected);
        }
        if !response.status().is_success() {
            return Err(PatError::Network);
        }
        let scopes = scopes_from_headers(response.headers());
        let body = response.bytes().await.map_err(|_| PatError::Network)?;
        if body.len() > RESPONSE_LIMIT {
            return Err(PatError::InvalidResponse);
        }
        let info = parse_account(&body, scopes)?;
        self.secrets
            .put(&token_ref(), &token)
            .map_err(|_| PatError::Storage)?;
        Ok(info)
    }
}

fn scopes_from_headers(headers: &HeaderMap) -> Option<Vec<String>> {
    headers
        .get("x-oauth-scopes")
        .and_then(|value| value.to_str().ok())
        .map(|text| {
            text.split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect()
        })
}

fn parse_account(bytes: &[u8], scopes: Option<Vec<String>>) -> Result<PatInfo, PatError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| PatError::InvalidResponse)?;
    let login = value
        .get("login")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or(PatError::InvalidResponse)?;
    Ok(PatInfo {
        login: login.to_owned(),
        scopes,
    })
}

#[cfg(test)]
mod tests;
