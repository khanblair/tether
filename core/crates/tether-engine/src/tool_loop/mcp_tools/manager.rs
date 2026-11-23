//! Lazy MCP server sessions and bounded tool-list caching.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tether_mcp::{McpServerConfig, McpSession, McpTool, McpTransport, ReqwestMcpTransport, Secret};

#[path = "manager_impl.rs"]
mod manager_impl;

pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub(super) const CALL_TIMEOUT: Duration = Duration::from_secs(60);
pub(super) const TOOL_CACHE_MS: u64 = 5 * 60 * 1_000;

/// Whether an MCP tool needs an approval prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpApproval {
    /// Ask the user before calling the tool.
    Ask,
    /// Use the in-memory network rule for supported modes.
    Auto,
}

/// A saved setting for one MCP tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpToolSetting {
    /// Server-advertised tool name.
    pub tool_name: String,
    /// Whether this tool is offered to models.
    pub enabled: bool,
    /// Approval behavior.
    pub approval: McpApproval,
}

/// Server data used by the manager. Header values remain redacted by `Secret`.
pub struct McpServerDefinition {
    /// Stable local id.
    pub id: String,
    /// User-facing name.
    pub name: String,
    /// HTTPS endpoint.
    pub url: String,
    /// Whether the server is available.
    pub enabled: bool,
    /// Sensitive request headers.
    pub headers: Vec<(String, Secret)>,
    /// Per-tool settings.
    pub tool_settings: Vec<McpToolSetting>,
}

/// The connection state of one MCP server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpServerStatus {
    /// A session is connected.
    Connected,
    /// The last connection or request failed.
    Failed(String),
    /// The server is disabled.
    Disabled,
    /// No connection has been attempted.
    NotTried,
}

/// Non-sensitive server metadata used by tool specifications.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpServerSnapshot {
    /// Stable local id.
    pub id: String,
    /// User-facing name.
    pub name: String,
    /// Saved tool settings.
    pub tool_settings: Vec<McpToolSetting>,
}

impl McpServerSnapshot {
    /// Returns whether a tool is offered to models.
    #[must_use]
    pub fn tool_enabled(&self, name: &str) -> bool {
        self.tool_settings
            .iter()
            .find(|setting| setting.tool_name == name)
            .is_none_or(|setting| setting.enabled)
    }

    /// Returns the tool's approval policy, defaulting to Ask.
    #[must_use]
    pub fn tool_approval(&self, name: &str) -> McpApproval {
        self.tool_settings
            .iter()
            .find(|setting| setting.tool_name == name)
            .map_or(McpApproval::Ask, |setting| setting.approval)
    }
}

/// Factory used to inject scripted transports in tests.
pub type McpTransportFactory =
    Arc<dyn Fn(&McpServerConfig) -> Result<Arc<dyn McpTransport>, String> + Send + Sync>;

pub(super) struct ServerState {
    pub(super) definition: McpServerDefinition,
    pub(super) status: McpServerStatus,
    pub(super) session: Option<McpSession>,
    pub(super) tools: Vec<McpTool>,
    pub(super) cached_at: Option<u64>,
}

/// Owns independent, lazily connected MCP sessions for one runtime.
pub struct McpManager {
    pub(super) servers: RwLock<HashMap<String, Arc<tokio::sync::Mutex<ServerState>>>>,
    pub(super) factory: McpTransportFactory,
    pub(super) clock: Arc<dyn Fn() -> u64 + Send + Sync>,
}

pub(super) fn server_config(definition: &McpServerDefinition) -> McpServerConfig {
    McpServerConfig {
        name: definition.name.clone(),
        url: definition.url.clone(),
        headers: definition
            .headers
            .iter()
            .map(|(name, secret)| (name.clone(), Secret::new(secret.expose().to_vec())))
            .collect(),
    }
}

pub(super) fn snapshot(definition: &McpServerDefinition) -> McpServerSnapshot {
    McpServerSnapshot {
        id: definition.id.clone(),
        name: definition.name.clone(),
        tool_settings: definition.tool_settings.clone(),
    }
}

pub(super) async fn close_session(state: &mut ServerState) -> Result<(), String> {
    let Some(session) = state.session.as_mut() else {
        return Ok(());
    };
    session.close().await.map_err(|error| error.to_string())?;
    state.session = None;
    Ok(())
}

pub(super) fn default_factory() -> McpTransportFactory {
    Arc::new(|config| {
        ReqwestMcpTransport::new(&config.name)
            .map(|transport| Arc::new(transport) as Arc<dyn McpTransport>)
            .map_err(|error| error.to_string())
    })
}

pub(super) fn cache_is_fresh(cached_at: Option<u64>, now: u64) -> bool {
    cached_at.is_some_and(|at| now.saturating_sub(at) < TOOL_CACHE_MS)
}

pub(super) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().try_into().unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
