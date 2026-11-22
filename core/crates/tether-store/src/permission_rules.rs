//! Project-scoped permission rule persistence.

use rusqlite::{OptionalExtension, params};
use tether_types::{Executor, NewRule, PermissionRule, ProjectId};
use ulid::Ulid;

use crate::{Store, StoreError};

impl Store {
    /// Adds a project rule, returning the existing row for a duplicate.
    ///
    /// # Errors
    /// Returns an error when the project is absent or SQLite cannot write the row.
    pub fn add_permission_rule(
        &self,
        project_id: &ProjectId,
        rule: &NewRule,
        created_at: i64,
    ) -> Result<PermissionRule, StoreError> {
        let id = Ulid::new().to_string();
        self.conn.execute(
            "INSERT OR IGNORE INTO permission_rules (id, project_id, tool, pattern, executor, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, project_id.as_str(), rule.tool, rule.pattern, rule.executor.map(Executor::as_str), created_at],
        )?;
        self.conn.query_row(
            "SELECT id, tool, pattern, executor FROM permission_rules WHERE project_id = ?1 AND tool = ?2 AND pattern = ?3 AND executor IS ?4",
            params![project_id.as_str(), rule.tool, rule.pattern, rule.executor.map(Executor::as_str)],
            |row| Ok(PermissionRule { id: row.get(0)?, tool: row.get(1)?, pattern: row.get(2)?, executor: parse_executor(row.get(3)?)? }),
        ).optional()?.ok_or(StoreError::MissingProject)
    }

    /// Lists rules belonging to one project.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the rows.
    pub fn permission_rules(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<PermissionRule>, StoreError> {
        let mut statement = self.conn.prepare("SELECT id, tool, pattern, executor FROM permission_rules WHERE project_id = ?1 ORDER BY created_at, id")?;
        let rows = statement.query_map([project_id.as_str()], |row| {
            Ok(PermissionRule {
                id: row.get(0)?,
                tool: row.get(1)?,
                pattern: row.get(2)?,
                executor: parse_executor(row.get(3)?)?,
            })
        })?;
        rows.map(|row| row.map_err(StoreError::from)).collect()
    }

    /// Deletes one rule only when it belongs to the selected project.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot delete the row.
    pub fn delete_permission_rule(
        &self,
        project_id: &ProjectId,
        rule_id: &str,
    ) -> Result<bool, StoreError> {
        Ok(self.conn.execute(
            "DELETE FROM permission_rules WHERE project_id = ?1 AND id = ?2",
            params![project_id.as_str(), rule_id],
        )? != 0)
    }
}

fn parse_executor(value: Option<String>) -> rusqlite::Result<Option<Executor>> {
    value.map_or(Ok(None), |value| {
        value
            .parse()
            .map(Some)
            .map_err(|_| rusqlite::Error::InvalidQuery)
    })
}
