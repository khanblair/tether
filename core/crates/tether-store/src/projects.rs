//! Project rows and workspace directory names.

use rusqlite::{OptionalExtension, Row, params};
use tether_types::{ModelRef, PermissionMode, ProjectId, Provider};
use ulid::Ulid;

use crate::{Store, StoreError};

const PROJECT_COLUMNS: &str = "SELECT p.id, p.name, p.path, p.created_at,
    COALESCE(MAX(s.updated_at), p.created_at), COUNT(s.id),
    p.default_provider, p.default_model_id, p.default_mode
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
    /// Default provider for new sessions, when a model is saved.
    pub default_provider: Option<Provider>,
    /// Default model id for new sessions, when a model is saved.
    pub default_model_id: Option<String>,
    /// Default permission mode for new sessions.
    pub default_mode: Option<PermissionMode>,
}

/// Optional project settings to apply when a new session starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDefaults {
    /// Saved model reference, if set.
    pub model: Option<ModelRef>,
    /// Saved permission mode, if set.
    pub mode: Option<PermissionMode>,
}

/// The remote metadata saved for one project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRemote {
    /// The repository's remote URL.
    pub remote_url: Option<String>,
    /// The repository's default branch.
    pub default_branch: Option<String>,
}

/// The commit identity saved for the current user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitIdentity {
    /// GitHub login, when the identity came from GitHub.
    pub login: Option<String>,
    /// Author and committer name.
    pub name: String,
    /// Author and committer email.
    pub email: String,
    /// Identity source, either `github` or `manual`.
    pub source: String,
    /// Update time in Unix milliseconds.
    pub updated_at: i64,
}

type ProjectRow = (
    String,
    String,
    String,
    i64,
    i64,
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
);

impl Store {
    /// Reads a project's remote URL and default branch.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the project.
    pub fn project_remote(&self, id: &ProjectId) -> Result<Option<ProjectRemote>, StoreError> {
        self.conn
            .query_row(
                "SELECT remote_url, default_branch FROM projects WHERE id = ?1",
                [id.as_str()],
                |row| {
                    Ok(ProjectRemote {
                        remote_url: row.get(0)?,
                        default_branch: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Saves a project's remote URL and default branch.
    ///
    /// # Errors
    /// Returns an error when the project is absent or SQLite cannot update it.
    pub fn set_project_remote(
        &self,
        id: &ProjectId,
        remote_url: &str,
        default_branch: &str,
    ) -> Result<(), StoreError> {
        let changed = self.conn.execute(
            "UPDATE projects SET remote_url = ?2, default_branch = ?3 WHERE id = ?1",
            params![id.as_str(), remote_url, default_branch],
        )?;
        (changed == 1)
            .then_some(())
            .ok_or(StoreError::MissingProject)
    }

    /// Reads the single saved Git identity.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the identity.
    pub fn git_identity(&self) -> Result<Option<GitIdentity>, StoreError> {
        self.conn
            .query_row(
                "SELECT login, name, email, source, updated_at FROM git_identity WHERE id = 1",
                [],
                |row| {
                    Ok(GitIdentity {
                        login: row.get(0)?,
                        name: row.get(1)?,
                        email: row.get(2)?,
                        source: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Replaces the saved Git identity.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot write the identity.
    pub fn set_git_identity(&self, identity: &GitIdentity) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO git_identity (id, login, name, email, source, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET login = excluded.login, name = excluded.name,
                 email = excluded.email, source = excluded.source, updated_at = excluded.updated_at",
            params![
                identity.login,
                identity.name,
                identity.email,
                identity.source,
                identity.updated_at
            ],
        )?;
        Ok(())
    }

    /// Removes the saved Git identity.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot delete the identity.
    pub fn clear_git_identity(&self) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM git_identity WHERE id = 1", [])?;
        Ok(())
    }

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
            default_provider: None,
            default_model_id: None,
            default_mode: None,
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

    /// Saves a project's model and permission defaults. A missing model clears both model columns.
    ///
    /// # Errors
    /// Returns an error when the project is absent or SQLite cannot update it.
    pub fn set_project_defaults(
        &self,
        id: &ProjectId,
        model: Option<ModelRef>,
        mode: Option<PermissionMode>,
    ) -> Result<(), StoreError> {
        if mode == Some(PermissionMode::Bypass) {
            return Err(StoreError::BypassNeedsConfirmation);
        }
        let (provider, model_id) = model.map_or((None, None), |value| {
            (Some(value.provider), Some(value.model_id))
        });
        let changed = self.conn.execute(
            "UPDATE projects SET default_provider = ?2, default_model_id = ?3, default_mode = ?4 WHERE id = ?1",
            params![
                id.as_str(),
                provider.map(Provider::as_str),
                model_id.as_deref(),
                mode.map(PermissionMode::as_str),
            ],
        )?;
        if changed == 0 {
            return Err(StoreError::MissingProject);
        }
        Ok(())
    }

    /// Reads the defaults for a new session without changing `create_session`.
    ///
    /// # Errors
    /// Returns an error when the project is absent or its row cannot be read.
    pub fn session_defaults(&self, id: &ProjectId) -> Result<ProjectDefaults, StoreError> {
        let project = self.load_project(id)?.ok_or(StoreError::MissingProject)?;
        let model = project
            .default_provider
            .zip(project.default_model_id)
            .map(|(provider, model_id)| ModelRef { provider, model_id });
        Ok(ProjectDefaults {
            model,
            mode: project.default_mode,
        })
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
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
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
        default_provider: row
            .6
            .map(|value| crate::sessions::parse_enum("default_provider", value))
            .transpose()?,
        default_model_id: row.7,
        default_mode: row
            .8
            .map(|value| crate::sessions::parse_enum("default_mode", value))
            .transpose()?,
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
