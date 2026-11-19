//! The shared HTTP client every provider sends its requests through.

use std::time::Duration;

use reqwest::redirect::Policy;
use reqwest::{Client, ClientBuilder};

use crate::ProviderError;

/// How long connecting may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long a read may wait for the next bytes. A stream sends something every few seconds,
/// so a longer silence means the connection is dead. There is no total time limit, because a
/// long answer streams for minutes.
const READ_TIMEOUT: Duration = Duration::from_secs(120);

/// The client settings shared by the real client and by tests: timeouts, and no redirects.
///
/// A redirect is never followed, because reqwest forwards custom headers such as `x-api-key`
/// to the new host. A redirect response reaches the adapter as an error instead.
pub fn client_builder() -> ClientBuilder {
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .redirect(Policy::none())
}

/// Builds the one HTTP client the engine shares across providers. It refuses plain HTTP, so
/// an API key never travels unencrypted.
///
/// # Errors
/// Returns an error when the TLS backend cannot start.
pub fn http_client() -> Result<Client, ProviderError> {
    client_builder()
        .https_only(true)
        .build()
        .map_err(|error| ProviderError::Network {
            message: error.without_url().to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_shared_client_refuses_plain_http() {
        let client = http_client().unwrap();
        let result = client.get("http://127.0.0.1:9/").send().await;
        assert!(result.is_err());
    }
}
