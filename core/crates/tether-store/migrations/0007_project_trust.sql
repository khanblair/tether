CREATE TABLE project_trust (
    project_id TEXT PRIMARY KEY REFERENCES projects (id) ON DELETE CASCADE,
    trusted_at INTEGER NOT NULL
) STRICT;
