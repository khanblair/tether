-- One checkpoint records file content immediately before an edit batch.
-- Missing files have existed = 0 and no blob hash.
CREATE TABLE checkpoints (
    id         TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    message_id TEXT REFERENCES messages (id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE INDEX checkpoints_by_session ON checkpoints (session_id, created_at DESC);

CREATE TABLE checkpoint_files (
    checkpoint_id TEXT NOT NULL REFERENCES checkpoints (id) ON DELETE CASCADE,
    path          TEXT NOT NULL,
    blob_hash     TEXT,
    existed       INTEGER NOT NULL CHECK (existed IN (0, 1)),
    PRIMARY KEY (checkpoint_id, path),
    CHECK ((blob_hash IS NULL) = (existed = 0))
) STRICT;
