-- Schema 0010: budget limits and whether session prices were known.

ALTER TABLE sessions ADD COLUMN price_known INTEGER NOT NULL DEFAULT 1 CHECK (price_known IN (0, 1));

CREATE TABLE budget_settings (
    id                  INTEGER PRIMARY KEY CHECK (id = 1),
    daily_limit_micros  INTEGER CHECK (daily_limit_micros >= 10000 AND daily_limit_micros <= 10000000000),
    session_limit_micros INTEGER CHECK (session_limit_micros >= 10000 AND session_limit_micros <= 10000000000),
    updated_at          INTEGER NOT NULL
) STRICT;

CREATE TABLE session_budget_overrides (
    session_id            TEXT PRIMARY KEY REFERENCES sessions (id) ON DELETE CASCADE,
    limit_micros          INTEGER NOT NULL CHECK (limit_micros >= 10000 AND limit_micros <= 10000000000),
    continued_past_micros INTEGER CHECK (continued_past_micros >= 0)
) STRICT;
