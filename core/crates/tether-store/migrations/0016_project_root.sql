-- Schema 0016: per-project roots and the saved root for new projects.

ALTER TABLE projects ADD COLUMN root TEXT CHECK (
    root IS NULL
    OR (
        substr(root, 1, 1) = '/'
        AND root NOT LIKE '%/../%'
        AND root NOT LIKE '%/..'
    )
);

CREATE TABLE app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;
