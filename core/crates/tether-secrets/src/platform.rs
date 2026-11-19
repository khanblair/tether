//! What the platform provides: wrapping with a key that never leaves secure hardware.

use std::fmt;

/// The platform could not wrap or unwrap. The message never contains secret bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformError {
    /// What went wrong.
    pub message: String,
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Implemented by the platform: Android Keystore today, the iOS Keychain later.
///
/// Both calls take a `context`. The store passes the secret's reference, and the platform
/// must bind it to the wrapped bytes (as authenticated data), so that a wrapped file moved
/// to another reference fails to unwrap.
pub trait SecretsPlatform: Send + Sync {
    /// Encrypts `plaintext` with a platform key and binds it to `context`.
    ///
    /// # Errors
    /// Returns an error when the platform key is unavailable or encryption fails.
    fn wrap(&self, plaintext: &[u8], context: &[u8]) -> Result<Vec<u8>, PlatformError>;

    /// Decrypts bytes from [`wrap`](Self::wrap). It fails when `context` differs from the
    /// one used to wrap, or when the bytes were changed.
    ///
    /// # Errors
    /// Returns an error when decryption or the integrity check fails.
    fn unwrap(&self, wrapped: &[u8], context: &[u8]) -> Result<Vec<u8>, PlatformError>;
}
