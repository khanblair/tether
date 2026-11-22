//! Network platform controls.

use std::sync::Arc;

use crate::callbacks::NetworkPlatform;
use crate::{SessionRuntime, SessionRuntimeError};

#[uniffi::export]
impl SessionRuntime {
    /// Sets the platform network callback and reads its initial state.
    ///
    /// # Errors
    /// Returns `Unavailable` when the callback cannot be stored.
    #[expect(clippy::needless_pass_by_value, reason = "UniFFI passes owned values")]
    pub fn set_network_platform(
        &self,
        platform: Arc<dyn NetworkPlatform>,
    ) -> Result<(), SessionRuntimeError> {
        self.network.enable(platform.is_online());
        Ok(())
    }

    /// Wakes paused turns after a connectivity change.
    pub fn network_changed(&self, online: bool) {
        self.network.set_online(online);
    }
}
