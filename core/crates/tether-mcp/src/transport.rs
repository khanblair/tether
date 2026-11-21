//! Transport contracts and the production HTTP transport.

use reqwest::Client;
use reqwest::header::{HeaderName, HeaderValue};
use std::fmt;
use tether_providers::http_client;
use tether_secrets::Secret;
use thiserror::Error;
use tokio::sync::mpsc;

/// A stream of response byte chunks.
pub struct BodyStream {
    receiver: mpsc::Receiver<Result<Vec<u8>, String>>,
}

impl fmt::Debug for BodyStream {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BodyStream(<stream>)")
    }
}

impl BodyStream {
    /// Creates a body stream from complete chunks.
    #[must_use]
    pub fn from_chunks(chunks: Vec<Vec<u8>>) -> Self {
        let (sender, receiver) = mpsc::channel(chunks.len().max(1));
        for chunk in chunks {
            drop(sender.try_send(Ok(chunk)));
        }
        drop(sender);
        Self { receiver }
    }

    /// Returns the next chunk.
    ///
    /// # Errors
    /// Returns the transport's body-read message.
    pub async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, String> {
        match self.receiver.recv().await {
            Some(Ok(chunk)) => Ok(Some(chunk)),
            Some(Err(message)) => Err(message),
            None => Ok(None),
        }
    }

    fn from_response(response: reqwest::Response) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        tokio::spawn(stream_response(response, sender));
        Self { receiver }
    }
}

async fn stream_response(
    mut response: reqwest::Response,
    sender: mpsc::Sender<Result<Vec<u8>, String>>,
) {
    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => return,
            Err(error) => {
                drop(sender.send(Err(error.without_url().to_string())).await);
                return;
            }
        };
        if sender.send(Ok(chunk.to_vec())).await.is_err() {
            return;
        }
    }
}

/// A JSON POST request handed to an MCP transport.
#[derive(Clone, Debug)]
pub struct HttpPost {
    /// The request URL.
    pub url: String,
    /// Request headers.
    pub headers: Vec<(String, String)>,
    /// UTF-8 JSON request body.
    pub body: Vec<u8>,
}

/// An HTTP DELETE request handed to an MCP transport.
#[derive(Clone, Debug)]
pub struct HttpDelete {
    /// The request URL.
    pub url: String,
    /// Request headers.
    pub headers: Vec<(String, String)>,
}

/// An HTTP response returned by an MCP transport.
#[derive(Debug)]
pub struct HttpReply {
    /// HTTP status code.
    pub status: u16,
    /// Response headers.
    pub headers: Vec<(String, String)>,
    /// Response body chunks.
    pub body: BodyStream,
}

/// Errors produced by an MCP client.
#[derive(Debug, Error)]
pub enum McpError {
    /// The server configuration or request was invalid.
    #[error("MCP server {server}: invalid configuration: {message}")]
    InvalidConfig {
        /// Server display name.
        server: String,
        /// Safe error detail.
        message: String,
    },
    /// The server returned an HTTP failure.
    #[error("MCP server {server}: HTTP status {status}")]
    Http {
        /// Server display name.
        server: String,
        /// HTTP status code.
        status: u16,
    },
    /// The server returned a malformed protocol message.
    #[error("MCP server {server}: protocol error: {message}")]
    Protocol {
        /// Server display name.
        server: String,
        /// Safe protocol detail.
        message: String,
    },
    /// The request could not be sent or read.
    #[error("MCP server {server}: network error: {message}")]
    Network {
        /// Server display name.
        server: String,
        /// Safe network detail.
        message: String,
    },
    /// The request deadline elapsed.
    #[error("MCP server {server}: request timed out")]
    Timeout {
        /// Server display name.
        server: String,
    },
    /// The caller cancelled the request.
    #[error("MCP server {server}: cancelled")]
    Cancelled {
        /// Server display name.
        server: String,
    },
}

/// A remote MCP server and its sensitive headers.
pub struct McpServerConfig {
    /// The local display name for the server.
    pub name: String,
    /// The HTTPS endpoint.
    pub url: String,
    /// Headers sent to this server.
    pub headers: Vec<(String, Secret)>,
}

impl fmt::Debug for McpServerConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("McpServerConfig")
            .field("name", &self.name)
            .field("url", &redact_url(&self.url))
            .field(
                "headers",
                &self
                    .headers
                    .iter()
                    .map(|(name, _)| name)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl McpServerConfig {
    pub(crate) fn validate(&self) -> Result<(), McpError> {
        let parsed = reqwest::Url::parse(&self.url).map_err(|_| invalid(&self.name, "URL"))?;
        if parsed.scheme() != "https"
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.host_str().is_none()
        {
            return Err(invalid(&self.name, "URL"));
        }
        for (name, secret) in &self.headers {
            HeaderName::from_bytes(name.as_bytes()).map_err(|_| invalid(&self.name, "header"))?;
            HeaderValue::from_bytes(secret.expose()).map_err(|_| invalid(&self.name, "header"))?;
        }
        Ok(())
    }
}

fn invalid(server: &str, thing: &str) -> McpError {
    McpError::InvalidConfig {
        server: server.to_owned(),
        message: format!("invalid {thing}"),
    }
}

fn redact_url(url: &str) -> String {
    reqwest::Url::parse(url).map_or_else(
        |_| "<invalid>".to_owned(),
        |mut parsed| {
            parsed.set_query(None);
            parsed.set_password(None).ok();
            parsed.set_username("").ok();
            parsed.to_string()
        },
    )
}

/// An async transport for MCP HTTP requests.
pub trait McpTransport: Send + Sync {
    /// Sends a POST request.
    fn post(&self, request: HttpPost) -> TransportFuture;
    /// Sends a DELETE request.
    fn delete(&self, request: HttpDelete) -> TransportFuture;
}

/// The boxed future returned by [`McpTransport`].
pub type TransportFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<HttpReply, McpError>> + Send>>;

/// The production MCP transport backed by reqwest.
pub struct ReqwestMcpTransport {
    client: Client,
}

impl ReqwestMcpTransport {
    /// Creates a transport using Tether's shared HTTPS client.
    ///
    /// # Errors
    /// Returns an error when the TLS client cannot be built.
    pub fn new(server: &str) -> Result<Self, McpError> {
        http_client()
            .map(|client| Self { client })
            .map_err(|error| McpError::Network {
                server: server.to_owned(),
                message: error.to_string(),
            })
    }
}

impl McpTransport for ReqwestMcpTransport {
    fn post(&self, request: HttpPost) -> TransportFuture {
        let client = self.client.clone();
        Box::pin(async move { send_post(client, request).await })
    }

    fn delete(&self, request: HttpDelete) -> TransportFuture {
        let client = self.client.clone();
        Box::pin(async move { send_delete(client, request).await })
    }
}

async fn send_post(client: Client, request: HttpPost) -> Result<HttpReply, McpError> {
    let request_builder = client.post(&request.url).body(request.body);
    let request_builder = apply_headers(request_builder, &request.headers)?;
    let response = request_builder.send().await.map_err(network_error)?;
    Ok(reply(response))
}

async fn send_delete(client: Client, request: HttpDelete) -> Result<HttpReply, McpError> {
    let request_builder = apply_headers(client.delete(&request.url), &request.headers)?;
    let response = request_builder.send().await.map_err(network_error)?;
    Ok(reply(response))
}

fn apply_headers(
    mut builder: reqwest::RequestBuilder,
    headers: &[(String, String)],
) -> Result<reqwest::RequestBuilder, McpError> {
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| McpError::Network {
            server: "unknown".to_owned(),
            message: "invalid header".to_owned(),
        })?;
        let value = HeaderValue::from_str(value).map_err(|_| McpError::Network {
            server: "unknown".to_owned(),
            message: "invalid header".to_owned(),
        })?;
        builder = builder.header(name, value);
    }
    Ok(builder)
}

fn reply(response: reqwest::Response) -> HttpReply {
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.to_string(), value.to_owned()))
        })
        .collect();
    let body = BodyStream::from_response(response);
    HttpReply {
        status,
        headers,
        body,
    }
}

fn network_error(error: reqwest::Error) -> McpError {
    McpError::Network {
        server: "unknown".to_owned(),
        message: error.without_url().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_debug_redacts_header_and_query() {
        let config = McpServerConfig {
            name: "test".to_owned(),
            url: "https://example.test/path?token=query".to_owned(),
            headers: vec![(
                "authorization".to_owned(),
                Secret::from_text("header-secret"),
            )],
        };
        let debug = format!("{config:?}");
        assert!(!debug.contains("header-secret"));
        assert!(!debug.contains("token=query"));
    }

    #[test]
    fn rejects_http_and_userinfo() {
        for url in [
            "http://example.test",
            "https://user:pass@example.test",
            "junk",
        ] {
            let config = McpServerConfig {
                name: "server".to_owned(),
                url: url.to_owned(),
                headers: vec![],
            };
            assert!(config.validate().is_err());
        }
    }
}
