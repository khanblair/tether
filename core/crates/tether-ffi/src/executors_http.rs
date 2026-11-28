//! HTTPS fetchers shared by executor installers.

use std::io::Write;
use std::sync::Arc;

use reqwest::{Client, Url};
use tether_linux::{RootfsFetcher, RootfsImage, RootfsProgress};
use tether_sandbox::{PackEntry, PackError, PackFetcher, PackFile};
use tokio_util::sync::CancellationToken;

use super::{ExecutorProgress, ExecutorRuntimeError};

const MANIFEST_CAP: usize = 1024 * 1024;
const SIGNATURE_CAP: usize = 256;

pub(super) struct HttpPackFetcher {
    pub(super) client: Client,
    pub(super) manifest_url: String,
    pub(super) progress: Arc<dyn ExecutorProgress>,
}

impl PackFetcher for HttpPackFetcher {
    fn fetch(
        &self,
        pack: &PackEntry,
        file: &PackFile,
        out: &mut dyn Write,
        cancel: &CancellationToken,
    ) -> Result<(), PackError> {
        let url = pack_file_url(&self.manifest_url, pack, file)
            .map_err(|()| PackError::Manifest("pack file URL is invalid".into()))?;
        let download = Download {
            client: &self.client,
            url: &url,
            limit: file.size_bytes,
            cancel,
        };
        download_sync(&download, out, |done, total| {
            self.progress.on_progress(done, total);
        })
        .map_err(|_| PackError::Io {
            id: pack.id.clone(),
            path: file.path.clone(),
        })
    }
}

pub(super) struct HttpRootfsFetcher {
    pub(super) client: Client,
    pub(super) progress: Arc<dyn ExecutorProgress>,
}

impl RootfsFetcher for HttpRootfsFetcher {
    fn fetch(
        &self,
        image: &RootfsImage,
        out: &mut dyn Write,
        cancel: &CancellationToken,
    ) -> Result<(), tether_linux::LinuxError> {
        let download = Download {
            client: &self.client,
            url: &image.url,
            limit: image.size_bytes,
            cancel,
        };
        download_sync(&download, out, |done, total| {
            self.progress.on_progress(done, total);
        })
        .map_err(tether_linux::LinuxError::Fetch)
    }
}

pub(super) struct ProgressAdapter {
    pub(super) callback: Arc<dyn ExecutorProgress>,
    pub(super) total: u64,
}

impl RootfsProgress for ProgressAdapter {
    fn downloading(&mut self, bytes: u64) {
        self.callback.on_progress(bytes, self.total);
    }

    fn done(&mut self) {
        self.callback.on_progress(self.total, self.total);
    }
}

pub(super) fn http_client() -> Result<Client, ExecutorRuntimeError> {
    client_with(redirect_allowed)
}

pub(super) fn client_with(rule: fn(&Url, usize) -> bool) -> Result<Client, ExecutorRuntimeError> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            let previous_hops = attempt.previous().len().saturating_sub(1);
            if rule(attempt.url(), previous_hops) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|_| ExecutorRuntimeError::Unavailable)
}

pub(super) fn redirect_allowed(target: &Url, previous_hops: usize) -> bool {
    const MAX_REDIRECT_HOPS: usize = 5;

    target.scheme() == "https"
        && previous_hops < MAX_REDIRECT_HOPS
        && target.host_str().is_some_and(|host| {
            host == "github.com"
                || host == "githubusercontent.com"
                || host.ends_with(".githubusercontent.com")
        })
}

pub(super) async fn fetch_manifest(
    client: &Client,
    manifest_url: &str,
) -> Result<(Vec<u8>, Vec<u8>), ExecutorRuntimeError> {
    let manifest = fetch_bytes(client, manifest_url, MANIFEST_CAP).await?;
    let signature = fetch_bytes(client, &format!("{manifest_url}.sig"), SIGNATURE_CAP).await?;
    Ok((manifest, signature))
}

pub(super) async fn fetch_bytes(
    client: &Client,
    url: &str,
    cap: usize,
) -> Result<Vec<u8>, ExecutorRuntimeError> {
    require_https(url)?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| ExecutorRuntimeError::Unavailable)?;
    if !response.status().is_success() {
        return Err(ExecutorRuntimeError::Unavailable);
    }
    if response
        .content_length()
        .is_some_and(|length| length > cap as u64)
    {
        return Err(ExecutorRuntimeError::InvalidConfiguration);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ExecutorRuntimeError::Unavailable)?
    {
        if bytes.len().saturating_add(chunk.len()) > cap {
            return Err(ExecutorRuntimeError::InvalidConfiguration);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// One HTTPS download: where it comes from, its size cap, and its cancel token.
struct Download<'a> {
    client: &'a Client,
    url: &'a str,
    limit: u64,
    cancel: &'a CancellationToken,
}

fn download_sync<F: Fn(u64, u64)>(
    download: &Download<'_>,
    out: &mut dyn Write,
    progress: F,
) -> Result<(), String> {
    require_https(download.url).map_err(|_| "download URL must be HTTPS".to_owned())?;
    let handle = tokio::runtime::Handle::current();
    handle.block_on(copy_download(download, out, &progress))
}

async fn copy_download<F: Fn(u64, u64)>(
    download: &Download<'_>,
    out: &mut dyn Write,
    progress: &F,
) -> Result<(), String> {
    let mut response = download
        .client
        .get(download.url)
        .send()
        .await
        .map_err(|_| "download failed".to_owned())?;
    if !response.status().is_success() {
        return Err("download failed".to_owned());
    }
    let mut written = 0_u64;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "download failed".to_owned())?
    {
        written = accept_chunk(download, written, chunk.len())?;
        out.write_all(&chunk)
            .map_err(|_| "write failed".to_owned())?;
        progress(written, download.limit);
    }
    if written != download.limit {
        return Err("download size mismatch".to_owned());
    }
    Ok(())
}

/// Checks one received chunk against cancellation and the size cap.
/// Returns the byte count after the chunk.
fn accept_chunk(download: &Download<'_>, written: u64, chunk_len: usize) -> Result<u64, String> {
    if download.cancel.is_cancelled() {
        return Err("download cancelled".to_owned());
    }
    let written = written.saturating_add(chunk_len as u64);
    if written > download.limit {
        return Err("download is too large".to_owned());
    }
    Ok(written)
}

fn pack_file_url(base: &str, pack: &PackEntry, file: &PackFile) -> Result<String, ()> {
    let url = Url::parse(base).map_err(|_| ())?;
    let path = format!("{}/{}/{}", pack.id, pack.version, file.path);
    url.join(&path)
        .map(|joined| joined.to_string())
        .map_err(|_| ())
}

pub(super) fn require_https(url: &str) -> Result<(), ExecutorRuntimeError> {
    let parsed = Url::parse(url).map_err(|_| ExecutorRuntimeError::InvalidConfiguration)?;
    #[cfg(test)]
    let local_test_http = parsed.scheme() == "http" && parsed.host_str() == Some("127.0.0.1");
    #[cfg(not(test))]
    let local_test_http = false;
    if parsed.scheme() == "https" || local_test_http {
        Ok(())
    } else {
        Err(ExecutorRuntimeError::InvalidConfiguration)
    }
}
