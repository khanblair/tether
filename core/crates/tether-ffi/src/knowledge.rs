//! FFI settings and staged diff access for knowledge tools.

use tether_types::{SessionId, ToolCallId};

use crate::sessions::{SessionRuntime, SessionRuntimeError};

#[uniffi::export]
impl SessionRuntime {
    /// Returns whether skill writes ask for approval.
    ///
    /// # Errors
    /// Returns `Unavailable` when the store cannot be read.
    pub fn skill_write_approval(&self) -> Result<bool, SessionRuntimeError> {
        self.store()?
            .skill_write_approval()
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Sets whether skill writes ask for approval.
    ///
    /// # Errors
    /// Returns `Unavailable` when the store cannot be written.
    pub fn set_skill_write_approval(&self, enabled: bool) -> Result<(), SessionRuntimeError> {
        self.store()?
            .set_write_approval("skill_write_approval", enabled)
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Returns whether memory writes ask for approval.
    ///
    /// # Errors
    /// Returns `Unavailable` when the store cannot be read.
    pub fn memory_write_approval(&self) -> Result<bool, SessionRuntimeError> {
        self.store()?
            .memory_write_approval()
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Sets whether memory writes ask for approval.
    ///
    /// # Errors
    /// Returns `Unavailable` when the store cannot be written.
    pub fn set_memory_write_approval(&self, enabled: bool) -> Result<(), SessionRuntimeError> {
        self.store()?
            .set_write_approval("memory_write_approval", enabled)
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Returns whether vault writes ask for approval.
    ///
    /// # Errors
    /// Returns `Unavailable` when the store cannot be read.
    pub fn vault_write_approval(&self) -> Result<bool, SessionRuntimeError> {
        self.store()?
            .vault_write_approval()
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Sets whether vault writes ask for approval.
    ///
    /// # Errors
    /// Returns `Unavailable` when the store cannot be written.
    pub fn set_vault_write_approval(&self, enabled: bool) -> Result<(), SessionRuntimeError> {
        self.store()?
            .set_write_approval("vault_write_approval", enabled)
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Enables or disables a named skill.
    ///
    /// # Errors
    /// Returns `Unavailable` when the store cannot be written.
    pub fn set_skill_enabled(&self, name: &str, enabled: bool) -> Result<(), SessionRuntimeError> {
        self.store()?
            .set_skill_enabled(name, enabled)
            .map_err(|_| SessionRuntimeError::Unavailable)
    }

    /// Returns a staged skill diff for the approval screen.
    ///
    /// # Errors
    /// Returns `Unavailable` when the store cannot be read.
    pub fn skill_change_diff(
        &self,
        session_id: String,
        tool_call_id: String,
    ) -> Result<String, SessionRuntimeError> {
        self.store()?
            .skill_change_diff(&SessionId::new(session_id), &ToolCallId::new(tool_call_id))
            .map(Option::unwrap_or_default)
            .map_err(|_| SessionRuntimeError::Unavailable)
    }
}
