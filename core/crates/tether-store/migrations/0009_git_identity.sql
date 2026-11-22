-- Schema 0009: the user's Git commit identity.

CREATE TABLE git_identity (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    login      TEXT,
    name       TEXT NOT NULL,
    email      TEXT NOT NULL,
    source     TEXT NOT NULL CHECK (source IN ('github', 'manual')),
    updated_at INTEGER NOT NULL
) STRICT;
