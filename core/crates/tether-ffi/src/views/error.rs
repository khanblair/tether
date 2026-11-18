//! The error view carried by events such as `TurnFailed`.

use tether_types::CoreError;

/// What kind of failure a [`CoreErrorView`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ErrorKindView {
    /// A bug in Tether.
    Internal,
    /// The network failed or timed out.
    Network,
    /// A provider or GitHub rejected the credentials.
    Auth,
    /// A provider asked the client to slow down.
    RateLimited,
    /// The request itself was wrong.
    InvalidInput,
    /// The thing asked for does not exist.
    NotFound,
    /// The user or the system cancelled the work.
    Cancelled,
}

/// FFI view of [`CoreError`]. It is a plain record, not a Kotlin exception, so
/// events that carry it compare by value and cost no stack trace.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CoreErrorView {
    /// What kind of failure it is.
    pub kind: ErrorKindView,
    /// What went wrong, safe to show.
    pub message: String,
    /// True when the same request can succeed on a retry.
    pub retryable: bool,
}

impl From<CoreError> for CoreErrorView {
    fn from(error: CoreError) -> Self {
        let kind = match &error {
            CoreError::Internal { .. } => ErrorKindView::Internal,
            CoreError::Network { .. } => ErrorKindView::Network,
            CoreError::Auth { .. } => ErrorKindView::Auth,
            CoreError::RateLimited { .. } => ErrorKindView::RateLimited,
            CoreError::InvalidInput { .. } => ErrorKindView::InvalidInput,
            CoreError::NotFound { .. } => ErrorKindView::NotFound,
            CoreError::Cancelled { .. } => ErrorKindView::Cancelled,
        };
        Self {
            kind,
            message: error.message().to_owned(),
            retryable: error.retryable(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_error_keeps_kind_message_and_retryable_flag() {
        let view = CoreErrorView::from(CoreError::Network {
            message: "timeout".into(),
        });
        assert_eq!(
            view,
            CoreErrorView {
                kind: ErrorKindView::Network,
                message: "timeout".into(),
                retryable: true
            }
        );
    }

    #[test]
    fn auth_error_is_not_retryable() {
        let view = CoreErrorView::from(CoreError::Auth {
            message: "bad key".into(),
        });
        assert_eq!(view.kind, ErrorKindView::Auth);
        assert!(!view.retryable);
    }
}
