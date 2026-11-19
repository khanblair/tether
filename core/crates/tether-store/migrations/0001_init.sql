-- Schema 0001: projects, sessions, messages, provider settings, model catalog, usage.
--
-- IDs are ULIDs stored as text. Times are Unix milliseconds. Money is USD times 1,000,000.
-- Enum columns hold the text form from tether-types; a store test keeps these CHECK lists
-- in sync with it. Tables for tool calls, approvals, checkpoints, and the rest arrive in later
-- migrations with the tasks that use them.
--
-- This file may change until a release opens the database. After that, add a new migration.

CREATE TABLE projects (
    id               TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    -- Relative to files/workspaces/: not empty, not absolute, and no .. segment.
    path             TEXT NOT NULL UNIQUE
        CHECK (path <> '' AND substr(path, 1, 1) <> '/' AND instr('/' || path || '/', '/../') = 0),
    remote_url       TEXT,
    default_branch   TEXT,
    default_provider TEXT CHECK (default_provider IN ('anthropic', 'openai', 'gemini', 'deepseek', 'openrouter')),
    default_model_id TEXT,
    default_mode     TEXT CHECK (default_mode IN ('plan', 'ask', 'accept_edits', 'auto', 'bypass')),
    created_at       INTEGER NOT NULL,
    opened_at        INTEGER,
    CHECK ((default_provider IS NULL) = (default_model_id IS NULL))
) STRICT;

CREATE TABLE sessions (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    title       TEXT NOT NULL DEFAULT '',
    provider    TEXT NOT NULL CHECK (provider IN ('anthropic', 'openai', 'gemini', 'deepseek', 'openrouter')),
    model_id    TEXT NOT NULL,
    mode        TEXT NOT NULL CHECK (mode IN ('plan', 'ask', 'accept_edits', 'auto', 'bypass')),
    status      TEXT NOT NULL CHECK (status IN (
        'idle', 'requesting', 'streaming', 'checking_tool', 'running_tool',
        'awaiting_approval', 'paused', 'interrupted', 'failed')),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    cost_micros INTEGER NOT NULL DEFAULT 0 CHECK (cost_micros >= 0),
    tokens_in   INTEGER NOT NULL DEFAULT 0 CHECK (tokens_in >= 0),
    tokens_out  INTEGER NOT NULL DEFAULT 0 CHECK (tokens_out >= 0)
) STRICT;

CREATE INDEX sessions_by_project ON sessions (project_id, updated_at DESC);

CREATE TABLE messages (
    id         TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    role       TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'tool', 'system_note')),
    seq        INTEGER NOT NULL CHECK (seq >= 0),
    created_at INTEGER NOT NULL,
    rewound    INTEGER NOT NULL DEFAULT 0 CHECK (rewound IN (0, 1)),
    UNIQUE (session_id, seq)
) STRICT;

CREATE TABLE message_parts (
    id         TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES messages (id) ON DELETE CASCADE,
    seq        INTEGER NOT NULL CHECK (seq >= 0),
    kind       TEXT NOT NULL CHECK (kind IN (
        'text', 'image_ref', 'tool_call', 'tool_result', 'thinking', 'compaction_summary')),
    body_json  TEXT NOT NULL,
    UNIQUE (message_id, seq)
) STRICT;

-- No key material here: secret_ref names the wrapped secret.
CREATE TABLE provider_configs (
    provider   TEXT PRIMARY KEY CHECK (provider IN ('anthropic', 'openai', 'gemini', 'deepseek', 'openrouter')),
    secret_ref TEXT,
    base_url   TEXT,
    status     TEXT NOT NULL DEFAULT 'unset'
        CHECK (status IN ('unset', 'testing', 'ok', 'bad_key', 'network_error', 'blocked')),
    tested_at  INTEGER
) STRICT;

-- Catalog cache. Prices are USD times 1,000,000 per million tokens; NULL means unknown.
CREATE TABLE models (
    provider                TEXT NOT NULL CHECK (provider IN ('anthropic', 'openai', 'gemini', 'deepseek', 'openrouter')),
    model_id                TEXT NOT NULL CHECK (model_id <> ''),
    display_name            TEXT NOT NULL,
    context_window          INTEGER CHECK (context_window > 0),
    max_output_tokens       INTEGER CHECK (max_output_tokens > 0),
    input_price_micros      INTEGER CHECK (input_price_micros >= 0),
    output_price_micros     INTEGER CHECK (output_price_micros >= 0),
    cache_read_price_micros INTEGER CHECK (cache_read_price_micros >= 0),
    cache_write_price_micros INTEGER CHECK (cache_write_price_micros >= 0),
    price_source            TEXT CHECK (price_source IN ('provider', 'bundled')),
    caps_json               TEXT NOT NULL DEFAULT '{}',
    position                INTEGER NOT NULL CHECK (position >= 0),
    PRIMARY KEY (provider, model_id),
    CHECK ((input_price_micros IS NULL) = (output_price_micros IS NULL)),
    CHECK ((input_price_micros IS NULL) = (price_source IS NULL)),
    CHECK (input_price_micros IS NOT NULL OR (cache_read_price_micros IS NULL AND cache_write_price_micros IS NULL))
) STRICT;

CREATE TABLE models_fetch (
    provider   TEXT PRIMARY KEY CHECK (provider IN ('anthropic', 'openai', 'gemini', 'deepseek', 'openrouter')),
    fetched_at INTEGER NOT NULL
) STRICT;

-- No foreign key on project_id: cost history outlives a deleted project. Days are UTC dates.
CREATE TABLE usage_daily (
    day         TEXT NOT NULL CHECK (day GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    provider    TEXT NOT NULL CHECK (provider IN ('anthropic', 'openai', 'gemini', 'deepseek', 'openrouter')),
    model_id    TEXT NOT NULL,
    project_id  TEXT NOT NULL,
    tokens_in   INTEGER NOT NULL DEFAULT 0 CHECK (tokens_in >= 0),
    tokens_out  INTEGER NOT NULL DEFAULT 0 CHECK (tokens_out >= 0),
    cost_micros INTEGER NOT NULL DEFAULT 0 CHECK (cost_micros >= 0),
    PRIMARY KEY (day, provider, model_id, project_id)
) STRICT;
