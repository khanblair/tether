//! Web backend contract and the DNS-filtering reqwest backend.

use std::future::Future;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::{Client, RequestBuilder};
use tokio::sync::mpsc;

use crate::fetch::FetchError;
use crate::ip::is_public_ip;

/// A stream of web response byte chunks.
pub struct WebBody {
    receiver: mpsc::Receiver<Result<Vec<u8>, String>>,
}

impl std::fmt::Debug for WebBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WebBody(<stream>)")
    }
}

impl WebBody {
    /// Creates a body from complete chunks.
    #[must_use]
    pub fn from_chunks(chunks: Vec<Vec<u8>>) -> Self {
        let (sender, receiver) = mpsc::channel(chunks.len().max(1));
        for chunk in chunks {
            drop(sender.try_send(Ok(chunk)));
        }
        drop(sender);
        Self { receiver }
    }

    /// Returns the next response chunk.
    ///
    /// # Errors
    /// Returns the backend's body-read message.
    pub async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, String> {
        match self.receiver.recv().await {
            Some(Ok(chunk)) => Ok(Some(chunk)),
            Some(Err(error)) => Err(error),
            None => Ok(None),
        }
    }

    fn from_response(response: reqwest::Response, max_bytes: usize) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        tokio::spawn(stream_response(response, sender, max_bytes));
        Self { receiver }
    }
}

async fn stream_response(
    mut response: reqwest::Response,
    sender: mpsc::Sender<Result<Vec<u8>, String>>,
    max_bytes: usize,
) {
    let mut read = 0;
    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => return,
            Err(error) => {
                drop(sender.send(Err(error.without_url().to_string())).await);
                return;
            }
        };
        let remaining = max_bytes.saturating_sub(read);
        if remaining == 0 {
            return;
        }
        let size = chunk.len().min(remaining);
        read += size;
        if sender.send(Ok(chunk[..size].to_vec())).await.is_err() {
            return;
        }
        if size < chunk.len() {
            return;
        }
    }
}

/// A response returned by a [`WebBackend`].
#[derive(Debug)]
pub struct WebReply {
    /// HTTP status code.
    pub status: u16,
    /// Response headers.
    pub headers: Vec<(String, String)>,
    /// Response body.
    pub body: WebBody,
}

/// An async backend for bounded web GET requests.
pub trait WebBackend: Send + Sync {
    /// Gets a URL with the supplied headers and body limit.
    fn get(&self, url: String, headers: Vec<(String, String)>, max_bytes: usize) -> WebFuture;
}

/// The boxed future returned by [`WebBackend`].
pub type WebFuture = Pin<Box<dyn Future<Output = Result<WebReply, FetchError>> + Send>>;

/// The production backend with connect-time public DNS filtering.
pub struct ReqwestWebBackend {
    client: Client,
}

impl ReqwestWebBackend {
    /// Builds the production web backend.
    ///
    /// # Errors
    /// Returns a client construction error.
    pub fn new() -> Result<Self, FetchError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(30))
            .redirect(Policy::none())
            .dns_resolver(Arc::new(PublicResolver))
            .https_only(false)
            .build()
            .map_err(|_| FetchError::Backend("could not create the web client".to_owned()))?;
        Ok(Self { client })
    }
}

impl WebBackend for ReqwestWebBackend {
    fn get(&self, url: String, headers: Vec<(String, String)>, max_bytes: usize) -> WebFuture {
        let client = self.client.clone();
        Box::pin(async move {
            let mut request = client.get(&url);
            request = apply_headers(request, &headers);
            let response = request
                .send()
                .await
                .map_err(|_| FetchError::Backend("web request failed".to_owned()))?;
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .filter_map(|pair| header_pair(pair.0, pair.1))
                .collect();
            Ok(WebReply {
                status,
                headers,
                body: WebBody::from_response(response, max_bytes),
            })
        })
    }
}

fn header_pair(name: &HeaderName, value: &HeaderValue) -> Option<(String, String)> {
    value
        .to_str()
        .ok()
        .map(|value| (name.to_string(), value.to_owned()))
}

fn apply_headers(mut request: RequestBuilder, headers: &[(String, String)]) -> RequestBuilder {
    for (name, value) in headers {
        request = request.header(name, value);
    }
    request
}

struct PublicResolver;

impl Resolve for PublicResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let result = tokio::task::spawn_blocking(move || (host.as_str(), 0).to_socket_addrs())
                .await
                .map_err(|error| boxed(error.to_string()))?;
            let addresses = public_addresses(result.map_err(|error| boxed(error.to_string()))?);
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

fn public_addresses(
    addresses: impl Iterator<Item = std::net::SocketAddr>,
) -> Vec<std::net::SocketAddr> {
    addresses
        .filter(|address| is_public_ip(address.ip()))
        .collect()
}

fn boxed(message: String) -> Box<dyn std::error::Error + Send + Sync> {
    message.into()
}
