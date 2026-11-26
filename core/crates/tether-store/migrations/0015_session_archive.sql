-- Schema 0015: archived sessions. An archived session stays in the database and can be
-- restored; lists and search leave it out.

ALTER TABLE sessions ADD COLUMN archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1));
