//! UniFFI face of the secret store: the platform callback and the vault Kotlin calls.

use std::sync::Arc;

use tether_secrets::{
    PlatformError, Secret, SecretRef, SecretStore, SecretsError, SecretsPlatform as CorePlatform,
};

/// The platform could not wrap or unwrap a secret.
///
/// The field is `detail`, not `message`: UniFFI gives every Kotlin exception a `message`
/// property, and a field with that name does not compile.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SecretsPlatformFailure {
    /// The Keystore call failed. The text never contains secret bytes.
    #[error("{detail}")]
    Failed {
        /// What went wrong.
        detail: String,
    },
}

impl From<uniffi::UnexpectedUniFFICallbackError> for SecretsPlatformFailure {
    fn from(error: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Failed {
            detail: error.reason,
        }
    }
}

/// Implemented in Kotlin with an Android Keystore key.
///
/// Both calls take a `context` (the secret's reference) that the implementation must bind
/// to the wrapped bytes as authenticated data, so a wrapped file moved to another reference
/// fails to unwrap.
#[uniffi::export(with_foreign)]
pub trait SecretsPlatform: Send + Sync {
    /// Encrypts `plaintext` with a platform key and binds it to `context`.
    ///
    /// # Errors
    /// Returns an error when the Keystore key is unavailable or encryption fails.
    fn wrap(&self, plaintext: Vec<u8>, context: Vec<u8>)
    -> Result<Vec<u8>, SecretsPlatformFailure>;

    /// Decrypts bytes from `wrap`. It fails when `context` differs or the bytes changed.
    ///
    /// # Errors
    /// Returns an error when decryption or the integrity check fails.
    fn unwrap(&self, wrapped: Vec<u8>, context: Vec<u8>)
    -> Result<Vec<u8>, SecretsPlatformFailure>;
}

/// Adapts the Kotlin callback to the store's own platform trait.
struct PlatformAdapter(Arc<dyn SecretsPlatform>);

fn platform_error(failure: SecretsPlatformFailure) -> PlatformError {
    let SecretsPlatformFailure::Failed { detail } = failure;
    PlatformError { message: detail }
}

impl CorePlatform for PlatformAdapter {
    fn wrap(&self, plaintext: &[u8], context: &[u8]) -> Result<Vec<u8>, PlatformError> {
        self.0
            .wrap(plaintext.to_vec(), context.to_vec())
            .map_err(platform_error)
    }

    fn unwrap(&self, wrapped: &[u8], context: &[u8]) -> Result<Vec<u8>, PlatformError> {
        self.0
            .unwrap(wrapped.to_vec(), context.to_vec())
            .map_err(platform_error)
    }
}

/// A secret operation failed.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SecretVaultError {
    /// The reference is not a valid name.
    #[error("{detail}")]
    InvalidReference {
        /// Which rule the reference broke.
        detail: String,
    },
    /// The secret could not be stored, read, or removed.
    #[error("{detail}")]
    Failed {
        /// What went wrong. It never contains secret values.
        detail: String,
    },
}

impl From<SecretsError> for SecretVaultError {
    fn from(error: SecretsError) -> Self {
        let detail = error.to_string();
        match error {
            SecretsError::InvalidReference { .. } => Self::InvalidReference { detail },
            _ => Self::Failed { detail },
        }
    }
}

/// Stores secrets wrapped by the platform keystore. Kotlin uses it to save a key and, after
/// a biometric check, to reveal it. Providers read keys inside the core, not through this.
#[derive(uniffi::Object)]
pub struct SecretVault {
    store: SecretStore,
}

#[uniffi::export]
impl SecretVault {
    /// Opens the vault over `dir` (the app's private `secrets` folder).
    #[uniffi::constructor]
    pub fn new(dir: String, platform: Arc<dyn SecretsPlatform>) -> Arc<Self> {
        Arc::new(Self {
            store: SecretStore::new(dir, Arc::new(PlatformAdapter(platform))),
        })
    }

    /// Stores `value` under `reference`, replacing any earlier value.
    ///
    /// # Errors
    /// Returns an error when the reference is invalid or the platform or file system fails.
    pub fn put(&self, reference: &str, value: &str) -> Result<(), SecretVaultError> {
        let reference = SecretRef::new(reference)?;
        Ok(self.store.put(&reference, &Secret::from_text(value))?)
    }

    /// The stored value, or `None` when there is none. Callers check biometrics first.
    ///
    /// # Errors
    /// Returns an error when the reference is invalid, the file is damaged, or the
    /// platform cannot unwrap it.
    pub fn reveal(&self, reference: &str) -> Result<Option<String>, SecretVaultError> {
        let reference = SecretRef::new(reference)?;
        let Some(secret) = self.store.get(&reference)? else {
            return Ok(None);
        };
        let text = secret.expose_text().map_err(|_| SecretVaultError::Failed {
            detail: "the stored secret is not text".to_owned(),
        })?;
        Ok(Some(text.to_owned()))
    }

    /// True when a secret exists under `reference`.
    ///
    /// # Errors
    /// Returns an error when the reference is invalid or the folder cannot be read.
    pub fn has(&self, reference: &str) -> Result<bool, SecretVaultError> {
        Ok(self.store.exists(&SecretRef::new(reference)?)?)
    }

    /// Removes the secret. Removing one that does not exist is not an error.
    ///
    /// # Errors
    /// Returns an error when the reference is invalid or the file cannot be removed.
    pub fn delete(&self, reference: &str) -> Result<(), SecretVaultError> {
        Ok(self.store.delete(&SecretRef::new(reference)?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scrambles bytes and tags them with the context. It is not encryption.
    struct Scramble;

    fn authentication_failed() -> SecretsPlatformFailure {
        SecretsPlatformFailure::Failed {
            detail: "authentication failed".into(),
        }
    }

    fn scramble(bytes: &[u8]) -> Vec<u8> {
        bytes.iter().map(|b| b ^ 0x33).collect()
    }

    fn unscramble(wrapped: &[u8], context: &[u8]) -> Result<Vec<u8>, SecretsPlatformFailure> {
        let (body, found) = wrapped.split_at(wrapped.len().saturating_sub(1));
        if found != [tag(context)] {
            return Err(authentication_failed());
        }
        Ok(scramble(body))
    }

    fn tag(context: &[u8]) -> u8 {
        context.iter().fold(7, |acc, b| acc.wrapping_add(*b))
    }

    impl SecretsPlatform for Scramble {
        fn wrap(
            &self,
            plaintext: Vec<u8>,
            context: Vec<u8>,
        ) -> Result<Vec<u8>, SecretsPlatformFailure> {
            let mut out = scramble(&plaintext);
            out.push(tag(&context));
            Ok(out)
        }

        fn unwrap(
            &self,
            wrapped: Vec<u8>,
            context: Vec<u8>,
        ) -> Result<Vec<u8>, SecretsPlatformFailure> {
            unscramble(&wrapped, &context)
        }
    }

    fn vault(dir: &tempfile::TempDir) -> Arc<SecretVault> {
        let folder = dir.path().join("secrets").to_string_lossy().into_owned();
        SecretVault::new(folder, Arc::new(Scramble))
    }

    #[test]
    fn a_secret_put_through_the_vault_is_revealed() {
        let dir = tempfile::tempdir().unwrap();
        let vault = vault(&dir);
        vault.put("provider-gemini", "AIza-test").unwrap();
        assert!(vault.has("provider-gemini").unwrap());
        assert_eq!(
            vault.reveal("provider-gemini").unwrap().as_deref(),
            Some("AIza-test")
        );
        vault.delete("provider-gemini").unwrap();
        assert_eq!(vault.reveal("provider-gemini").unwrap(), None);
    }

    #[test]
    fn a_bad_reference_is_reported_as_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let err = vault(&dir).put("../x", "v").unwrap_err();
        assert!(matches!(err, SecretVaultError::InvalidReference { .. }));
    }

    #[test]
    fn a_platform_failure_comes_back_as_a_failed_error() {
        let dir = tempfile::tempdir().unwrap();
        let vault = vault(&dir);
        vault.put("a", "v").unwrap();
        let folder = dir.path().join("secrets");
        std::fs::copy(folder.join("a.bin"), folder.join("b.bin")).unwrap();
        let err = vault.reveal("b").unwrap_err();
        assert!(matches!(err, SecretVaultError::Failed { .. }));
    }
}
