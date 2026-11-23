//! UniFFI MCP records and errors.

/// A header supplied while adding an MCP server.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct McpHeaderInput {
    /// Header name.
    pub name: String,
    /// Header value. It is written only to the secret store.
    pub value: String,
}

/// One MCP tool shown in server settings.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct McpToolView {
    /// Server-advertised tool name.
    pub name: String,
    /// Server-provided description.
    pub description: String,
    /// Whether the tool is offered to models.
    pub enabled: bool,
    /// `ask` or `auto`.
    pub approval: String,
}

/// One MCP server shown in settings. Header values are never included.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct McpServerView {
    /// Stable server id.
    pub id: String,
    /// User-facing server name.
    pub name: String,
    /// Server endpoint.
    pub url: String,
    /// Whether the server is enabled.
    pub enabled: bool,
    /// `connected`, `failed`, `disabled`, or `not_tried`.
    pub status: String,
    /// Saved header names only.
    pub header_names: Vec<String>,
    /// Cached server tools and their local settings.
    pub tools: Vec<McpToolView>,
}

/// An MCP settings operation failed.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum McpRuntimeError {
    /// The server name is outside the supported form.
    #[error("invalid MCP server name")]
    InvalidName,
    /// The URL is not an HTTPS URL without userinfo.
    #[error("invalid MCP server URL")]
    InvalidUrl,
    /// A header name is outside the supported form.
    #[error("invalid MCP header name")]
    InvalidHeaderName,
    /// More than eight headers were supplied.
    #[error("MCP servers support at most 8 headers")]
    TooManyHeaders,
    /// A header value is too large.
    #[error("MCP header values are limited to 4 KiB")]
    HeaderTooLong,
    /// The requested server does not exist.
    #[error("MCP server not found")]
    NotFound,
    /// The approval value is not supported.
    #[error("MCP approval must be ask or auto")]
    InvalidApproval,
    /// The database or secret store is unavailable.
    #[error("MCP server storage is unavailable")]
    Unavailable,
    /// A server test or refresh failed.
    #[error("MCP server test failed: {reason}")]
    TestFailed {
        /// Safe failure detail.
        reason: String,
    },
}
