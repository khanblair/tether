CREATE TABLE approvals (
    id           TEXT PRIMARY KEY,
    session_id   TEXT NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    tool_call_id TEXT NOT NULL,
    tool         TEXT NOT NULL,
    target       TEXT NOT NULL,
    decision     TEXT NOT NULL CHECK (decision IN ('allow', 'deny')),
    reason       TEXT,
    decided_at   INTEGER NOT NULL,
    via          TEXT NOT NULL CHECK (via IN ('app', 'notification'))
) STRICT;

CREATE INDEX approvals_by_session ON approvals (session_id, decided_at);

CREATE TABLE permission_rules (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    tool       TEXT NOT NULL,
    pattern    TEXT NOT NULL,
    executor   TEXT CHECK (executor IN ('sandbox', 'linux', 'ci')),
    created_at INTEGER NOT NULL,
    UNIQUE (project_id, tool, pattern, executor)
) STRICT;

CREATE UNIQUE INDEX permission_rules_unique_rule
ON permission_rules (project_id, tool, pattern, COALESCE(executor, ''));

CREATE INDEX permission_rules_by_project ON permission_rules (project_id, created_at);
