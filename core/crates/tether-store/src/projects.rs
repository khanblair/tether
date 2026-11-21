//! Project rows and workspace directory names.

use rusqlite::{OptionalExtension, Row, params};
use tether_types::ProjectId;
use ulid::Ulid;

use crate::{Store, StoreError};

const PROJECT_COLUMNS: &str = "SELECT p.id, p.name, p.path, p.created_at,
    COALESCE(MAX(s.updated_at), p.created_at), COUNT(s.id)
    FROM projects p LEFT JOIN sessions s ON s.project_id = p.id";

/// A saved project and its current session activity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    /// Project id.
    pub id: ProjectId,
    /// Trimmed display name.
    pub name: String,
    /// Directory name relative to the workspaces root.
    pub path: String,
    /// Creation time in Unix milliseconds.
    pub created_at: i64,
    /// Newest session update time, or creation time when empty.
    pub last_activity_at: i64,
    /// Number of saved sessions.
    pub session_count: u32,
}

type ProjectRow = (String, String, String, i64, i64, i64);

impl Store {
    /// Creates a project with a unique, safe workspace directory name.
    ///
    /// # Errors
    /// Returns an error for an invalid name or a database failure.
    pub fn create_project(&mut self, name: &str, now_ms: i64) -> Result<ProjectRecord, StoreError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(StoreError::InvalidProjectName);
        }
        let id = ProjectId::new(Ulid::new().to_string());
        let path = project_path(name, &id);
        self.conn.execute(
            "INSERT INTO projects (id, name, path, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id.as_str(), name, path, now_ms],
        )?;
        Ok(ProjectRecord {
            id,
            name: name.into(),
            path,
            created_at: now_ms,
            last_activity_at: now_ms,
            session_count: 0,
        })
    }

    /// Lists projects by newest session activity, with id descending for ties.
    ///
    /// # Errors
    /// Returns an error when the database cannot be read or a count overflows.
    pub fn list_projects(&self) -> Result<Vec<ProjectRecord>, StoreError> {
        let sql = format!("{PROJECT_COLUMNS} GROUP BY p.id ORDER BY 5 DESC, p.id DESC");
        let mut statement = self.conn.prepare(&sql)?;
        let rows = statement.query_map([], project_row)?;
        rows.map(|row| row.map_err(StoreError::from).and_then(decode_project))
            .collect()
    }

    /// Loads a project by id, or `None` when absent.
    ///
    /// # Errors
    /// Returns an error when the database cannot be read or a count overflows.
    pub fn load_project(&self, id: &ProjectId) -> Result<Option<ProjectRecord>, StoreError> {
        let sql = format!("{PROJECT_COLUMNS} WHERE p.id = ?1 GROUP BY p.id");
        self.conn
            .query_row(&sql, [id.as_str()], project_row)
            .optional()?
            .map(decode_project)
            .transpose()
    }

    /// Removes a project row after workspace creation fails.
    ///
    /// # Errors
    /// Returns an error when the database cannot remove the row.
    pub fn delete_project(&mut self, id: &ProjectId) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM projects WHERE id = ?1", [id.as_str()])?;
        Ok(())
    }
}

fn project_row(row: &Row<'_>) -> rusqlite::Result<ProjectRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn decode_project(row: ProjectRow) -> Result<ProjectRecord, StoreError> {
    Ok(ProjectRecord {
        id: ProjectId::new(row.0),
        name: row.1,
        path: row.2,
        created_at: row.3,
        last_activity_at: row.4,
        session_count: u32::try_from(row.5).map_err(|_| StoreError::SessionCounterOverflow)?,
    })
}

fn project_path(name: &str, id: &ProjectId) -> String {
    let mut slug = String::new();
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' {
            slug.push(ch);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "project" } else { slug };
    let suffix = id
        .as_str()
        .get(id.as_str().len() - 8..)
        .unwrap_or(id.as_str());
    format!(
        "{}-{}",
        slug.chars().take(40).collect::<String>(),
        suffix.to_ascii_lowercase()
    )
}
