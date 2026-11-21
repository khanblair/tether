use std::fmt;

/// A failure from Linux mode setup or process execution.
#[derive(Debug)]
pub enum LinuxError {
    /// The signed manifest is invalid.
    Signature,
    /// The manifest failed validation.
    Manifest(String),
    /// A requested archive or filesystem path is invalid.
    Path(String),
    /// A filesystem operation failed.
    Io(String),
    /// An archive entry failed a safety check.
    Archive(String),
    /// A download failed or did not match its declaration.
    Fetch(String),
    /// The operation was cancelled.
    Cancelled,
    /// Linux mode is not installed or cannot be used yet.
    NotReady(String),
    /// A child process could not be started or waited for.
    Process(String),
    /// Base package installation failed.
    Packages(String),
}

impl fmt::Display for LinuxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Signature => write!(formatter, "Linux manifest signature verification failed"),
            Self::Manifest(message) => write!(formatter, "Linux manifest invalid: {message}"),
            Self::Path(message) => write!(formatter, "Linux path rejected: {message}"),
            Self::Io(message) => write!(formatter, "Linux storage operation failed: {message}"),
            Self::Archive(message) => write!(
                formatter,
                "Linux root filesystem archive rejected: {message}"
            ),
            Self::Fetch(message) => write!(
                formatter,
                "Linux root filesystem download failed: {message}"
            ),
            Self::Cancelled => write!(formatter, "Linux operation cancelled"),
            Self::NotReady(message) => write!(formatter, "Linux mode is not ready: {message}"),
            Self::Process(message) => write!(formatter, "Linux process failed: {message}"),
            Self::Packages(message) => write!(formatter, "Linux base packages failed: {message}"),
        }
    }
}

impl std::error::Error for LinuxError {}

pub(crate) fn io_error(context: &str) -> LinuxError {
    LinuxError::Io(context.to_owned())
}
