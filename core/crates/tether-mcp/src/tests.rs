use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{
    BodyStream, HttpDelete, HttpPost, HttpReply, McpError, McpServerConfig, McpSession,
    McpTransport, Secret, TransportFuture,
};

struct FakeTransport {
    replies: Mutex<VecDeque<HttpReply>>,
    posts: Mutex<Vec<HttpPost>>,
    deletes: Mutex<Vec<HttpDelete>>,
}

impl FakeTransport {
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

impl McpTransport for FakeTransport {
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

fn json_reply(value: &Value) -> HttpReply {
    HttpReply {
        status: 200,
        headers: vec![("content-type".to_owned(), "application/json".to_owned())],
        body: BodyStream::from_chunks(vec![serde_json::to_vec(&value).unwrap()]),
    }
}
fn empty_reply(status: u16) -> HttpReply {
    HttpReply {
        status,
        headers: vec![],
        body: BodyStream::from_chunks(vec![]),
    }
}
fn config() -> McpServerConfig {
    McpServerConfig {
        name: "Test Server".to_owned(),
        url: "https://example.test/mcp?secret=query".to_owned(),
        headers: vec![(
            "Authorization".to_owned(),
            Secret::from_text("header-secret"),
        )],
    }
}
fn initialize_reply() -> HttpReply {
    HttpReply {
        status: 200,
        headers: vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("Mcp-Session-Id".to_owned(), "session-1".to_owned()),
        ],
        body: BodyStream::from_chunks(vec![
            br#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18"}}"#.to_vec(),
        ]),
    }
}

#[tokio::test]
async fn handshake_sends_client_info_and_session_headers() {
    let transport = Arc::new(FakeTransport::new(vec![
        initialize_reply(),
        empty_reply(202),
        json_reply(
            &serde_json::json!({"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"read","inputSchema":{"type":"object"}}]}}),
        ),
    ]));
    let cancel = CancellationToken::new();
    let mut session = McpSession::connect(config(), transport.clone(), cancel.clone())
        .await
        .unwrap();
    let listing = session.list_tools(&cancel).await.unwrap();
    assert_eq!(listing.tools[0].name, "read");
    let posts = transport.posts.lock().unwrap();
    let initialize: Value = serde_json::from_slice(&posts[0].body).unwrap();
    assert_eq!(initialize["params"]["clientInfo"]["name"], "tether");
    assert_eq!(initialize["params"]["protocolVersion"], "2025-06-18");
    assert_eq!(
        posts[1].body,
        br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
    );
    assert!(
        posts[2].headers.iter().any(
            |(name, value)| name.eq_ignore_ascii_case("mcp-session-id") && value == "session-1"
        )
    );
    assert!(
        posts[2]
            .headers
            .iter()
            .any(|(name, value)| name == "MCP-Protocol-Version" && value == "2025-06-18")
    );
    assert!(
        !format!(
            "{:?}",
            McpError::InvalidConfig {
                server: "Test Server".to_owned(),
                message: "invalid header".to_owned()
            }
        )
        .contains("header-secret")
    );
}

#[tokio::test]
async fn sse_reply_handles_split_events_and_ping() {
    let stream = b"data: {\"jsonrpc\":\"2.0\",\"method\":\"notice\"}\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"ping\"}\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[]}}\n\n";
    let split = stream.len() / 2;
    let sse = HttpReply {
        status: 200,
        headers: vec![("content-type".to_owned(), "text/event-stream".to_owned())],
        body: BodyStream::from_chunks(vec![stream[..split].to_vec(), stream[split..].to_vec()]),
    };
    let transport = Arc::new(FakeTransport::new(vec![
        initialize_reply(),
        empty_reply(202),
        sse,
        empty_reply(202),
    ]));
    let cancel = CancellationToken::new();
    let mut session = McpSession::connect(config(), transport.clone(), cancel.clone())
        .await
        .unwrap();
    let listing = session.list_tools(&cancel).await.unwrap();
    assert_eq!(listing.tools.len(), 0);
    let posts = transport.posts.lock().unwrap();
    let ping: Value = serde_json::from_slice(&posts[3].body).unwrap();
    assert_eq!(ping["id"], 9);
    assert_eq!(ping["result"], serde_json::json!({}));
}
