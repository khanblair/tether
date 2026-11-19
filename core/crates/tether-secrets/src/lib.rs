//! Secret references wrapped by the platform keystore.
//!
//! The core never holds an encryption key. [`SecretStore`] asks the platform (Android
//! Keystore today) to wrap each secret, and keeps only the wrapped bytes in
//! `secrets/<reference>.bin`.

mod error;
mod platform;
mod reference;
mod secret;
mod store;

pub use error::SecretsError;
pub use platform::{PlatformError, SecretsPlatform};
pub use reference::SecretRef;
pub use secret::Secret;
pub use store::SecretStore;

#[cfg(test)]
mod store_tests;
