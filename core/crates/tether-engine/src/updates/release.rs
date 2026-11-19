//! GitHub release metadata fetching and validation.

use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use reqwest::{Client, StatusCode};
use serde::Deserialize;

use super::version::{self, Version};
use super::{Asset, Release, UpdateCheck, UpdateConfig, UpdateError, valid_asset_url};

const MAX_RELEASE_BODY_BYTES: usize = 1024 * 1024;
const MAX_NOTES_CHARS: usize = 8000;

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    published_at: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

pub(crate) async fn check(
    config: &UpdateConfig,
    current: Version,
    client: &Client,
) -> Result<UpdateCheck, UpdateError> {
    let response = send_request(config, client).await?;
    let release = read_release(response).await?;
    if release.draft || release.prerelease {
        return Err(UpdateError::Unavailable);
    }
    let latest = version::parse_tag(&release.tag_name)?;
    if latest <= current {
        return Ok(UpdateCheck::UpToDate {
            current: current.as_bare(),
            latest: latest.as_bare(),
        });
    }
    newer_release(config, release, latest)
}

async fn send_request(
    config: &UpdateConfig,
    client: &Client,
) -> Result<reqwest::Response, UpdateError> {
    let url = format!(
        "{}/repos/{}/releases/latest",
        config.api_base.trim_end_matches('/'),
        config.repo
    );
    let response = client
        .get(url)
        .headers(request_headers())
        .send()
        .await
        .map_err(|_| UpdateError::Network)?;
    match response.status() {
        StatusCode::NOT_FOUND => Err(UpdateError::Unavailable),
        status if !status.is_success() => Err(UpdateError::Network),
        _ => Ok(response),
    }
}

fn request_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static("2022-11-28"),
    );
    headers.insert(USER_AGENT, HeaderValue::from_static("tether-updater"));
    headers
}

async fn read_release(mut response: reqwest::Response) -> Result<GitHubRelease, UpdateError> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| UpdateError::Network)? {
        if body.len().saturating_add(chunk.len()) > MAX_RELEASE_BODY_BYTES {
            return Err(UpdateError::Invalid);
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| UpdateError::Invalid)
}

fn newer_release(
    config: &UpdateConfig,
    source: GitHubRelease,
    version: Version,
) -> Result<UpdateCheck, UpdateError> {
    let apk_name = format!("{}v{}.apk", config.asset_prefix, version.as_bare());
    let apk = source
        .assets
        .iter()
        .find(|asset| asset.name == apk_name)
        .ok_or(UpdateError::Unavailable)?;
    let sums = source
        .assets
        .iter()
        .find(|asset| asset.name == "SHA256SUMS")
        .ok_or(UpdateError::Unavailable)?;
    if !valid_asset_url(config, &apk.browser_download_url)
        || !valid_asset_url(config, &sums.browser_download_url)
    {
        return Err(UpdateError::Invalid);
    }
    Ok(UpdateCheck::Available(Release {
        version: version.as_bare(),
        tag: source.tag_name,
        notes: trim_notes(source.body.as_deref().unwrap_or_default()),
        published_at: source.published_at,
        apk: Asset {
            name: apk.name.clone(),
            url: apk.browser_download_url.clone(),
            size: apk.size,
        },
        sums: Asset {
            name: sums.name.clone(),
            url: sums.browser_download_url.clone(),
            size: sums.size,
        },
    }))
}

fn trim_notes(value: &str) -> String {
    value.chars().take(MAX_NOTES_CHARS).collect()
}
