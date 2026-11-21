-- Schema 0011: context compaction records and the global threshold.

CREATE TABLE compactions (
    id                 TEXT PRIMARY KEY,
    session_id         TEXT NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    summary_message_id TEXT NOT NULL REFERENCES messages (id) ON DELETE CASCADE,
    first_message_id   TEXT NOT NULL REFERENCES messages (id) ON DELETE CASCADE,
    last_message_id    TEXT NOT NULL REFERENCES messages (id) ON DELETE CASCADE,
    tokens_before      INTEGER NOT NULL CHECK (tokens_before >= 0),
    tokens_after       INTEGER NOT NULL CHECK (tokens_after >= 0),
    created_at         INTEGER NOT NULL,
    focus              TEXT
) STRICT;

CREATE INDEX compactions_by_session ON compactions (session_id, created_at);

CREATE TABLE context_settings (
    id                 INTEGER PRIMARY KEY CHECK (id = 1),
    compact_at_percent INTEGER NOT NULL CHECK (compact_at_percent BETWEEN 50 AND 95)
) STRICT;

INSERT INTO context_settings (id, compact_at_percent) VALUES (1, 80);
