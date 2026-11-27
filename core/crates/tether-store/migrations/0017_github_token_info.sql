-- Schema 0017: persisted GitHub token kind, scopes, and repository reach.

CREATE TABLE github_token_info (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    kind            TEXT NOT NULL CHECK (kind IN ('classic', 'fine_grained')),
    scopes_json     TEXT NOT NULL CHECK (json_valid(scopes_json)),
    login           TEXT NOT NULL CHECK (login <> ''),
    can_read_repos  INTEGER NOT NULL CHECK (can_read_repos IN (0, 1)),
    checked_at      INTEGER NOT NULL
) STRICT;
