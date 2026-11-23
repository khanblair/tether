//! MCP store and view adapters.

use std::sync::Arc;

use tether_engine::{McpApproval, McpManager, McpServerDefinition, McpToolSetting};
use tether_secrets::SecretRef;
use tether_store::{McpServerRecord, Store};

use super::{map_store_error, parse_approval, status_text};
use crate::SecretVault;
use crate::mcp_types::{McpRuntimeError, McpServerView, McpToolView};

pub(crate) fn manager_from_store(
    store: &Store,
    vault: &SecretVault,
) -> Result<Arc<McpManager>, McpRuntimeError> {
    let definitions = store
        .list_mcp_servers()
        .map_err(map_store_error)?
        .into_iter()
        .map(|server| definition_from_store(store, vault, &server))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(McpManager::new(definitions))
}

pub(super) fn definition_from_store(
    store: &Store,
    vault: &SecretVault,
    server: &McpServerRecord,
) -> Result<McpServerDefinition, McpRuntimeError> {
    let headers = store
        .mcp_server_headers(&server.id)
        .map_err(map_store_error)?
        .into_iter()
        .filter_map(|header| {
            let reference = SecretRef::new(&header.secret_ref).ok()?;
            vault
                .store
                .get(&reference)
                .ok()
                .flatten()
                .map(|secret| (header.header_name, secret))
        })
        .collect();
    let settings = store
        .mcp_tool_settings(&server.id)
        .map_err(map_store_error)?
        .into_iter()
        .filter_map(|setting| {
            Some(McpToolSetting {
                tool_name: setting.tool_name,
                enabled: setting.enabled,
                approval: parse_approval(&setting.approval).ok()?,
            })
        })
        .collect();
    Ok(McpServerDefinition {
        id: server.id.clone(),
        name: server.name.clone(),
        url: server.url.clone(),
        enabled: server.enabled,
        headers,
        tool_settings: settings,
    })
}

pub(super) fn server_view(
    store: &Store,
    manager: &McpManager,
    server: McpServerRecord,
) -> Result<McpServerView, McpRuntimeError> {
    let headers = store
        .mcp_server_headers(&server.id)
        .map_err(map_store_error)?;
    let tools = manager
        .cached_server_tools()
        .into_iter()
        .filter(|(snapshot, _)| snapshot.id == server.id)
        .map(|(snapshot, tool)| McpToolView {
            name: tool.name.clone(),
            description: tool.description.unwrap_or_default(),
            enabled: snapshot.tool_enabled(&tool.name),
            approval: match snapshot.tool_approval(&tool.name) {
                McpApproval::Ask => "ask".to_owned(),
                McpApproval::Auto => "auto".to_owned(),
            },
        })
        .collect();
    Ok(McpServerView {
        id: server.id.clone(),
        name: server.name,
        url: server.url,
        enabled: server.enabled,
        status: manager
            .status(&server.id)
            .as_ref()
            .map_or_else(|| "not_tried".to_owned(), status_text),
        header_names: headers
            .into_iter()
            .map(|header| header.header_name)
            .collect(),
        tools,
    })
}
