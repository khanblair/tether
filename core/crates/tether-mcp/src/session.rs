//! MCP initialization and tool operations.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::protocol::{protocol, read_response};
use crate::render::render_result;
use crate::transport::{HttpDelete, HttpPost, HttpReply, McpError, McpServerConfig, McpTransport};

const CURRENT_VERSION: &str = "2025-06-18";
const FALLBACK_VERSION: &str = "2025-03-26";
const MAX_TOOLS: usize = 500;
const MAX_PAGES: usize = 100;

/// A tool advertised by an MCP server.
#[derive(Clone, Debug, PartialEq)]
pub struct McpTool {
    /// Tool name as advertised by the server.
    pub name: String,
    /// Optional display title.
    pub title: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Compact JSON input schema.
    pub input_schema_json: String,
    /// MCP read-only hint.
    pub read_only_hint: Option<bool>,
    /// MCP destructive hint.
    pub destructive_hint: Option<bool>,
}

/// Tools and non-fatal warnings from a list operation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct McpToolList {
    /// Valid tools.
    pub tools: Vec<McpTool>,
    /// Warnings for skipped malformed tools.
    pub warnings: Vec<String>,
}

/// Rendered result of an MCP tool call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpToolResult {
    /// Text returned by the tool.
    pub text: String,
    /// Whether the server reported an error.
    pub is_error: bool,
    /// Whether the result was bounded.
    pub truncated: bool,
}

/// A stateful client session with one MCP server.
pub struct McpSession {
    pub(crate) config: McpServerConfig,
    pub(crate) transport: Arc<dyn McpTransport>,
    pub(crate) session_id: Option<String>,
    pub(crate) protocol_version: String,
    next_id: u64,
    timeout: Duration,
}

impl McpSession {
    /// Performs the MCP initialize handshake.
    ///
    /// # Errors
    /// Returns a configuration, transport, protocol, timeout, or cancellation error.
    pub async fn connect(
        config: McpServerConfig,
        transport: Arc<dyn McpTransport>,
        cancel: CancellationToken,
    ) -> Result<Self, McpError> {
        config.validate()?;
        let mut session = Self {
            config,
            transport,
            session_id: None,
            protocol_version: CURRENT_VERSION.to_owned(),
            next_id: 1,
            timeout: Duration::from_secs(60),
        };
        session.initialize(&cancel).await?;
        Ok(session)
    }

    /// Changes the timeout applied to each MCP operation.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Lists tools, following bounded cursor pages.
    ///
    /// # Errors
    /// Returns a transport or malformed MCP response error.
    #[expect(
        clippy::excessive_nesting,
        reason = "Cursor paging has a bounded page loop"
    )]
    pub async fn list_tools(
        &mut self,
        cancel: &CancellationToken,
    ) -> Result<McpToolList, McpError> {
        let mut page_count = 0;
        let mut cursor = None;
        let mut listing = McpToolList::default();
        while page_count < MAX_PAGES && listing.tools.len() < MAX_TOOLS {
            page_count += 1;
            let result = self
                .request("tools/list", list_params(cursor.as_ref()), cancel)
                .await?;
            let page = result
                .get("tools")
                .and_then(Value::as_array)
                .ok_or_else(|| protocol(&self.config.name, "tools/list result"))?;
            add_tools(&mut listing, page);
            let Some(next) = result.get("nextCursor").and_then(Value::as_str) else {
                break;
            };
            cursor = Some(next.to_owned());
        }
        Ok(listing)
    }

    /// Calls a tool and renders its untrusted result as text.
    ///
    /// # Errors
    /// Returns a transport, timeout, protocol, or cancellation error.
    pub async fn call_tool(
        &mut self,
        name: &str,
        args_json: &str,
        cancel: &CancellationToken,
    ) -> Result<McpToolResult, McpError> {
        let arguments: Value = serde_json::from_str(args_json)
            .map_err(|_| protocol(&self.config.name, "tool arguments are not JSON"))?;
        let result = self
            .request(
                "tools/call",
                json!({"name": name, "arguments": arguments}),
                cancel,
            )
            .await?;
        let is_error = result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let (text, truncated) = render_result(&result);
        Ok(McpToolResult {
            text,
            is_error,
            truncated,
        })
    }

    /// Closes the session when the server supports DELETE.
    ///
    /// # Errors
    /// Returns a transport or HTTP error other than 405.
    pub async fn close(&mut self) -> Result<(), McpError> {
        if self.session_id.is_none() {
            return Ok(());
        }
        let headers = self.request_headers(true)?;
        let reply = self
            .transport
            .delete(HttpDelete {
                url: self.config.url.clone(),
                headers,
            })
            .await
            .map_err(|error| with_server(error, &self.config.name))?;
        if reply.status != 405 && !(200..300).contains(&reply.status) {
            return Err(McpError::Http {
                server: self.config.name.clone(),
                status: reply.status,
            });
        }
        self.session_id = None;
        Ok(())
    }

    async fn initialize(&mut self, cancel: &CancellationToken) -> Result<(), McpError> {
        let id = self.next_id();
        let body = json!({"jsonrpc":"2.0","id":id,"method":"initialize","params":{"protocolVersion":CURRENT_VERSION,"capabilities":{},"clientInfo":{"name":"tether","version":env!("CARGO_PKG_VERSION")}}});
        let reply = self.post(body, false, cancel).await?;
        let (response, session_id) = read_response(reply, id, self, cancel).await?;
        let version = response
            .get("protocolVersion")
            .and_then(Value::as_str)
            .ok_or_else(|| protocol(&self.config.name, "missing protocol version"))?;
        if version != CURRENT_VERSION && version != FALLBACK_VERSION {
            return Err(protocol(&self.config.name, "unsupported protocol version"));
        }
        version.clone_into(&mut self.protocol_version);
        self.session_id = session_id;
        let reply = self
            .post(
                json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
                true,
                cancel,
            )
            .await?;
        if !(200..300).contains(&reply.status) {
            return Err(McpError::Http {
                server: self.config.name.clone(),
                status: reply.status,
            });
        }
        Ok(())
    }

    async fn request(
        &mut self,
        method: &str,
        params: Value,
        cancel: &CancellationToken,
    ) -> Result<Value, McpError> {
        let id = self.next_id();
        let body = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let first =
            tokio::time::timeout(self.timeout, self.request_once(body.clone(), id, cancel)).await;
        let Ok(first) = first else {
            return Err(McpError::Timeout {
                server: self.config.name.clone(),
            });
        };
        if matches!(first, Err(McpError::Http { status: 404, .. })) && self.session_id.is_some() {
            self.session_id = None;
            self.initialize(cancel).await?;
            let retry =
                tokio::time::timeout(self.timeout, self.request_once(body, id, cancel)).await;
            return match retry {
                Ok(result) => result,
                Err(_) => Err(McpError::Timeout {
                    server: self.config.name.clone(),
                }),
            };
        }
        first
    }

    async fn request_once(
        &self,
        body: Value,
        id: u64,
        cancel: &CancellationToken,
    ) -> Result<Value, McpError> {
        if cancel.is_cancelled() {
            return Err(self.cancelled(id).await);
        }
        let reply = tokio::select! { biased; () = cancel.cancelled() => return Err(self.cancelled(id).await), result = self.post(body, true, cancel) => result? };
        if reply.status == 404 {
            return Err(McpError::Http {
                server: self.config.name.clone(),
                status: 404,
            });
        }
        if !(200..300).contains(&reply.status) {
            return Err(McpError::Http {
                server: self.config.name.clone(),
                status: reply.status,
            });
        }
        Ok(read_response(reply, id, self, cancel).await?.0)
    }

    pub(crate) async fn post(
        &self,
        body: Value,
        session: bool,
        _cancel: &CancellationToken,
    ) -> Result<HttpReply, McpError> {
        let headers = self.request_headers(session)?;
        let body = serde_json::to_vec(&body)
            .map_err(|_| protocol(&self.config.name, "cannot encode request"))?;
        self.transport
            .post(HttpPost {
                url: self.config.url.clone(),
                headers,
                body,
            })
            .await
            .map_err(|error| with_server(error, &self.config.name))
    }

    fn request_headers(&self, session: bool) -> Result<Vec<(String, String)>, McpError> {
        let mut headers = vec![
            (
                "Accept".to_owned(),
                "application/json, text/event-stream".to_owned(),
            ),
            ("Content-Type".to_owned(), "application/json".to_owned()),
            (
                "MCP-Protocol-Version".to_owned(),
                self.protocol_version.clone(),
            ),
        ];
        if session && let Some(id) = &self.session_id {
            headers.push(("Mcp-Session-Id".to_owned(), id.clone()));
        }
        for (name, secret) in &self.config.headers {
            headers.push((
                name.clone(),
                secret
                    .expose_text()
                    .map_err(|_| McpError::InvalidConfig {
                        server: self.config.name.clone(),
                        message: "invalid header".to_owned(),
                    })?
                    .to_owned(),
            ));
        }
        Ok(headers)
    }

    pub(crate) async fn cancelled(&self, id: u64) -> McpError {
        let token = CancellationToken::new();
        drop(self.post(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":id,"reason":"cancelled"}}), true, &token).await);
        McpError::Cancelled {
            server: self.config.name.clone(),
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

fn list_params(cursor: Option<&String>) -> Value {
    cursor.map_or_else(|| json!({}), |value| json!({"cursor": value}))
}

fn add_tools(listing: &mut McpToolList, page: &[Value]) {
    for raw in page {
        if listing.tools.len() == MAX_TOOLS {
            return;
        }
        match parse_tool(raw) {
            Some(tool) => listing.tools.push(tool),
            None => listing
                .warnings
                .push("skipped malformed MCP tool".to_owned()),
        }
    }
}

fn parse_tool(raw: &Value) -> Option<McpTool> {
    let object = raw.as_object()?;
    let name = object.get("name")?.as_str()?.to_owned();
    let schema = object.get("inputSchema")?.as_object()?;
    if name.is_empty() {
        return None;
    }
    let annotations = object.get("annotations").and_then(Value::as_object);
    Some(McpTool {
        name,
        title: object
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_owned),
        description: object
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_owned),
        input_schema_json: serde_json::to_string(schema).ok()?,
        read_only_hint: annotations
            .and_then(|value| value.get("readOnlyHint"))
            .and_then(Value::as_bool),
        destructive_hint: annotations
            .and_then(|value| value.get("destructiveHint"))
            .and_then(Value::as_bool),
    })
}

fn with_server(error: McpError, server: &str) -> McpError {
    match error {
        McpError::Network { message, .. } => McpError::Network {
            server: server.to_owned(),
            message,
        },
        other => other,
    }
}
