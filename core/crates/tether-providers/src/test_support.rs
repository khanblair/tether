//! Shared local HTTP server helpers for provider tests.

use std::io;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

const PAUSE: Duration = Duration::from_millis(5);

/// The request the server received. It has no `Debug` impl because it holds the API key.
pub(crate) struct SeenRequest {
    pub(crate) request_line: String,
    pub(crate) body: String,
    headers: Vec<(String, String)>,
}

impl SeenRequest {
    /// Returns a header without regard to case.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// A canned HTTP response.
pub(crate) struct Reply {
    head: String,
    pieces: Vec<Vec<u8>>,
    chunked: bool,
    hold_open: bool,
    abort: bool,
}

impl Reply {
    /// Creates a JSON response with the supplied status and headers.
    pub(crate) fn json(status: &str, extra_headers: &[&str], body: &str) -> Self {
        let mut extra = extra_headers.join("\r\n");
        if !extra.is_empty() {
            extra.push_str("\r\n");
        }
        let head = format!(
            "HTTP/1.1 {status}\r\n{extra}content-type: application/json\r\n\
             content-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        Self {
            head,
            pieces: vec![body.as_bytes().to_vec()],
            chunked: false,
            hold_open: false,
            abort: false,
        }
    }

    /// Creates a chunked streaming response.
    pub(crate) fn stream(sse: &str, piece_len: usize) -> Self {
        let head = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\
                    transfer-encoding: chunked\r\nconnection: close\r\n\r\n";
        Self {
            head: head.to_owned(),
            pieces: sse
                .as_bytes()
                .chunks(piece_len)
                .map(<[u8]>::to_vec)
                .collect(),
            chunked: true,
            hold_open: false,
            abort: false,
        }
    }

    /// Drops the connection after the last body piece.
    pub(crate) fn then_abort(mut self) -> Self {
        self.abort = true;
        self
    }

    /// Keeps the connection open after the last body piece.
    pub(crate) fn then_hold_open(mut self) -> Self {
        self.hold_open = true;
        self
    }
}

/// A running local server.
pub(crate) struct TestServer {
    /// The base URL for the provider under test.
    pub(crate) base_url: String,
    /// The received request, after the reply has been sent.
    pub(crate) seen: JoinHandle<io::Result<SeenRequest>>,
}

/// Starts a server on a free local port.
pub(crate) async fn start(reply: Reply) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let seen = tokio::spawn(serve(listener, reply));
    TestServer { base_url, seen }
}

/// Returns a local base URL where no server is listening.
pub(crate) async fn refused_base_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    format!("http://{}", listener.local_addr().unwrap())
}

async fn serve(listener: TcpListener, reply: Reply) -> io::Result<SeenRequest> {
    let (mut socket, _) = listener.accept().await?;
    socket.set_nodelay(true)?;
    let seen = read_request(&mut socket).await?;
    write_reply(&mut socket, reply).await?;
    Ok(seen)
}

async fn read_request(socket: &mut TcpStream) -> io::Result<SeenRequest> {
    let mut raw = Vec::new();
    let head_len = loop {
        read_more(socket, &mut raw).await?;
        if let Some(position) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
            break position + 4;
        }
    };
    let head = String::from_utf8_lossy(&raw[..head_len]).into_owned();
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or_default().to_owned();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .collect::<Vec<_>>();
    let length = content_length(&headers);
    while raw.len() < head_len + length {
        read_more(socket, &mut raw).await?;
    }
    let body = String::from_utf8_lossy(&raw[head_len..head_len + length]).into_owned();
    Ok(SeenRequest {
        request_line,
        body,
        headers,
    })
}

fn content_length(headers: &[(String, String)]) -> usize {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse().ok())
        .unwrap_or(0)
}

async fn read_more(socket: &mut TcpStream, raw: &mut Vec<u8>) -> io::Result<()> {
    let mut buffer = [0_u8; 4096];
    let count = socket.read(&mut buffer).await?;
    if count == 0 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    raw.extend_from_slice(&buffer[..count]);
    Ok(())
}

async fn write_reply(socket: &mut TcpStream, reply: Reply) -> io::Result<()> {
    socket.write_all(reply.head.as_bytes()).await?;
    for piece in &reply.pieces {
        if reply.chunked {
            let framed = [format!("{:x}\r\n", piece.len()).as_bytes(), piece, b"\r\n"].concat();
            socket.write_all(&framed).await?;
        } else {
            socket.write_all(piece).await?;
        }
        socket.flush().await?;
        tokio::time::sleep(PAUSE).await;
    }
    if reply.abort {
        return Ok(());
    }
    if reply.hold_open {
        std::future::pending::<()>().await;
    }
    if reply.chunked {
        socket.write_all(b"0\r\n\r\n").await?;
    }
    socket.shutdown().await
}
