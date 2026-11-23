use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::{
    BodyStream, HttpDelete, HttpPost, HttpReply, McpError, McpServerConfig, McpSession,
    McpToolResult, McpTransport, Secret, TransportFuture, namespaced_tool_name,
    parse_namespaced_tool_name,
};

struct ScriptedTransport {
    replies: Mutex<VecDeque<HttpReply>>,
    posts: Mutex<Vec<HttpPost>>,
    deletes: Mutex<Vec<HttpDelete>>,
}

impl ScriptedTransport {
    fn new(replies: Vec<HttpReply>) -> Self {
        Self {
            replies: Mutex::new(replies.into()),
            posts: Mutex::new(Vec::new()),
            deletes: Mutex::new(Vec::new()),
        }
    }

    fn next(&self) -> Result<HttpReply, McpError> {
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| McpError::Network {
                server: "fake".to_owned(),
                message: "script ended".to_owned(),
            })
    }
}

impl McpTransport for ScriptedTransport {
    fn post(&self, request: HttpPost) -> TransportFuture {
        self.posts.lock().unwrap().push(request);
        let reply = self.next();
        Box::pin(async move { reply })
    }

    fn delete(&self, request: HttpDelete) -> TransportFuture {
        self.deletes.lock().unwrap().push(request);
        let reply = self.next();
        Box::pin(async move { reply })
    }
}

struct CancelTransport {
    posts: Mutex<Vec<HttpPost>>,
    call_started: Notify,
}

impl CancelTransport {
    fn new() -> Self {
        Self {
            posts: Mutex::new(Vec::new()),
            call_started: Notify::new(),
        }
    }
}

impl McpTransport for CancelTransport {
    fn post(&self, request: HttpPost) -> TransportFuture {
        let value: Value = serde_json::from_slice(&request.body).unwrap();
        let is_call = value.get("method").and_then(Value::as_str) == Some("tools/call");
        self.posts.lock().unwrap().push(request);
        if is_call {
            self.call_started.notify_one();
            return Box::pin(std::future::pending());
        }
        let reply = if value.get("method").and_then(Value::as_str) == Some("initialize") {
            initialize_reply("session-1", "2025-06-18")
        } else {
            empty_reply(202)
        };
        Box::pin(async move { Ok(reply) })
    }

    fn delete(&self, _request: HttpDelete) -> TransportFuture {
        Box::pin(async { Ok(empty_reply(405)) })
    }
}

fn config() -> McpServerConfig {
    McpServerConfig {
        name: "Test Server".to_owned(),
        url: "https://example.test/mcp".to_owned(),
        headers: vec![(
            "Authorization".to_owned(),
            Secret::from_text("header-secret"),
        )],
    }
}

fn empty_reply(status: u16) -> HttpReply {
    HttpReply {
        status,
        headers: Vec::new(),
        body: BodyStream::from_chunks(Vec::new()),
    }
}

fn json_reply(value: &Value) -> HttpReply {
    HttpReply {
        status: 200,
        headers: vec![("content-type".to_owned(), "application/json".to_owned())],
        body: BodyStream::from_chunks(vec![serde_json::to_vec(&value).unwrap()]),
    }
}

fn initialize_reply(session: &str, version: &str) -> HttpReply {
    initialize_reply_with_id(session, version, 1)
}

fn initialize_reply_with_id(session: &str, version: &str, id: u64) -> HttpReply {
    HttpReply {
        status: 200,
        headers: vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("mcp-session-id".to_owned(), session.to_owned()),
        ],
        body: BodyStream::from_chunks(vec![
            serde_json::to_vec(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"protocolVersion": version}
            }))
            .unwrap(),
        ]),
    }
}

fn initialized_script(session: &str, version: &str) -> Vec<HttpReply> {
    vec![initialize_reply(session, version), empty_reply(202)]
}

fn initialized_script_with_id(session: &str, version: &str, id: u64) -> Vec<HttpReply> {
    vec![
        initialize_reply_with_id(session, version, id),
        empty_reply(202),
    ]
}

fn join_scripts(scripts: Vec<Vec<HttpReply>>) -> Vec<HttpReply> {
    scripts.into_iter().flatten().collect()
}

fn tool(name: &str) -> Value {
    json!({"name": name, "inputSchema": {"type": "object"}})
}

fn request_method(request: &HttpPost) -> String {
    let value: Value = serde_json::from_slice(&request.body).unwrap();
    value["method"].as_str().unwrap_or_default().to_owned()
}

#[tokio::test]
async fn negotiates_fallback_and_rejects_unknown_versions() {
    let transport = Arc::new(ScriptedTransport::new(initialized_script(
        "fallback",
        "2025-03-26",
    )));
    let session = McpSession::connect(config(), transport.clone(), CancellationToken::new())
        .await
        .unwrap();

    {
        let posts = transport.posts.lock().unwrap();
        assert_eq!(
            posts[1].headers[2],
            ("MCP-Protocol-Version".to_owned(), "2025-03-26".to_owned())
        );
        assert!(session.session_id.is_some());
    }

    let transport = Arc::new(ScriptedTransport::new(initialized_script(
        "bad",
        "2099-01-01",
    )));
    let result = McpSession::connect(config(), transport, CancellationToken::new()).await;

    assert!(
        matches!(result, Err(McpError::Protocol { message, .. }) if message == "unsupported protocol version")
    );
}

#[tokio::test]
async fn json_and_sse_replies_have_the_same_result() {
    let result = json!({"tools": [tool("read")]});
    let json_transport = Arc::new(ScriptedTransport::new(join_scripts(vec![
        initialized_script("json", "2025-06-18"),
        vec![json_reply(&json!({"jsonrpc":"2.0","id":2,"result":result}))],
    ])));
    let mut json_session = McpSession::connect(config(), json_transport, CancellationToken::new())
        .await
        .unwrap();
    let json_result = json_session
        .list_tools(&CancellationToken::new())
        .await
        .unwrap();

    let data = b"data: {\"jsonrpc\":\"2.0\",\"method\":\"notice\"}\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"ping\"}\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"read\",\"inputSchema\":{\"type\":\"object\"}}]}}\n\n";
    let split = data.len() / 2;
    let sse_reply = HttpReply {
        status: 200,
        headers: vec![("content-type".to_owned(), "text/event-stream".to_owned())],
        body: BodyStream::from_chunks(vec![data[..split].to_vec(), data[split..].to_vec()]),
    };
    let sse_transport = Arc::new(ScriptedTransport::new(join_scripts(vec![
        initialized_script("sse", "2025-06-18"),
        vec![sse_reply, empty_reply(202)],
    ])));
    let mut sse_session =
        McpSession::connect(config(), sse_transport.clone(), CancellationToken::new())
            .await
            .unwrap();
    let sse_result = sse_session
        .list_tools(&CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(json_result, sse_result);
    let posts = sse_transport.posts.lock().unwrap();
    let ping_response: Value = serde_json::from_slice(&posts[3].body).unwrap();
    assert_eq!(ping_response["id"], 8);
    assert_eq!(ping_response["result"], json!({}));
}

#[tokio::test]
async fn lists_cursor_pages_skips_bad_tools_and_caps_at_500() {
    let mut first = (0..249)
        .map(|i| tool(&format!("first-{i}")))
        .collect::<Vec<_>>();
    first.push(json!({"name":"bad"}));
    let second = (0..251)
        .map(|i| tool(&format!("second-{i}")))
        .collect::<Vec<_>>();
    let replies = join_scripts(vec![
        initialized_script("list", "2025-06-18"),
        vec![json_reply(
            &json!({"jsonrpc":"2.0","id":2,"result":{"tools": first, "nextCursor": "page-2"}}),
        )],
        vec![json_reply(
            &json!({"jsonrpc":"2.0","id":3,"result":{"tools": second, "nextCursor": "ignored"}}),
        )],
    ]);
    let transport = Arc::new(ScriptedTransport::new(replies));
    let mut session = McpSession::connect(config(), transport.clone(), CancellationToken::new())
        .await
        .unwrap();

    let listing = session.list_tools(&CancellationToken::new()).await.unwrap();

    assert_eq!(listing.tools.len(), 500);
    assert_eq!(listing.warnings, vec!["skipped malformed MCP tool"]);
    let posts = transport.posts.lock().unwrap();
    assert_eq!(posts.len(), 4);
    let second_request: Value = serde_json::from_slice(&posts[3].body).unwrap();
    assert_eq!(second_request["params"]["cursor"], "page-2");
}

#[tokio::test]
async fn renders_all_tool_result_shapes_and_json_rpc_errors() {
    let result = json!({
        "isError": true,
        "content": [
            {"type":"text","text":"hello"},
            {"type":"image","mimeType":"image/png","data":"abcd"},
            {"type":"resource_link","name":"doc","uri":"urn:doc"},
            {"type":"resource","resource":{"text":"embedded"}}
        ]
    });
    let replies = join_scripts(vec![
        initialized_script("call", "2025-06-18"),
        vec![json_reply(&json!({"jsonrpc":"2.0","id":2,"result":result}))],
        vec![json_reply(
            &json!({"jsonrpc":"2.0","id":3,"result":{"structuredContent":{"ok":true}}}),
        )],
        vec![json_reply(
            &json!({"jsonrpc":"2.0","id":4,"error":{"code":-32000,"message":"nope"}}),
        )],
    ]);
    let transport = Arc::new(ScriptedTransport::new(replies));
    let mut session = McpSession::connect(config(), transport, CancellationToken::new())
        .await
        .unwrap();

    let rendered = session
        .call_tool("demo", "{}", &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(
        rendered,
        McpToolResult {
            text: "hello\n[image: image/png, 4 bytes]\ndoc (urn:doc)\nembedded".to_owned(),
            is_error: true,
            truncated: false,
        }
    );
    let structured = session
        .call_tool("demo", "{}", &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(structured.text, r#"{"ok":true}"#);
    let error = session
        .call_tool("demo", "{}", &CancellationToken::new())
        .await
        .unwrap();
    assert!(error.is_error);
    assert_eq!(error.text, "nope");
}

#[tokio::test]
async fn truncates_tool_results_at_one_mib_on_a_character_boundary() {
    let large = "é".repeat(600_000);
    let transport = Arc::new(ScriptedTransport::new(join_scripts(vec![
        initialized_script("large", "2025-06-18"),
        vec![json_reply(
            &json!({"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":large}]}}),
        )],
    ])));
    let mut session = McpSession::connect(config(), transport, CancellationToken::new())
        .await
        .unwrap();

    let result = session
        .call_tool("large", "{}", &CancellationToken::new())
        .await
        .unwrap();

    assert!(result.truncated);
    assert!(result.text.ends_with("\n[truncated]"));
    assert!(result.text.is_char_boundary(result.text.len()));
}

#[tokio::test]
async fn cancellation_sends_cancelled_notification_with_request_id() {
    let transport = Arc::new(CancelTransport::new());
    let cancel = CancellationToken::new();
    let mut session = McpSession::connect(config(), transport.clone(), cancel.clone())
        .await
        .unwrap();
    let call_cancel = cancel.clone();
    let call = tokio::spawn(async move { session.call_tool("slow", "{}", &call_cancel).await });
    transport.call_started.notified().await;
    cancel.cancel();
    let result = call.await.unwrap();

    assert!(matches!(result, Err(McpError::Cancelled { .. })));
    let posts = transport.posts.lock().unwrap();
    let cancelled = posts
        .iter()
        .find(|post| request_method(post) == "notifications/cancelled")
        .unwrap();
    let body: Value = serde_json::from_slice(&cancelled.body).unwrap();
    assert_eq!(body["params"]["requestId"], 2);
}

#[tokio::test]
async fn reinitializes_once_after_session_loss_and_retries_once() {
    let replies = join_scripts(vec![
        initialized_script("one", "2025-06-18"),
        vec![empty_reply(404)],
        initialized_script_with_id("two", "2025-06-18", 3),
        vec![json_reply(
            &json!({"jsonrpc":"2.0","id":2,"result":{"tools":[]}}),
        )],
    ]);
    let transport = Arc::new(ScriptedTransport::new(replies));
    let mut session = McpSession::connect(config(), transport.clone(), CancellationToken::new())
        .await
        .unwrap();

    session.list_tools(&CancellationToken::new()).await.unwrap();

    let posts = transport.posts.lock().unwrap();
    assert_eq!(
        posts
            .iter()
            .filter(|post| request_method(post) == "initialize")
            .count(),
        2
    );
    assert_eq!(
        posts
            .iter()
            .filter(|post| request_method(post) == "tools/list")
            .count(),
        2
    );
    assert_eq!(
        posts[4]
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("mcp-session-id"))
            .map(|(_, value)| value.as_str()),
        Some("two")
    );
}

#[tokio::test]
async fn two_session_losses_return_the_second_404() {
    let replies = join_scripts(vec![
        initialized_script("one", "2025-06-18"),
        vec![empty_reply(404)],
        initialized_script_with_id("two", "2025-06-18", 3),
        vec![empty_reply(404)],
    ]);
    let transport = Arc::new(ScriptedTransport::new(replies));
    let mut session = McpSession::connect(config(), transport, CancellationToken::new())
        .await
        .unwrap();

    let result = session.list_tools(&CancellationToken::new()).await;

    assert!(matches!(result, Err(McpError::Http { status: 404, .. })));
}

#[tokio::test]
async fn close_sends_delete_and_accepts_405() {
    let transport = Arc::new(ScriptedTransport::new(join_scripts(vec![
        initialized_script("close", "2025-06-18"),
        vec![empty_reply(405)],
    ])));
    let mut session = McpSession::connect(config(), transport.clone(), CancellationToken::new())
        .await
        .unwrap();

    session.close().await.unwrap();

    let deletes = transport.deletes.lock().unwrap();
    assert_eq!(deletes.len(), 1);
    assert!(
        deletes[0]
            .headers
            .iter()
            .any(|(name, value)| name.eq_ignore_ascii_case("mcp-session-id") && value == "close")
    );
}

#[test]
fn config_never_debugs_or_displays_header_values() {
    let config = config();
    let debug = format!("{config:?}");
    assert!(!debug.contains("header-secret"));

    let error = McpError::InvalidConfig {
        server: "server".to_owned(),
        message: "invalid header".to_owned(),
    };
    assert!(!format!("{error:?}").contains("header-secret"));
    assert!(!format!("{error}").contains("header-secret"));
}

#[test]
fn names_are_bounded_stable_collision_resistant_and_invertible() {
    let sanitized = namespaced_tool_name("My Server", "Read.File");
    assert_eq!(sanitized, "mcp__my_server__read_file");
    assert_eq!(
        parse_namespaced_tool_name(&sanitized),
        Some(("my_server".to_owned(), "read_file".to_owned()))
    );

    let first = namespaced_tool_name(&"server".repeat(40), &"tool".repeat(40));
    let second = namespaced_tool_name(&"server".repeat(40), &format!("{}x", "tool".repeat(40)));
    assert!(first.len() <= 64);
    assert_eq!(
        first,
        namespaced_tool_name(&"server".repeat(40), &"tool".repeat(40))
    );
    assert_ne!(first, second);
    assert!(parse_namespaced_tool_name(&first).is_some());
}
