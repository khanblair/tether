//! Provider key lookup and provider factory for a session.

use std::sync::Arc;

use tether_engine::{ProviderFactory, provider_for};
use tether_secrets::SecretRef;
use tether_types::{CoreError, Provider};

use super::SessionRuntimeError;
use crate::SecretVault;

pub(super) fn provider_key(
    vault: &SecretVault,
    provider: Provider,
) -> Result<tether_secrets::Secret, SessionRuntimeError> {
    let reference = SecretRef::new(&format!("provider-{}", provider.as_str()))
        .map_err(|_| SessionRuntimeError::Unavailable)?;
    vault
        .store
        .get(&reference)
        .map_err(|_| SessionRuntimeError::Unavailable)?
        .ok_or(SessionRuntimeError::MissingKey)
}

pub(super) fn session_provider_factory(
    vault: Arc<SecretVault>,
    provider: Provider,
) -> ProviderFactory {
    Arc::new(move || {
        let key = provider_key(&vault, provider).map_err(|error| match error {
            SessionRuntimeError::MissingKey => CoreError::Auth {
                message: "provider key is not saved".into(),
            },
            _ => CoreError::Internal {
                message: "provider key is unavailable".into(),
            },
        })?;
        let key_text = key.expose_text().map_err(|_| CoreError::Internal {
            message: "provider key is unavailable".into(),
        })?;
        provider_for(provider, key_text).map_err(CoreError::from)
    })
}
