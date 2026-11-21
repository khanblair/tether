CREATE TABLE session_prompt_snapshot (
    session_id   TEXT PRIMARY KEY REFERENCES sessions (id) ON DELETE CASCADE,
    memory_block TEXT NOT NULL,
    vault_block  TEXT NOT NULL,
    created_at   INTEGER NOT NULL
) STRICT;
