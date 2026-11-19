//! The name of a stored secret.

use crate::SecretsError;

/// The longest reference, in characters.
const MAX_LEN: usize = 64;

/// Names one stored secret, such as `provider-anthropic`. It becomes a file name, so it
/// allows only `a-z`, `0-9`, `-`, and `_`, up to 64 characters. A reference can never be a path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretRef(String);

impl SecretRef {
    /// Checks and wraps a reference.
    ///
    /// # Errors
    /// Returns an error when the reference is empty, too long, or has a character outside
    /// `a-z`, `0-9`, `-`, and `_`.
    pub fn new(value: &str) -> Result<Self, SecretsError> {
        if value.is_empty() {
            return Err(SecretsError::InvalidReference {
                reason: "it is empty",
            });
        }
        if value.len() > MAX_LEN {
            return Err(SecretsError::InvalidReference {
                reason: "it is longer than 64 characters",
            });
        }
        let allowed =
            |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
        if !value.chars().all(allowed) {
            return Err(SecretsError::InvalidReference {
                reason: "it has a character outside a-z, 0-9, - and _",
            });
        }
        Ok(Self(value.to_owned()))
    }

    /// The reference as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names_are_accepted() {
        for name in ["provider-anthropic", "github_token", "a", "k2"] {
            assert_eq!(SecretRef::new(name).unwrap().as_str(), name);
        }
    }

    #[test]
    fn anything_that_could_be_a_path_is_rejected() {
        let too_long = "a".repeat(65);
        for name in [
            "", "../x", "a/b", "a\\b", ".", "..", "A", "a b", "é", "a.bin", &too_long,
        ] {
            assert!(SecretRef::new(name).is_err(), "accepted {name:?}");
        }
    }

    #[test]
    fn a_name_of_exactly_the_limit_is_accepted() {
        assert!(SecretRef::new(&"a".repeat(64)).is_ok());
    }
}
