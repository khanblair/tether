-- Schema 0012: MCP server settings and secret references.

CREATE TABLE mcp_servers (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE CHECK (length(name) BETWEEN 1 AND 40),
    url        TEXT NOT NULL,
    enabled    INTEGER NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE mcp_server_headers (
    server_id   TEXT NOT NULL REFERENCES mcp_servers (id) ON DELETE CASCADE,
    header_name TEXT NOT NULL,
    secret_ref  TEXT NOT NULL,
    PRIMARY KEY (server_id, header_name)
) STRICT;

CREATE TABLE mcp_tool_settings (
    server_id TEXT NOT NULL REFERENCES mcp_servers (id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL,
    enabled   INTEGER NOT NULL,
    approval  TEXT NOT NULL CHECK (approval IN ('ask', 'auto')),
    PRIMARY KEY (server_id, tool_name)
) STRICT;
