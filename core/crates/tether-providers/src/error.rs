//! Errors a provider reports. The engine converts them to `CoreError`.

use tether_types::CoreError;
use thiserror::Error;

/// A failed request or stream. Messages never contain API keys.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProviderError {
    /// The provider rejected the key.
    #[error("the provider rejected the API key: {message}")]
    Auth {
        /// What the provider said.
        message: String,
    },
    /// The provider forbids this region or account.
    #[error("the provider blocked access: {message}")]
    Blocked {
        /// What the provider said.
        message: String,
    },
    /// The provider asked the client to slow down.
    #[error("rate limited: {message}")]
    RateLimited {
        /// What the provider said.
        message: String,
        /// Seconds to wait, when the provider said.
        retry_after_secs: Option<u64>,
    },
    /// The provider rejected the request itself.
    #[error("the provider rejected the request: {message}")]
    InvalidRequest {
        /// What the provider said.
        message: String,
    },
    /// The provider failed or is overloaded (HTTP 5xx).
    #[error("provider error {status}: {message}")]
    Server {
        /// The HTTP status code.
        status: u16,
        /// What the provider said.
        message: String,
    },
    /// The connection failed or timed out.
    #[error("network error: {message}")]
    Network {
        /// What went wrong.
        message: String,
    },
    /// The stream did not follow the provider's format.
    #[error("the response stream was malformed: {message}")]
    MalformedStream {
        /// What was wrong.
        message: String,
    },
    /// The caller cancelled the request or stopped listening.
    #[error("cancelled")]
    Cancelled,
}

impl From<ProviderError> for CoreError {
    fn from(error: ProviderError) -> Self {
        let message = error.to_string();
        match error {
            ProviderError::Auth { .. } | ProviderError::Blocked { .. } => Self::Auth { message },
            ProviderError::RateLimited { .. } => Self::RateLimited { message },
            ProviderError::InvalidRequest { .. } => Self::InvalidInput { message },
            ProviderError::Server { .. } | ProviderError::Network { .. } => {
                Self::Network { message }
            }
            ProviderError::MalformedStream { .. } => Self::Internal { message },
            ProviderError::Cancelled => Self::Cancelled { message },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_and_rate_limit_errors_are_retryable_after_conversion() {
        let server: CoreError = ProviderError::Server {
            status: 529,
            message: "overloaded".into(),
        }
        .into();
        let limited: CoreError = ProviderError::RateLimited {
            message: "slow".into(),
            retry_after_secs: Some(3),
        }
        .into();
        assert!(server.retryable());
        assert!(limited.retryable());
    }

    #[test]
    fn auth_and_invalid_request_errors_are_not_retryable() {
        let auth: CoreError = ProviderError::Auth {
            message: "bad key".into(),
        }
        .into();
        let invalid: CoreError = ProviderError::InvalidRequest {
            message: "too long".into(),
        }
        .into();
        assert!(!auth.retryable());
        assert!(!invalid.retryable());
    }
}
