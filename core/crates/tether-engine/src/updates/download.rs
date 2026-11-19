//! Bounded APK download, hashing, and storage operations.

use std::path::{Path, PathBuf};

use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

use super::sums::digest_for;
use super::{Asset, MAX_APK_BYTES, Release, UpdateConfig, UpdateError, valid_asset_url};

const MAX_SUMS_BYTES: usize = 64 * 1024;
const APK_PREFIX: &str = "tether-full-v";

pub(crate) struct Request<'a> {
    pub(crate) config: &'a UpdateConfig,
    pub(crate) release: &'a Release,
    pub(crate) dir: &'a Path,
    pub(crate) progress: &'a (dyn Fn(u64, u64) + Sync),
    pub(crate) cancel: CancellationToken,
    pub(crate) client: &'a Client,
}

pub(crate) async fn run(request: &Request<'_>) -> Result<PathBuf, UpdateError> {
    validate_release(request.config, request.release)?;
    fs::create_dir_all(request.dir)
        .await
        .map_err(|_| UpdateError::Storage)?;
    let final_path = request.dir.join(&request.release.apk.name);
    let part_path = request
        .dir
        .join(format!("{}.part", request.release.apk.name));
    let result = run_inner(request, &final_path, &part_path).await;
    if result.is_err() {
        cleanup_part(&part_path).await?;
    }
    result.map(|()| final_path)
}

async fn run_inner(
    request: &Request<'_>,
    final_path: &Path,
    part_path: &Path,
) -> Result<(), UpdateError> {
    remove_stale_files(request.dir, &request.release.apk.name).await?;
    if request.release.apk.size > MAX_APK_BYTES {
        return Err(UpdateError::TooLarge);
    }
    if request.cancel.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    let sums = fetch_sums(request.client, &request.release.sums, &request.cancel).await?;
    let expected = digest_for(&sums, &request.release.apk.name)?;
    if Box::pin(file_matches(final_path, expected, &request.cancel)).await? {
        (request.progress)(0, request.release.apk.size);
        (request.progress)(request.release.apk.size, request.release.apk.size);
        return Ok(());
    }
    download_apk(request, part_path, expected).await?;
    fs::rename(part_path, final_path)
        .await
        .map_err(|_| UpdateError::Storage)
}

fn validate_release(config: &UpdateConfig, release: &Release) -> Result<(), UpdateError> {
    let expected_name = format!("{}v{}.apk", config.asset_prefix, release.version);
    if release.apk.name != expected_name
        || !is_apk_name(&release.apk.name)
        || release.sums.name != "SHA256SUMS"
        || !valid_asset_url(config, &release.apk.url)
        || !valid_asset_url(config, &release.sums.url)
    {
        return Err(UpdateError::Invalid);
    }
    Ok(())
}

async fn remove_stale_files(dir: &Path, current_name: &str) -> Result<(), UpdateError> {
    let mut entries = fs::read_dir(dir).await.map_err(|_| UpdateError::Storage)?;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|_| UpdateError::Storage)?
    {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let matching = is_apk_name(name) || name.strip_suffix(".part").is_some_and(is_apk_name);
        if !matching || name == current_name || name == format!("{current_name}.part") {
            continue;
        }
        let is_file = entry
            .file_type()
            .await
            .map_err(|_| UpdateError::Storage)?
            .is_file();
        if is_file {
            fs::remove_file(dir.join(name))
                .await
                .map_err(|_| UpdateError::Storage)?;
        }
    }
    Ok(())
}

fn is_apk_name(name: &str) -> bool {
    let Some(version) = name
        .strip_prefix(APK_PREFIX)
        .and_then(|value| value.strip_suffix(".apk"))
    else {
        return false;
    };
    let mut parts = version.split('.');
    parts.clone().count() == 3
        && parts.all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

async fn fetch_sums(
    client: &Client,
    asset: &Asset,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, UpdateError> {
    let response = send_asset(client, &asset.url, cancel).await?;
    read_bounded(response, MAX_SUMS_BYTES, cancel).await
}

async fn send_asset(
    client: &Client,
    url: &str,
    cancel: &CancellationToken,
) -> Result<reqwest::Response, UpdateError> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(UpdateError::Cancelled),
        response = client.get(url).send() => {
            let response = response.map_err(|_| UpdateError::Network)?;
            if response.status().is_success() { Ok(response) } else { Err(UpdateError::Network) }
        }
    }
}

async fn read_bounded(
    mut response: reqwest::Response,
    limit: usize,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, UpdateError> {
    let mut body = Vec::new();
    while let Some(chunk) = next_chunk(&mut response, cancel).await? {
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(UpdateError::Invalid);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

async fn next_chunk(
    response: &mut reqwest::Response,
    cancel: &CancellationToken,
) -> Result<Option<Vec<u8>>, UpdateError> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(UpdateError::Cancelled),
        chunk = response.chunk() => chunk.map(|item| item.map(|bytes| bytes.to_vec())).map_err(|_| UpdateError::Network),
    }
}

async fn file_matches(
    path: &Path,
    expected: [u8; 32],
    cancel: &CancellationToken,
) -> Result<bool, UpdateError> {
    let mut file = match File::open(path).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(UpdateError::Storage),
    };
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        if cancel.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        let count = file
            .read(&mut buffer)
            .await
            .map_err(|_| UpdateError::Storage)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().as_slice() == expected)
}

async fn download_apk(
    request: &Request<'_>,
    part_path: &Path,
    expected: [u8; 32],
) -> Result<(), UpdateError> {
    let mut response =
        send_asset(request.client, &request.release.apk.url, &request.cancel).await?;
    if response
        .content_length()
        .is_some_and(|length| length != request.release.apk.size)
    {
        return Err(UpdateError::Invalid);
    }
    let mut file = File::create(part_path)
        .await
        .map_err(|_| UpdateError::Storage)?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0_u64;
    let mut last_percent = 0_u64;
    (request.progress)(0, request.release.apk.size);
    while let Some(chunk) = next_chunk_response(&mut response, &request.cancel).await? {
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        if downloaded > request.release.apk.size {
            return Err(UpdateError::TooLarge);
        }
        file.write_all(&chunk)
            .await
            .map_err(|_| UpdateError::Storage)?;
        hasher.update(&chunk);
        report_progress(
            request.progress,
            downloaded,
            request.release.apk.size,
            &mut last_percent,
        );
    }
    if downloaded != request.release.apk.size {
        return Err(UpdateError::Invalid);
    }
    if request.cancel.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    file.flush().await.map_err(|_| UpdateError::Storage)?;
    if hasher.finalize().as_slice() != expected {
        return Err(UpdateError::Checksum);
    }
    (request.progress)(request.release.apk.size, request.release.apk.size);
    Ok(())
}

async fn next_chunk_response(
    response: &mut reqwest::Response,
    cancel: &CancellationToken,
) -> Result<Option<Vec<u8>>, UpdateError> {
    next_chunk(response, cancel).await
}

fn report_progress(
    progress: &(dyn Fn(u64, u64) + Sync),
    downloaded: u64,
    total: u64,
    last_percent: &mut u64,
) {
    let percent = downloaded
        .saturating_mul(100)
        .checked_div(total)
        .unwrap_or(0);
    if percent > *last_percent && percent < 100 {
        *last_percent = percent;
        progress(downloaded, total);
    }
}

async fn cleanup_part(path: &Path) -> Result<(), UpdateError> {
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(UpdateError::Storage),
    }
}
