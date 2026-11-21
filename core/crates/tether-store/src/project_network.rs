//! Per-project Linux network permission.

use tether_types::ProjectId;

use crate::{Store, StoreError};

impl Store {
    /// Reads the project network setting. New projects default to off.
    ///
    /// # Errors
    /// Returns an error when the project is missing or the database cannot be read.
    pub fn project_network(&self, project_id: &ProjectId) -> Result<bool, StoreError> {
        self.conn
            .query_row(
                "SELECT COALESCE((SELECT allowed FROM project_network WHERE project_id = p.id), 0) FROM projects p WHERE p.id = ?1",
                [project_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map(|allowed| allowed != 0)
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => StoreError::MissingProject,
                error => StoreError::Sqlite(error),
            })
    }

    /// Saves the project network setting.
    ///
    /// # Errors
    /// Returns an error when the project is missing or the database cannot be written.
    pub fn set_project_network(
        &self,
        project_id: &ProjectId,
        allowed: bool,
        updated_at: i64,
    ) -> Result<(), StoreError> {
        if self.project_path(project_id)?.is_none() {
            return Err(StoreError::MissingProject);
        }
        let changed = self.conn.execute(
            "INSERT INTO project_network (project_id, allowed, updated_at) VALUES (?1, ?2, ?3) ON CONFLICT(project_id) DO UPDATE SET allowed = excluded.allowed, updated_at = excluded.updated_at",
            rusqlite::params![project_id.as_str(), i64::from(allowed), updated_at],
        )?;
        if changed == 0 {
            return Err(StoreError::MissingProject);
        }
        Ok(())
    }
}
