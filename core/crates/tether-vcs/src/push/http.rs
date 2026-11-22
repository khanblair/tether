//! Blocking smart HTTP requests on `reqwest`, with no credential in the URL.

use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use tether_secrets::Secret;

use super::TransportError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;

/// One request: a GET, or a POST when `body` is set.
pub(super) struct Request<'a> {
    pub(super) url: &'a str,
    pub(super) token: &'a Secret,
    pub(super) headers: BTreeMap<String, String>,
    pub(super) post: Option<(&'a str, Vec<u8>)>,
}

/// Runs the request to completion on a private current-thread runtime. The caller runs this on
/// a blocking thread, never inside an async task.
pub(super) fn send(request: Request<'_>) -> Result<Vec<u8>, TransportError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| TransportError)?;
    runtime.block_on(send_async(request))
}

async fn send_async(request: Request<'_>) -> Result<Vec<u8>, TransportError> {
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
        .redirect(same_host_policy())
        .build()
        .map_err(|_| TransportError)?;
    let headers = header_map(&request)?;
    let builder = match request.post {
        Some((_, body)) => client.post(request.url).body(body),
        None => client.get(request.url),
    };
    let mut response = builder
        .headers(headers)
        .send()
        .await
        .map_err(|_| TransportError)?;
    if !response.status().is_success() {
        return Err(TransportError);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| TransportError)? {
        if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(TransportError);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Follows redirects only to the same host, so the token never travels to another host.
fn same_host_policy() -> Policy {
    Policy::custom(|attempt| {
        let same_host = attempt
            .previous()
            .first()
            .is_some_and(|first| first.host_str() == attempt.url().host_str());
        if attempt.previous().len() >= MAX_REDIRECTS || !same_host {
            attempt.stop()
        } else {
            attempt.follow()
        }
    })
}

fn header_map(request: &Request<'_>) -> Result<HeaderMap, TransportError> {
    let token = request.token.expose_text().map_err(|_| TransportError)?;
    let mut map = HeaderMap::new();
    let mut auth = HeaderValue::from_str(&format!(
        "Basic {}",
        super::base64_encode(format!("x-access-token:{token}").as_bytes())
    ))
    .map_err(|_| TransportError)?;
    auth.set_sensitive(true);
    map.insert(AUTHORIZATION, auth);
    for (name, value) in &request.headers {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| TransportError)?;
        let value = HeaderValue::from_str(value).map_err(|_| TransportError)?;
        map.insert(name, value);
    }
    if let Some((content_type, _)) = &request.post {
        map.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(content_type).map_err(|_| TransportError)?,
        );
    }
    Ok(map)
}
