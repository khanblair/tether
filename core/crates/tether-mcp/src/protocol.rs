//! JSON-RPC and SSE response handling.

use serde_json::{Value, json};
use tether_providers::sse::SseParser;
use tokio_util::sync::CancellationToken;

use crate::session::McpSession;
use crate::transport::{BodyStream, HttpReply, McpError};

#[expect(
    clippy::excessive_nesting,
    reason = "SSE messages require an event loop inside a response loop"
)]
pub(crate) async fn read_response(
    mut reply: HttpReply,
    expected_id: u64,
    session: &McpSession,
    cancel: &CancellationToken,
) -> Result<(Value, Option<String>), McpError> {
    let session_id = header(&reply.headers, "mcp-session-id");
    let sse = header(&reply.headers, "content-type")
        .is_some_and(|value| value.starts_with("text/event-stream"));
    if !sse {
        let bytes = read_body(&mut reply.body, session, expected_id, cancel).await?;
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| protocol(&session.config.name, "response is not JSON"))?;
        let value = message(value, expected_id, session)
            .await?
            .ok_or_else(|| protocol(&session.config.name, "response id did not match"))?;
        return Ok((value, session_id));
    }
    let mut parser = SseParser::new();
    while let Some(chunk) = next_body(&mut reply.body, session, expected_id, cancel).await? {
        for event in parser.push(&chunk) {
            if event.data.is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&event.data)
                .map_err(|_| protocol(&session.config.name, "SSE data is not JSON"))?;
            if let Some(value) = message(value, expected_id, session).await? {
                return Ok((value, session_id));
            }
        }
    }
    Err(protocol(
        &session.config.name,
        "SSE ended before the response",
    ))
}

async fn message(
    value: Value,
    expected_id: u64,
    session: &McpSession,
) -> Result<Option<Value>, McpError> {
    let Some(object) = value.as_object() else {
        return Err(protocol(
            &session.config.name,
            "JSON-RPC message is not an object",
        ));
    };
    if object.get("method").is_some() {
        if object.get("id").is_none() {
            return Ok(None);
        }
        let method = object
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let id = object.get("id").cloned().unwrap_or(Value::Null);
        let token = CancellationToken::new();
        drop(
            session
                .post(server_request_response(method, &id), true, &token)
                .await,
        );
        return Ok(None);
    }
    if object.get("id").and_then(Value::as_u64) != Some(expected_id) {
        return Ok(None);
    }
    if let Some(error) = object.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("JSON-RPC error");
        return Ok(Some(
            json!({"isError":true,"content":[{"type":"text","text":message}]}),
        ));
    }
    object
        .get("result")
        .cloned()
        .ok_or_else(|| protocol(&session.config.name, "JSON-RPC response has no result"))
        .map(Some)
}

fn server_request_response(method: &str, id: &Value) -> Value {
    if method == "ping" {
        json!({"jsonrpc":"2.0","id":id,"result":{}})
    } else {
        json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"method not found"}})
    }
}

async fn next_body(
    body: &mut BodyStream,
    session: &McpSession,
    expected_id: u64,
    cancel: &CancellationToken,
) -> Result<Option<Vec<u8>>, McpError> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(session.cancelled(expected_id).await),
        chunk = body.next_chunk() => chunk.map_err(|_| McpError::Network { server: session.config.name.clone(), message: "response body read failed".to_owned() }),
    }
}

async fn read_body(
    body: &mut BodyStream,
    session: &McpSession,
    expected_id: u64,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, McpError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = next_body(body, session, expected_id, cancel).await? {
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub(crate) fn header(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.clone())
}
pub(crate) fn protocol(server: &str, message: &str) -> McpError {
    McpError::Protocol {
        server: server.to_owned(),
        message: message.to_owned(),
    }
}
