//! MCP manager operations.
use super::{
    CALL_TIMEOUT, CONNECT_TIMEOUT, McpApproval, McpManager, McpServerDefinition, McpServerSnapshot,
    McpServerStatus, McpTool, McpToolSetting, McpTransportFactory, ServerState, cache_is_fresh,
    close_session, default_factory, now_ms, server_config, snapshot,
};
use std::sync::{Arc, RwLock};
use tether_mcp::{McpSession, McpToolList, McpToolResult};
use tokio_util::sync::CancellationToken;
#[expect(
    clippy::excessive_nesting,
    reason = "MCP manager state transitions stay together for lock safety"
)]
impl McpManager {
    /// Creates a manager using Tether's HTTPS MCP transport.
    #[must_use]
    pub fn new(servers: Vec<McpServerDefinition>) -> Arc<Self> {
        Self::with_transport_factory(servers, default_factory())
    }

    /// Creates a manager with an injected transport factory.
    #[must_use]
    pub fn with_transport_factory(
        servers: Vec<McpServerDefinition>,
        factory: McpTransportFactory,
    ) -> Arc<Self> {
        Self::with_factory_and_clock(servers, factory, Arc::new(now_ms))
    }

    /// Creates a manager with injected transport and clock seams.
    #[must_use]
    pub fn with_factory_and_clock(
        servers: Vec<McpServerDefinition>,
        factory: McpTransportFactory,
        clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    ) -> Arc<Self> {
        let states = servers
            .into_iter()
            .map(|definition| {
                let status = if definition.enabled {
                    McpServerStatus::NotTried
                } else {
                    McpServerStatus::Disabled
                };
                let id = definition.id.clone();
                (
                    id,
                    Arc::new(tokio::sync::Mutex::new(ServerState {
                        definition,
                        status,
                        session: None,
                        tools: Vec::new(),
                        cached_at: None,
                    })),
                )
            })
            .collect();
        Arc::new(Self {
            servers: RwLock::new(states),
            factory,
            clock,
        })
    }

    /// Returns a server's current status without connecting it.
    #[must_use]
    pub fn status(&self, id: &str) -> Option<McpServerStatus> {
        let slot = self.servers.read().ok()?.get(id)?.clone();
        Some(slot.try_lock().ok()?.status.clone())
    }

    /// Returns the last cached tool list without connecting a server.
    #[must_use]
    pub fn cached_tools(&self, id: &str) -> Vec<McpTool> {
        self.servers
            .read()
            .ok()
            .and_then(|servers| servers.get(id).cloned())
            .and_then(|slot| slot.try_lock().ok().map(|state| state.tools.clone()))
            .unwrap_or_default()
    }

    /// Returns cached tools paired with non-sensitive server metadata.
    #[must_use]
    pub fn cached_server_tools(&self) -> Vec<(McpServerSnapshot, McpTool)> {
        self.servers
            .read()
            .ok()
            .into_iter()
            .flat_map(|servers| servers.values().cloned().collect::<Vec<_>>())
            .filter_map(|slot| {
                let state = slot.try_lock().ok()?;
                if !state.definition.enabled {
                    return None;
                }
                let server = snapshot(&state.definition);
                Some(
                    state
                        .tools
                        .clone()
                        .into_iter()
                        .map(move |tool| (server.clone(), tool))
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect()
    }

    /// Resolves a cached namespaced tool for preparation.
    ///
    /// # Errors
    /// Returns an error when the cached tool or server is unavailable.
    pub fn lookup_tool(&self, name: &str) -> Result<(String, String, String, McpApproval), String> {
        self.cached_server_tools()
            .into_iter()
            .find_map(|(server, tool)| {
                (server.tool_enabled(&tool.name)
                    && tether_mcp::namespaced_tool_name(&server.name, &tool.name) == name)
                    .then(|| {
                        (
                            server.id.clone(),
                            server.name.clone(),
                            tool.name.clone(),
                            server.tool_approval(&tool.name),
                        )
                    })
            })
            .ok_or_else(|| "MCP tool is not available".to_owned())
    }

    /// Returns a non-sensitive server name.
    ///
    /// # Errors
    /// Returns an error when the server id is unknown.
    pub fn server_name(&self, id: &str) -> Result<String, String> {
        let slot = self.slot(id)?;
        slot.try_lock()
            .map(|state| state.definition.name.clone())
            .map_err(|_| "MCP server is busy".to_owned())
    }

    /// Refreshes a server's tools, connecting it lazily when needed.
    ///
    /// # Errors
    /// Returns a safe connection or MCP protocol error. The failure is also saved in status.
    pub async fn refresh(
        &self,
        id: &str,
        cancel: &CancellationToken,
    ) -> Result<McpToolList, String> {
        let slot = self.slot(id)?;
        let mut state = slot.lock().await;
        self.refresh_locked(&mut state, cancel).await
    }

    /// Returns cached tools or refreshes an expired list.
    ///
    /// # Errors
    /// Returns a safe connection or MCP protocol error.
    pub async fn tools(&self, id: &str, cancel: &CancellationToken) -> Result<McpToolList, String> {
        let slot = self.slot(id)?;
        let mut state = slot.lock().await;
        if cache_is_fresh(state.cached_at, (self.clock)()) {
            return Ok(McpToolList {
                tools: state.tools.clone(),
                warnings: Vec::new(),
            });
        }
        self.refresh_locked(&mut state, cancel).await
    }

    /// Calls one currently advertised tool. A stale list is refreshed before failure.
    ///
    /// # Errors
    /// Returns a validation, connection, timeout, cancellation, or MCP tool error.
    pub async fn call(
        &self,
        id: &str,
        tool_name: &str,
        args_json: &str,
        cancel: &CancellationToken,
    ) -> Result<McpToolResult, String> {
        let slot = self.slot(id)?;
        let mut state = slot.lock().await;
        let listed = if cache_is_fresh(state.cached_at, (self.clock)()) {
            state.tools.clone()
        } else {
            self.refresh_locked(&mut state, cancel).await?.tools
        };
        if !listed.iter().any(|tool| tool.name == tool_name) {
            let refreshed = self.refresh_locked(&mut state, cancel).await?;
            if !refreshed.tools.iter().any(|tool| tool.name == tool_name) {
                return Err(format!("MCP tool no longer exists: {tool_name}"));
            }
        }
        let session = state.session.as_mut().ok_or("MCP session is unavailable")?;
        session.set_timeout(CALL_TIMEOUT);
        session
            .call_tool(tool_name, args_json, cancel)
            .await
            .map_err(|error| error.to_string())
    }

    /// Enables or disables a server and closes a session when disabling it.
    ///
    /// # Errors
    /// Returns an error when the server id is unknown or closing fails.
    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        let slot = self.slot(id)?;
        let mut state = slot.lock().await;
        state.definition.enabled = enabled;
        if !enabled {
            close_session(&mut state).await?;
            state.status = McpServerStatus::Disabled;
            state.tools.clear();
            state.cached_at = None;
            return Ok(());
        }
        state.status = McpServerStatus::NotTried;
        state.cached_at = None;
        Ok(())
    }

    /// Replaces one server definition and closes any old session.
    ///
    /// # Errors
    /// Returns an error when the manager or an old session cannot be updated.
    pub async fn upsert(&self, definition: McpServerDefinition) -> Result<(), String> {
        let id = definition.id.clone();
        let slot = {
            let mut servers = self
                .servers
                .write()
                .map_err(|_| "MCP manager is unavailable")?;
            if let Some(slot) = servers.get(&id) {
                slot.clone()
            } else {
                let status = if definition.enabled {
                    McpServerStatus::NotTried
                } else {
                    McpServerStatus::Disabled
                };
                let slot = Arc::new(tokio::sync::Mutex::new(ServerState {
                    status,
                    definition,
                    session: None,
                    tools: Vec::new(),
                    cached_at: None,
                }));
                servers.insert(id, slot.clone());
                return Ok(());
            }
        };
        let mut state = slot.lock().await;
        close_session(&mut state).await?;
        state.definition = definition;
        state.status = if state.definition.enabled {
            McpServerStatus::NotTried
        } else {
            McpServerStatus::Disabled
        };
        state.tools.clear();
        state.cached_at = None;
        Ok(())
    }

    /// Saves one per-tool setting in the in-memory definition.
    ///
    /// # Errors
    /// Returns an error when the server id is unknown or the manager is busy.
    pub async fn set_tool_setting(&self, id: &str, setting: McpToolSetting) -> Result<(), String> {
        let slot = self.slot(id)?;
        let mut state = slot.lock().await;
        if let Some(existing) = state
            .definition
            .tool_settings
            .iter_mut()
            .find(|existing| existing.tool_name == setting.tool_name)
        {
            *existing = setting;
        } else {
            state.definition.tool_settings.push(setting);
        }
        Ok(())
    }

    /// Closes and removes a server.
    ///
    /// # Errors
    /// Returns an error when the server id is unknown or closing fails.
    pub async fn remove(&self, id: &str) -> Result<(), String> {
        let slot = self
            .servers
            .write()
            .map_err(|_| "MCP manager is unavailable")?
            .remove(id)
            .ok_or("MCP server not found")?;
        let mut state = slot.lock().await;
        close_session(&mut state).await
    }

    /// Closes every active server session.
    ///
    /// # Errors
    /// Returns the first close failure, if any.
    pub async fn shutdown(&self) -> Result<(), String> {
        let slots: Vec<_> = self
            .servers
            .read()
            .map_err(|_| "MCP manager is unavailable")?
            .values()
            .cloned()
            .collect();
        for slot in slots {
            let mut state = slot.lock().await;
            close_session(&mut state).await?;
        }
        Ok(())
    }

    fn slot(&self, id: &str) -> Result<Arc<tokio::sync::Mutex<ServerState>>, String> {
        self.servers
            .read()
            .map_err(|_| "MCP manager is unavailable")?
            .get(id)
            .cloned()
            .ok_or_else(|| "MCP server not found".to_owned())
    }

    async fn refresh_locked(
        &self,
        state: &mut ServerState,
        cancel: &CancellationToken,
    ) -> Result<McpToolList, String> {
        state.cached_at = None;
        let tools = self.ensure_session_and_tools(state, cancel).await?;
        let listing = McpToolList {
            tools,
            warnings: Vec::new(),
        };
        state.cached_at = Some((self.clock)());
        Ok(listing)
    }

    async fn ensure_session_and_tools(
        &self,
        state: &mut ServerState,
        cancel: &CancellationToken,
    ) -> Result<Vec<McpTool>, String> {
        if !state.definition.enabled {
            return Err("MCP server is disabled".to_owned());
        }
        if state.session.is_none() {
            let config = server_config(&state.definition);
            let transport = match (self.factory)(&config) {
                Ok(transport) => transport,
                Err(error) => {
                    state.status = McpServerStatus::Failed(error.clone());
                    return Err(error);
                }
            };
            let connected = tokio::time::timeout(
                CONNECT_TIMEOUT,
                McpSession::connect(config, transport, cancel.clone()),
            )
            .await
            .map_or_else(
                |_| Err("MCP server connection timed out".to_owned()),
                |result| result.map_err(|error| error.to_string()),
            );
            let mut session = match connected {
                Ok(session) => session,
                Err(error) => {
                    state.status = McpServerStatus::Failed(error.clone());
                    return Err(error);
                }
            };
            session.set_timeout(CALL_TIMEOUT);
            state.session = Some(session);
            state.status = McpServerStatus::Connected;
        }
        if state.cached_at.is_some() && !state.tools.is_empty() {
            return Ok(state.tools.clone());
        }
        let session = state.session.as_mut().ok_or("MCP session is unavailable")?;
        match session.list_tools(cancel).await {
            Ok(listing) => {
                state.tools.clone_from(&listing.tools);
                Ok(listing.tools)
            }
            Err(error) => {
                let text = error.to_string();
                state.status = McpServerStatus::Failed(text.clone());
                Err(text)
            }
        }
    }
}
