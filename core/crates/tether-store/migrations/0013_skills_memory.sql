-- Schema 0013: skill settings, staged skill writes, usage, and the frozen skills block.

CREATE TABLE skill_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

CREATE TABLE skill_disabled (
    name TEXT PRIMARY KEY
) STRICT;

CREATE TABLE skill_changes (
    id           TEXT PRIMARY KEY,
    session_id   TEXT NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    tool_call_id TEXT NOT NULL,
    action       TEXT NOT NULL,
    name         TEXT NOT NULL,
    diff         TEXT NOT NULL,
    created_at   INTEGER NOT NULL
) STRICT;

CREATE TABLE skill_usage (
    name          TEXT PRIMARY KEY,
    last_viewed_at INTEGER NOT NULL,
    views          INTEGER NOT NULL CHECK (views >= 0),
    created_at    INTEGER NOT NULL
) STRICT;

ALTER TABLE session_prompt_snapshot ADD COLUMN skills_block TEXT NOT NULL DEFAULT '';
