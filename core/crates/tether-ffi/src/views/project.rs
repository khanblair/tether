//! A project's public fields for the platform.

use tether_store::ProjectRecord;

/// Project details and session activity.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProjectView {
    /// Project id.
    pub id: String,
    /// Trimmed display name.
    pub name: String,
    /// Directory name relative to the workspaces root.
    pub path: String,
    /// Creation time in Unix milliseconds.
    pub created_at_ms: i64,
    /// Newest session activity, or creation time when empty.
    pub last_activity_ms: i64,
    /// Number of sessions.
    pub session_count: u32,
}

impl From<ProjectRecord> for ProjectView {
    fn from(project: ProjectRecord) -> Self {
        Self {
            id: project.id.into(),
            name: project.name,
            path: project.path,
            created_at_ms: project.created_at,
            last_activity_ms: project.last_activity_at,
            session_count: project.session_count,
        }
    }
}
