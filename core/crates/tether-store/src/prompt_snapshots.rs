//! Frozen prompt blocks stored per session.

use rusqlite::{OptionalExtension, params};
use tether_types::SessionId;

use crate::{Store, StoreError};

/// The memory and vault blocks captured when a session first opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSnapshot {
    /// Frozen memory system block.
    pub memory_block: String,
    /// Frozen vault system block.
    pub vault_block: String,
    /// Frozen skills index.
    pub skills_block: String,
    /// Capture time in Unix milliseconds.
    pub created_at: i64,
}

impl Store {
    /// Saves a snapshot once. Existing snapshots are never overwritten.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot write the row.
    #[expect(
        clippy::too_many_arguments,
        reason = "snapshot columns remain explicit at the storage boundary"
    )]
    pub fn save_prompt_snapshot(
        &self,
        session_id: &SessionId,
        memory_block: &str,
        vault_block: &str,
        skills_block: &str,
        created_at: i64,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO session_prompt_snapshot
             (session_id, memory_block, vault_block, skills_block, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session_id.as_str(),
                memory_block,
                vault_block,
                skills_block,
                created_at
            ],
        )?;
        Ok(())
    }

    /// Loads the frozen blocks for a session, if one has been captured.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the row.
    pub fn load_prompt_snapshot(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<PromptSnapshot>, StoreError> {
        self.conn
            .query_row(
                "SELECT memory_block, vault_block, skills_block, created_at
                 FROM session_prompt_snapshot WHERE session_id = ?1",
                [session_id.as_str()],
                |row| {
                    Ok(PromptSnapshot {
                        memory_block: row.get(0)?,
                        vault_block: row.get(1)?,
                        skills_block: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
    use tether_types::{ModelRef, PermissionMode, Provider};

    fn session(store: &mut Store) -> SessionId {
        let project = store.create_project("Prompt", 1).unwrap();
        store
            .create_session(
                &project.id,
                &ModelRef {
                    provider: Provider::OpenAi,
                    model_id: "model".into(),
                },
                PermissionMode::Ask,
                1,
            )
            .unwrap()
            .id
    }

    #[test]
    fn snapshot_saves_once_and_cascades_with_the_session() {
        let mut store = Store::open_in_memory().unwrap();
        let id = session(&mut store);
        store
            .save_prompt_snapshot(&id, "first", "vault", "skills", 10)
            .unwrap();
        store
            .save_prompt_snapshot(&id, "second", "new", "new-skills", 20)
            .unwrap();

        assert_eq!(
            store.load_prompt_snapshot(&id).unwrap(),
            Some(PromptSnapshot {
                memory_block: "first".into(),
                vault_block: "vault".into(),
                skills_block: "skills".into(),
                created_at: 10,
            })
        );
        let project = store.load_session(&id).unwrap().unwrap().project_id;
        store.delete_project(&project).unwrap();
        assert_eq!(store.load_prompt_snapshot(&id).unwrap(), None);
    }
}
