//! One-time recovery of turns left active by a process that died.

use std::path::Path;

use tether_store::Store;

use crate::SessionRuntimeError;

/// Marks every session left mid-turn by a dead process as interrupted.
///
/// Call this once when the app process starts, before any runtime is built.
/// Building a `SessionRuntime` never does this, so a runtime built while a
/// turn streams leaves that turn's status alone.
///
/// Returns the number of sessions that changed status. Calling it again
/// returns 0 when nothing is left active.
///
/// # Errors
/// Returns `Unavailable` when the database cannot be opened or SQLite fails.
#[uniffi::export]
pub fn recover_interrupted_sessions(database_path: &str) -> Result<u32, SessionRuntimeError> {
    let store =
        Store::open(Path::new(database_path)).map_err(|_| SessionRuntimeError::Unavailable)?;
    let changed = store
        .recover_active_sessions()
        .map_err(|_| SessionRuntimeError::Unavailable)?;
    Ok(u32::try_from(changed).unwrap_or(u32::MAX))
}
