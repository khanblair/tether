//! UniFFI controls for MCP servers and their secret-backed headers.

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tether_engine::{McpApproval, McpManager, McpServerStatus, McpToolSetting};
use tether_secrets::{Secret, SecretRef, SecretStore};
use tether_store::{Store, StoreError};
use tokio_util::sync::CancellationToken;

use crate::SecretVault;
use crate::mcp_types::{McpHeaderInput, McpRuntimeError, McpServerView};

#[path = "mcp_support.rs"]
mod support;
pub(crate) use support::manager_from_store;
use support::{definition_from_store, server_view};

const MAX_HEADERS: usize = 8;
const MAX_HEADER_VALUE_BYTES: usize = 4 * 1024;

/// Owns saved MCP servers, their manager, and the secret store.
#[derive(uniffi::Object)]
pub struct McpRuntime {
    database_path: String,
    vault: Arc<SecretVault>,
    manager: Arc<McpManager>,
    runtime: tokio::runtime::Runtime,
}

impl Drop for McpRuntime {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            drop(self.runtime.block_on(self.manager.shutdown()));
        }
    }
}

#[uniffi::export]
impl McpRuntime {
    /// Opens MCP settings and starts its async runtime.
    ///
    /// # Errors
    /// Returns an error when the database or runtime cannot start.
    #[uniffi::constructor]
    pub fn new(database_path: &str, vault: Arc<SecretVault>) -> Result<Arc<Self>, McpRuntimeError> {
        let store =
            Store::open(Path::new(database_path)).map_err(|_| McpRuntimeError::Unavailable)?;
        let manager = manager_from_store(&store, &vault)?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|_| McpRuntimeError::Unavailable)?;
        Ok(Arc::new(Self {
            database_path: database_path.to_owned(),
            vault,
            manager,
            runtime,
        }))
    }

    /// Adds an MCP server and stores header values in the secret store.
    ///
    /// # Errors
    /// Returns a typed validation or storage error.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "UniFFI passes owned values for record lists"
    )]
    pub fn add_mcp_server(
        &self,
        name: &str,
        url: &str,
        headers: Vec<McpHeaderInput>,
    ) -> Result<String, McpRuntimeError> {
        validate_name(name)?;
        validate_url(url)?;
        validate_headers(&headers)?;
        let mut store = self.store()?;
        let server = store
            .create_mcp_server(name, url, true, now_ms())
            .map_err(map_store_error)?;
        let references = headers
            .iter()
            .enumerate()
            .map(|(index, header)| (header.name.clone(), format!("mcp-{}-{index}", server.id)))
            .collect::<Vec<_>>();
        if let Err(error) = put_headers(&self.vault.store, &references, &headers) {
            drop(store.delete_mcp_server(&server.id));
            return Err(error);
        }
        store
            .replace_mcp_server_headers(&server.id, &references)
            .map_err(map_store_error)?;
        let definition = definition_from_store(&store, &self.vault, &server)?;
        self.runtime
            .block_on(self.manager.upsert(definition))
            .map_err(|_| McpRuntimeError::Unavailable)?;
        Ok(server.id)
    }

    /// Removes an MCP server and its secret-backed headers.
    ///
    /// # Errors
    /// Returns `NotFound` or a storage error.
    pub fn remove_mcp_server(&self, id: &str) -> Result<(), McpRuntimeError> {
        let store = self.store()?;
        let headers = store.mcp_server_headers(id).map_err(map_store_error)?;
        if store
            .load_mcp_server(id)
            .map_err(map_store_error)?
            .is_none()
        {
            return Err(McpRuntimeError::NotFound);
        }
        self.runtime
            .block_on(self.manager.remove(id))
            .map_err(|_| McpRuntimeError::Unavailable)?;
        for header in headers {
            self.vault
                .store
                .delete(
                    &SecretRef::new(&header.secret_ref)
                        .map_err(|_| McpRuntimeError::Unavailable)?,
                )
                .map_err(|_| McpRuntimeError::Unavailable)?;
        }
        store.delete_mcp_server(id).map_err(map_store_error)
    }

    /// Lists MCP servers without returning header values.
    ///
    /// # Errors
    /// Returns an error when storage cannot be read.
    pub fn list_mcp_servers(&self) -> Result<Vec<McpServerView>, McpRuntimeError> {
        let store = self.store()?;
        store
            .list_mcp_servers()
            .map_err(map_store_error)?
            .into_iter()
            .map(|server| server_view(&store, &self.manager, server))
            .collect()
    }

    /// Connects to a server and reports its advertised tool count.
    ///
    /// # Errors
    /// Returns the safe connection failure when the test cannot complete.
    pub fn test_mcp_server(&self, id: &str) -> Result<u32, McpRuntimeError> {
        if self
            .store()?
            .load_mcp_server(id)
            .map_err(map_store_error)?
            .is_none()
        {
            return Err(McpRuntimeError::NotFound);
        }
        let listing = self
            .runtime
            .block_on(self.manager.refresh(id, &CancellationToken::new()))
            .map_err(|reason| McpRuntimeError::TestFailed { reason })?;
        u32::try_from(listing.tools.len()).map_err(|_| McpRuntimeError::Unavailable)
    }

    /// Enables or disables one MCP server.
    ///
    /// # Errors
    /// Returns `NotFound` or an update error.
    pub fn set_mcp_server_enabled(&self, id: &str, enabled: bool) -> Result<(), McpRuntimeError> {
        self.store()?
            .set_mcp_server_enabled(id, enabled)
            .map_err(map_store_error)?;
        self.runtime
            .block_on(self.manager.set_enabled(id, enabled))
            .map_err(|_| McpRuntimeError::Unavailable)
    }

    /// Saves one MCP tool's enabled and approval settings.
    ///
    /// # Errors
    /// Returns `NotFound`, invalid approval, or a storage error.
    pub fn set_mcp_tool(
        &self,
        server_id: &str,
        tool: String,
        enabled: bool,
        approval: &str,
    ) -> Result<(), McpRuntimeError> {
        let parsed = parse_approval(approval)?;
        self.store()?
            .set_mcp_tool_setting(server_id, &tool, enabled, approval)
            .map_err(map_store_error)?;
        self.runtime
            .block_on(self.manager.set_tool_setting(
                server_id,
                McpToolSetting {
                    tool_name: tool,
                    enabled,
                    approval: parsed,
                },
            ))
            .map_err(|_| McpRuntimeError::Unavailable)
    }

    /// Refreshes one server's cached tools.
    ///
    /// # Errors
    /// Returns `NotFound`, connection, or protocol failure.
    pub fn refresh_mcp_tools(&self, id: &str) -> Result<(), McpRuntimeError> {
        if self
            .store()?
            .load_mcp_server(id)
            .map_err(map_store_error)?
            .is_none()
        {
            return Err(McpRuntimeError::NotFound);
        }
        self.runtime
            .block_on(self.manager.refresh(id, &CancellationToken::new()))
            .map(|_| ())
            .map_err(|reason| McpRuntimeError::TestFailed { reason })
    }
}

impl McpRuntime {
    fn store(&self) -> Result<Store, McpRuntimeError> {
        Store::open(Path::new(&self.database_path)).map_err(|_| McpRuntimeError::Unavailable)
    }
}

fn put_headers(
    store: &SecretStore,
    references: &[(String, String)],
    headers: &[McpHeaderInput],
) -> Result<(), McpRuntimeError> {
    for ((_, reference), header) in references.iter().zip(headers) {
        let secret_ref = SecretRef::new(reference).map_err(|_| McpRuntimeError::Unavailable)?;
        store
            .put(&secret_ref, &Secret::from_text(&header.value))
            .map_err(|_| McpRuntimeError::Unavailable)?;
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), McpRuntimeError> {
    let valid = (1..=40).contains(&name.len())
        && name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    valid.then_some(()).ok_or(McpRuntimeError::InvalidName)
}

fn validate_url(url: &str) -> Result<(), McpRuntimeError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| McpRuntimeError::InvalidUrl)?;
    (parsed.scheme() == "https"
        && parsed.host_str().is_some()
        && parsed.username().is_empty()
        && parsed.password().is_none())
    .then_some(())
    .ok_or(McpRuntimeError::InvalidUrl)
}

fn validate_headers(headers: &[McpHeaderInput]) -> Result<(), McpRuntimeError> {
    if headers.len() > MAX_HEADERS {
        return Err(McpRuntimeError::TooManyHeaders);
    }
    for header in headers {
        if !(1..=64).contains(&header.name.len())
            || !header
                .name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(McpRuntimeError::InvalidHeaderName);
        }
        if header.value.len() > MAX_HEADER_VALUE_BYTES {
            return Err(McpRuntimeError::HeaderTooLong);
        }
    }
    Ok(())
}

fn parse_approval(value: &str) -> Result<McpApproval, McpRuntimeError> {
    match value {
        "ask" => Ok(McpApproval::Ask),
        "auto" => Ok(McpApproval::Auto),
        _ => Err(McpRuntimeError::InvalidApproval),
    }
}

fn status_text(status: &McpServerStatus) -> String {
    match status {
        McpServerStatus::Connected => "connected".to_owned(),
        McpServerStatus::Failed(_) => "failed".to_owned(),
        McpServerStatus::Disabled => "disabled".to_owned(),
        McpServerStatus::NotTried => "not_tried".to_owned(),
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "used as a map_err callback, which passes the error by value"
)]
fn map_store_error(error: StoreError) -> McpRuntimeError {
    if matches!(error, StoreError::MissingMcpServer) {
        McpRuntimeError::NotFound
    } else {
        McpRuntimeError::Unavailable
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_validation_accepts_the_documented_boundary_values() {
        assert!(validate_name("a").is_ok());
        assert!(validate_name(&"a".repeat(40)).is_ok());
        assert!(
            validate_headers(&[McpHeaderInput {
                name: "X-Test-1".into(),
                value: "x".repeat(MAX_HEADER_VALUE_BYTES),
            }])
            .is_ok()
        );
        assert!(validate_url("https://example.test/mcp").is_ok());
    }

    #[test]
    fn add_validation_rejects_each_invalid_rule() {
        assert!(validate_name("A").is_err());
        assert!(validate_name("-server").is_err());
        assert!(validate_name(&"a".repeat(41)).is_err());
        assert!(validate_url("http://example.test/mcp").is_err());
        assert!(validate_url("https://user@example.test/mcp").is_err());
        assert!(
            validate_headers(&[McpHeaderInput {
                name: "bad_name".into(),
                value: "x".into(),
            }])
            .is_err()
        );
        assert!(
            validate_headers(&[McpHeaderInput {
                name: "X-Test".into(),
                value: "x".repeat(MAX_HEADER_VALUE_BYTES + 1),
            }])
            .is_err()
        );
        assert!(
            validate_headers(
                &(0..=MAX_HEADERS)
                    .map(|index| McpHeaderInput {
                        name: format!("X-{index}"),
                        value: "v".into(),
                    })
                    .collect::<Vec<_>>()
            )
            .is_err()
        );
    }

    #[test]
    fn status_text_does_not_include_failure_details() {
        assert_eq!(
            status_text(&McpServerStatus::Failed("header-secret".into())),
            "failed"
        );
    }
}
