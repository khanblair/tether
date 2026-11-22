CREATE TABLE project_network (
    project_id TEXT PRIMARY KEY REFERENCES projects (id) ON DELETE CASCADE,
    allowed    INTEGER NOT NULL CHECK (allowed IN (0, 1)),
    updated_at INTEGER NOT NULL
) STRICT;
