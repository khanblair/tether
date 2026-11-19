//! Errors the secret store reports. Messages never contain secret values.

use std::io;

use tether_types::CoreError;
use thiserror::Error;

/// A failed secret operation.
#[derive(Debug, Error)]
pub enum SecretsError {
    /// The reference is not a valid name.
    #[error("invalid secret reference: {reason}")]
    InvalidReference {
        /// Which rule the reference broke.
        reason: &'static str,
    },
    /// Reading or writing the secret file failed.
    #[error("could not {action} the secret file: {source}")]
    Io {
        /// What the store was doing, such as "write".
        action: &'static str,
        /// The operating system error.
        source: io::Error,
    },
    /// The secret file is not in the expected format.
    #[error("the secret file for {reference} is damaged or from a newer app")]
    Corrupt {
        /// The reference whose file could not be read.
        reference: String,
    },
    /// The platform could not wrap or unwrap the secret.
    #[error("the platform keystore failed: {message}")]
    Platform {
        /// What the platform reported.
        message: String,
    },
}

impl From<SecretsError> for CoreError {
    fn from(error: SecretsError) -> Self {
        let message = error.to_string();
        match error {
            SecretsError::InvalidReference { .. } => Self::InvalidInput { message },
            SecretsError::Io { .. }
            | SecretsError::Corrupt { .. }
            | SecretsError::Platform { .. } => Self::Internal { message },
        }
    }
}
