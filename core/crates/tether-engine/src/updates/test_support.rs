use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::{Asset, Release, UpdateConfig};

#[derive(Clone)]
pub(crate) struct Request {
    path: String,
    headers: Vec<(String, String)>,
}

impl Request {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

pub(crate) struct Route {
    path: String,
    reply: Reply,
}

pub(crate) struct Reply {
    status: u16,
    body: Vec<u8>,
    length: Option<u64>,
    redirect: Option<String>,
    pieces: Option<usize>,
}

pub(crate) struct Server {
    pub(crate) base_url: String,
    pub(crate) requests: Arc<Mutex<Vec<Request>>>,
    task: tokio::task::JoinHandle<()>,
}

pub(crate) async fn start_server(routes: Vec<Route>, expected: usize) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&requests);
    let task = tokio::spawn(async move {
        for _ in 0..expected {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_request(&mut socket).await;
            seen.lock().unwrap().push(request.clone());
            let route = routes
                .iter()
                .find(|route| route.path == request.path)
                .unwrap();
            write_reply(&mut socket, &route.reply).await;
        }
    });
    Server {
        base_url,
        requests,
        task,
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub(crate) fn route(path: &str, reply: Reply) -> Route {
    Route {
        path: path.into(),
        reply,
    }
}

pub(crate) fn reply(status: u16, body: &str) -> Reply {
    reply_with_length(status, body, Some(body.len() as u64))
}

pub(crate) fn reply_with_length(status: u16, body: impl AsRef<[u8]>, length: Option<u64>) -> Reply {
    Reply {
        status,
        body: body.as_ref().to_vec(),
        length,
        redirect: None,
        pieces: None,
    }
}

pub(crate) fn route_stream(path: &str, body: Vec<u8>, pieces: usize) -> Route {
    route(
        path,
        Reply {
            status: 200,
            body,
            length: None,
            redirect: None,
            pieces: Some(pieces),
        },
    )
}

pub(crate) fn local_config(base_url: &str) -> UpdateConfig {
    UpdateConfig {
        api_base: base_url.into(),
        repo: "khanblair/tether".into(),
        asset_prefix: "tether-full-".into(),
        allow_local_http: true,
    }
}

pub(crate) fn test_release(base_url: &str, size: u64) -> Release {
    Release {
        version: "0.0.3".into(),
        tag: "v0.0.3".into(),
        notes: String::new(),
        published_at: "2026-11-19T00:00:00Z".into(),
        apk: Asset {
            name: "tether-full-v0.0.3.apk".into(),
            url: format!("{base_url}/apk"),
            size,
        },
        sums: Asset {
            name: "SHA256SUMS".into(),
            url: format!("{base_url}/sums"),
            size: 64,
        },
    }
}

pub(crate) fn digest_line(bytes: &[u8], name: &str) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:02x}  {name}\n")
}

pub(crate) fn release_json(
    tag: &str,
    notes: Option<&str>,
    flags: (bool, bool),
    urls: (&str, &str),
    apk_size: u64,
) -> String {
    let (draft, prerelease) = flags;
    let (apk_url, sums_url) = urls;
    let body = notes.map_or("null".into(), |notes| format!("{notes:?}"));
    format!(
        r#"{{"tag_name":"{tag}","body":{body},"draft":{draft},"prerelease":{prerelease},"published_at":"2026-11-19T00:00:00Z","assets":[{{"name":"tether-full-{tag}.apk","size":{apk_size},"browser_download_url":"{apk_url}"}},{{"name":"SHA256SUMS","size":64,"browser_download_url":"{sums_url}"}}]}}"#
    )
}

async fn read_request(socket: &mut tokio::net::TcpStream) -> Request {
    let mut bytes = Vec::new();
    loop {
        let mut buffer = [0_u8; 1024];
        let count = socket.read(&mut buffer).await.unwrap();
        bytes.extend_from_slice(&buffer[..count]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8_lossy(&bytes);
    let mut lines = text.lines();
    let path = lines
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_owned();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.into(), value.trim().into()))
        .collect();
    Request { path, headers }
}

async fn write_reply(socket: &mut tokio::net::TcpStream, reply: &Reply) {
    let status_text = if reply.status == 200 { "OK" } else { "Status" };
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nconnection: close\r\n",
        reply.status, status_text
    );
    if let Some(location) = &reply.redirect {
        write!(head, "location: {location}\r\n").expect("String writes cannot fail");
    }
    if let Some(length) = reply.length {
        write!(head, "content-length: {length}\r\n").expect("String writes cannot fail");
    }
    head.push_str("\r\n");
    socket.write_all(head.as_bytes()).await.unwrap();
    if let Some(count) = reply.pieces {
        for piece in reply.body.chunks(reply.body.len().max(1) / count.max(1)) {
            socket.write_all(piece).await.unwrap();
            socket.flush().await.unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    } else {
        socket.write_all(&reply.body).await.unwrap();
    }
}
