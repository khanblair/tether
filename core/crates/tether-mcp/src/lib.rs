//! MCP client for Streamable HTTP servers.

mod names;
mod protocol;
mod render;
mod session;
mod transport;

pub use names::{namespaced_tool_name, parse_namespaced_tool_name};
pub use session::{McpSession, McpTool, McpToolList, McpToolResult};
pub use tether_secrets::Secret;
pub use transport::{
    BodyStream, HttpDelete, HttpPost, HttpReply, McpError, McpServerConfig, McpTransport,
    ReqwestMcpTransport, TransportFuture,
};

#[cfg(test)]
mod tests;
