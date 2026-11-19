//! A secret value held in memory.

use std::fmt;
use std::str::Utf8Error;

use zeroize::Zeroizing;

/// A secret value. The bytes are overwritten when it is dropped, and it never prints.
pub struct Secret(Zeroizing<Vec<u8>>);

impl Secret {
    /// Wraps secret bytes. The bytes are wiped when the secret is dropped.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// Copies secret text into a new secret.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self::new(text.as_bytes().to_vec())
    }

    /// The secret bytes. Use them for the one call that needs them and do not store them.
    #[must_use]
    pub fn expose(&self) -> &[u8] {
        &self.0
    }

    /// The secret as text.
    ///
    /// # Errors
    /// Returns an error when the bytes are not valid UTF-8.
    pub fn expose_text(&self) -> Result<&str, Utf8Error> {
        std::str::from_utf8(&self.0)
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_hides_the_value() {
        let secret = Secret::from_text("sk-live-123");
        assert_eq!(format!("{secret:?}"), "Secret(<redacted>)");
    }

    #[test]
    fn text_round_trips() {
        let secret = Secret::from_text("sk-live-123");
        assert_eq!(secret.expose_text().unwrap(), "sk-live-123");
        assert_eq!(secret.expose(), b"sk-live-123");
    }

    #[test]
    fn non_utf8_bytes_are_not_text() {
        assert!(Secret::new(vec![0xff, 0xfe]).expose_text().is_err());
    }
}
