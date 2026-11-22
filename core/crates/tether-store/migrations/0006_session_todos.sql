CREATE TABLE session_todos (
    session_id TEXT NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    position   INTEGER NOT NULL CHECK (position >= 0),
    text       TEXT NOT NULL,
    status     TEXT NOT NULL CHECK (status IN ('pending', 'in_progress', 'completed')),
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (session_id, position)
) STRICT;
