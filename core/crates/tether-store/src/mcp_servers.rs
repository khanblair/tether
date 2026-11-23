//! MCP server, header-reference, and tool-setting queries.

use rusqlite::{OptionalExtension, params};
use ulid::Ulid;

use crate::{Store, StoreError};

/// A saved MCP server without its sensitive header values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpServerRecord {
    /// Stable server id.
    pub id: String,
    /// User-facing server name.
    pub name: String,
    /// HTTPS endpoint.
    pub url: String,
    /// Whether new sessions may use the server.
    pub enabled: bool,
    /// Creation time in Unix milliseconds.
    pub created_at: i64,
}

/// A saved MCP header name and its secret-store reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpServerHeaderRecord {
    /// Owning server id.
    pub server_id: String,
    /// Header name. The value is never stored here.
    pub header_name: String,
    /// Reference into the secret store.
    pub secret_ref: String,
}

/// A saved per-tool MCP setting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpToolSettingRecord {
    /// Owning server id.
    pub server_id: String,
    /// Server-advertised tool name.
    pub tool_name: String,
    /// Whether the tool is offered to models.
    pub enabled: bool,
    /// Approval policy, either `ask` or `auto`.
    pub approval: String,
}

impl Store {
    /// Creates an enabled MCP server with a generated id.
    ///
    /// # Errors
    /// Returns an error when SQLite rejects the row.
    pub fn create_mcp_server(
        &self,
        name: &str,
        url: &str,
        enabled: bool,
        created_at: i64,
    ) -> Result<McpServerRecord, StoreError> {
        let id = Ulid::new().to_string();
        self.conn.execute(
            "INSERT INTO mcp_servers (id, name, url, enabled, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, url, i64::from(enabled), created_at],
        )?;
        Ok(McpServerRecord {
            id,
            name: name.to_owned(),
            url: url.to_owned(),
            enabled,
            created_at,
        })
    }

    /// Loads one MCP server without its header values.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the row.
    pub fn load_mcp_server(&self, id: &str) -> Result<Option<McpServerRecord>, StoreError> {
        self.conn
            .query_row(
                "SELECT id, name, url, enabled, created_at FROM mcp_servers WHERE id = ?1",
                [id],
                server_row,
            )
            .optional()
            .map_err(StoreError::from)
    }

    /// Lists MCP servers in creation order.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the rows.
    pub fn list_mcp_servers(&self) -> Result<Vec<McpServerRecord>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT id, name, url, enabled, created_at FROM mcp_servers ORDER BY created_at, id",
        )?;
        Ok(statement
            .query_map([], server_row)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Sets whether one MCP server is enabled.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot update the row.
    pub fn set_mcp_server_enabled(&self, id: &str, enabled: bool) -> Result<(), StoreError> {
        let changed = self.conn.execute(
            "UPDATE mcp_servers SET enabled = ?2 WHERE id = ?1",
            params![id, i64::from(enabled)],
        )?;
        if changed == 0 {
            return Err(StoreError::MissingMcpServer);
        }
        Ok(())
    }

    /// Deletes one MCP server and its dependent rows.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot delete the row.
    pub fn delete_mcp_server(&self, id: &str) -> Result<(), StoreError> {
        let changed = self
            .conn
            .execute("DELETE FROM mcp_servers WHERE id = ?1", [id])?;
        if changed == 0 {
            return Err(StoreError::MissingMcpServer);
        }
        Ok(())
    }

    /// Replaces all header references for one server.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot update the rows.
    pub fn replace_mcp_server_headers(
        &mut self,
        server_id: &str,
        headers: &[(String, String)],
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM mcp_server_headers WHERE server_id = ?1",
            [server_id],
        )?;
        for (name, secret_ref) in headers {
            tx.execute(
                "INSERT INTO mcp_server_headers (server_id, header_name, secret_ref) VALUES (?1, ?2, ?3)",
                params![server_id, name, secret_ref],
            )?;
        }
        Ok(tx.commit()?)
    }

    /// Lists header names and secret references for one server.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the rows.
    pub fn mcp_server_headers(
        &self,
        server_id: &str,
    ) -> Result<Vec<McpServerHeaderRecord>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT server_id, header_name, secret_ref FROM mcp_server_headers
             WHERE server_id = ?1 ORDER BY header_name",
        )?;
        Ok(statement
            .query_map([server_id], |row| {
                Ok(McpServerHeaderRecord {
                    server_id: row.get(0)?,
                    header_name: row.get(1)?,
                    secret_ref: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Replaces one tool setting.
    ///
    /// # Errors
    /// Returns an error when SQLite rejects the setting.
    pub fn set_mcp_tool_setting(
        &self,
        server_id: &str,
        tool_name: &str,
        enabled: bool,
        approval: &str,
    ) -> Result<(), StoreError> {
        if self.load_mcp_server(server_id)?.is_none() {
            return Err(StoreError::MissingMcpServer);
        }
        self.conn.execute(
            "INSERT INTO mcp_tool_settings (server_id, tool_name, enabled, approval)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(server_id, tool_name) DO UPDATE SET enabled = excluded.enabled, approval = excluded.approval",
            params![server_id, tool_name, i64::from(enabled), approval],
        )?;
        Ok(())
    }

    /// Lists saved tool settings for one server.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the rows.
    pub fn mcp_tool_settings(
        &self,
        server_id: &str,
    ) -> Result<Vec<McpToolSettingRecord>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT server_id, tool_name, enabled, approval FROM mcp_tool_settings
             WHERE server_id = ?1 ORDER BY tool_name",
        )?;
        Ok(statement
            .query_map([server_id], |row| {
                Ok(McpToolSettingRecord {
                    server_id: row.get(0)?,
                    tool_name: row.get(1)?,
                    enabled: row.get::<_, i64>(2)? != 0,
                    approval: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }
}

fn server_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpServerRecord> {
    Ok(McpServerRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        url: row.get(2)?,
        enabled: row.get::<_, i64>(3)? != 0,
        created_at: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_rows_and_dependent_settings_round_trip() {
        let mut store = Store::open_in_memory().unwrap();
        let server = store
            .create_mcp_server("docs", "https://example.test/mcp", true, 7)
            .unwrap();
        store
            .replace_mcp_server_headers(&server.id, &[("Authorization".into(), "mcp-ref-0".into())])
            .unwrap();
        store
            .set_mcp_tool_setting(&server.id, "read", false, "ask")
            .unwrap();
        assert_eq!(store.list_mcp_servers().unwrap(), vec![server.clone()]);
        assert_eq!(
            store.mcp_server_headers(&server.id).unwrap()[0].secret_ref,
            "mcp-ref-0"
        );
        assert!(!store.mcp_tool_settings(&server.id).unwrap()[0].enabled);
        store.delete_mcp_server(&server.id).unwrap();
        assert_eq!(store.mcp_server_headers(&server.id).unwrap().len(), 0);
        assert_eq!(store.mcp_tool_settings(&server.id).unwrap().len(), 0);
    }

    #[test]
    fn server_names_are_unique() {
        let store = Store::open_in_memory().unwrap();
        store
            .create_mcp_server("docs", "https://one.test", true, 1)
            .unwrap();
        assert!(
            store
                .create_mcp_server("docs", "https://two.test", true, 2)
                .is_err()
        );
    }
}
