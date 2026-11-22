//! Project trust flags.

use rusqlite::OptionalExtension;
use tether_types::ProjectId;

use crate::{Store, StoreError};

impl Store {
    /// Marks an existing project as trusted.
    ///
    /// # Errors
    /// Returns an error when the project is absent or SQLite fails.
    pub fn trust_project(&self, project_id: &ProjectId, trusted_at: i64) -> Result<(), StoreError> {
        let exists = self
            .conn
            .query_row(
                "SELECT 1 FROM projects WHERE id = ?1",
                [project_id.as_str()],
                |_| Ok(()),
            )
            .optional()?;
        if exists.is_none() {
            return Err(StoreError::MissingProject);
        }
        let changed = self.conn.execute(
            "INSERT INTO project_trust (project_id, trusted_at) VALUES (?1, ?2) ON CONFLICT(project_id) DO UPDATE SET trusted_at = excluded.trusted_at",
            rusqlite::params![project_id.as_str(), trusted_at],
        )?;
        if changed == 0 {
            return Err(StoreError::MissingProject);
        }
        Ok(())
    }

    /// Removes a project's trust flag.
    ///
    /// # Errors
    /// Returns an error when SQLite fails.
    pub fn revoke_project_trust(&self, project_id: &ProjectId) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM project_trust WHERE project_id = ?1",
            [project_id.as_str()],
        )?;
        Ok(())
    }

    /// Returns whether the project is trusted.
    ///
    /// # Errors
    /// Returns an error when SQLite fails.
    pub fn is_project_trusted(&self, project_id: &ProjectId) -> Result<bool, StoreError> {
        self.conn
            .query_row(
                "SELECT 1 FROM project_trust WHERE project_id = ?1",
                [project_id.as_str()],
                |_| Ok(()),
            )
            .optional()
            .map(|value| value.is_some())
            .map_err(StoreError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_can_be_set_read_cleared_and_cascades_on_delete() {
        let mut store = Store::open_in_memory().unwrap();
        let project = store.create_project("Trust", 1).unwrap();

        assert!(!store.is_project_trusted(&project.id).unwrap());
        store.trust_project(&project.id, 2).unwrap();
        assert!(store.is_project_trusted(&project.id).unwrap());
        store.revoke_project_trust(&project.id).unwrap();
        assert!(!store.is_project_trusted(&project.id).unwrap());
        store.trust_project(&project.id, 3).unwrap();
        store.delete_project(&project.id).unwrap();
        assert!(!store.is_project_trusted(&project.id).unwrap());
    }
}
