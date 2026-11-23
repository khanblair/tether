//! Downloads a GitHub skill ZIP through a validated codeload URL.

use std::time::Duration;

use reqwest::Client;
use tokio::runtime::Runtime;

use super::BODY_CAP;
use super::github_url::github_zip_url;
use crate::skills::SkillsRuntimeError;

/// A seam for fake ZIP downloads in tests.
pub(super) trait SkillDownloader: Send + Sync {
    /// Downloads a ZIP body for a validated URL.
    fn download(&self, url: &str) -> Result<Vec<u8>, SkillsRuntimeError>;
}

#[derive(Debug, Clone, Copy)]
pub(super) struct GithubDownloader;

impl SkillDownloader for GithubDownloader {
    fn download(&self, url: &str) -> Result<Vec<u8>, SkillsRuntimeError> {
        let download_url = github_zip_url(url)?;
        let client = github_client()?;
        let runtime = Runtime::new().map_err(|_| SkillsRuntimeError::Unavailable)?;
        runtime.block_on(fetch_body(client, download_url))
    }
}

/// Builds the HTTP client. Redirects are followed only between GitHub hosts.
fn github_client() -> Result<Client, SkillsRuntimeError> {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt
                .url()
                .host_str()
                .is_some_and(|host| host == "github.com" || host == "codeload.github.com")
            {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|_| SkillsRuntimeError::Unavailable)
}

/// Streams the response body and rejects anything over the size cap.
async fn fetch_body(client: Client, download_url: String) -> Result<Vec<u8>, SkillsRuntimeError> {
    let mut response = client
        .get(download_url)
        .send()
        .await
        .map_err(|_| SkillsRuntimeError::Download)?
        .error_for_status()
        .map_err(|_| SkillsRuntimeError::Download)?;
    if response
        .content_length()
        .is_some_and(|size| size > BODY_CAP as u64)
    {
        return Err(SkillsRuntimeError::TooLarge);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| SkillsRuntimeError::Download)?
    {
        append_capped(&mut bytes, &chunk)?;
    }
    Ok(bytes)
}

fn append_capped(bytes: &mut Vec<u8>, chunk: &[u8]) -> Result<(), SkillsRuntimeError> {
    if bytes.len().saturating_add(chunk.len()) > BODY_CAP {
        return Err(SkillsRuntimeError::TooLarge);
    }
    bytes.extend_from_slice(chunk);
    Ok(())
}
