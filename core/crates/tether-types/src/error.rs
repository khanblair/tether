//! The error every command and event reports to the platform layer.

use thiserror::Error;

/// A failure the user can see. Crates keep their own `thiserror` enums and
/// convert to this one at the engine boundary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    /// A bug in Tether, or a panic caught at the FFI boundary.
    #[error("internal error: {message}")]
    Internal {
        /// What went wrong, safe to show.
        message: String,
    },
    /// The network failed or timed out.
    #[error("network error: {message}")]
    Network {
        /// What went wrong, safe to show.
        message: String,
    },
    /// A provider or GitHub rejected the credentials.
    #[error("authentication failed: {message}")]
    Auth {
        /// What went wrong, safe to show.
        message: String,
    },
    /// A provider asked the client to slow down.
    #[error("rate limited: {message}")]
    RateLimited {
        /// What went wrong, safe to show.
        message: String,
    },
    /// The request itself was wrong.
    #[error("invalid input: {message}")]
    InvalidInput {
        /// What went wrong, safe to show.
        message: String,
    },
    /// The thing asked for does not exist.
    #[error("not found: {message}")]
    NotFound {
        /// What went wrong, safe to show.
        message: String,
    },
    /// The user or the system cancelled the work.
    #[error("cancelled: {message}")]
    Cancelled {
        /// What happened, safe to show.
        message: String,
    },
}

impl CoreError {
    /// True when trying the same request again can succeed.
    #[must_use]
    pub fn retryable(&self) -> bool {
        matches!(self, Self::Network { .. } | Self::RateLimited { .. })
    }

    /// The message without the kind prefix that `Display` adds.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Internal { message }
            | Self::Network { message }
            | Self::Auth { message }
            | Self::RateLimited { message }
            | Self::InvalidInput { message }
            | Self::NotFound { message }
            | Self::Cancelled { message } => message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_network_and_rate_limit_errors_are_retryable() {
        let net = CoreError::Network {
            message: "timeout".into(),
        };
        let auth = CoreError::Auth {
            message: "bad key".into(),
        };
        assert!(net.retryable());
        assert!(!auth.retryable());
        assert_eq!(net.message(), "timeout");
        assert_eq!(net.to_string(), "network error: timeout");
    }
}
