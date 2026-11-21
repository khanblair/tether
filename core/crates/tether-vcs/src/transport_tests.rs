use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::thread;

use tether_secrets::Secret;

use super::test_support::{TempDir, add_commit, git_ok, init_repo, write};
use super::{
    CancellationToken, PushPhase, PushProgress, PushRequest, RefUpdate, ReqwestTransport, push,
};

struct Progress;

impl PushProgress for Progress {
    fn update(&self, _phase: PushPhase, _current: u64, _total: Option<u64>) {}
}

/// Finds a header value by name, ignoring case: hyper sends lowercase names.
fn header_value(headers: &str, name: &str) -> Option<String> {
    headers.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim()
            .eq_ignore_ascii_case(name)
            .then(|| value.trim().to_owned())
    })
}

fn read_request(stream: &mut TcpStream) -> (String, Vec<u8>) {
    let mut bytes = Vec::new();
    let header_end;
    loop {
        let mut chunk = [0; 4096];
        let count = stream.read(&mut chunk).expect("read HTTP request");
        assert!(count > 0, "request ended before headers");
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            header_end = end + 4;
            break;
        }
    }
    let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let length = header_value(&headers, "content-length")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    while bytes.len() - header_end < length {
        let mut chunk = [0; 4096];
        let count = stream.read(&mut chunk).expect("read HTTP body");
        assert!(count > 0, "request ended before body");
        bytes.extend_from_slice(&chunk[..count]);
    }
    (headers, bytes[header_end..header_end + length].to_vec())
}

fn respond(stream: &mut TcpStream, body: &[u8], content_type: &str) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("write HTTP headers");
    stream.write_all(body).expect("write HTTP body");
}

fn run_server(listener: &TcpListener, bare: &std::path::Path) -> bool {
    let mut authorized = false;
    for request_index in 0..2 {
        let (mut stream, _) = listener.accept().expect("accept HTTP request");
        authorized |= serve_request(&mut stream, request_index, bare);
    }
    authorized
}

fn serve_request(stream: &mut TcpStream, index: usize, bare: &std::path::Path) -> bool {
    let (headers, body) = read_request(stream);
    let authorized =
        header_value(&headers, "authorization").is_some_and(|value| value.starts_with("Basic "));
    if index == 0 {
        assert!(headers.contains("GET /repo.git/info/refs?service=git-receive-pack"));
        let output = Command::new("git")
            .args(["receive-pack", "--stateless-rpc", "--advertise-refs"])
            .arg(bare)
            .output()
            .expect("advertise refs");
        respond(
            stream,
            &output.stdout,
            "application/x-git-receive-pack-advertisement",
        );
    } else {
        receive_pack(stream, &headers, &body, bare);
    }
    authorized
}

fn receive_pack(stream: &mut TcpStream, headers: &str, body: &[u8], bare: &std::path::Path) {
    assert!(headers.contains("POST /repo.git/git-receive-pack"));
    let mut child = Command::new("git")
        .args(["receive-pack", "--stateless-rpc"])
        .arg(bare)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("receive pack");
    child
        .stdin
        .as_mut()
        .expect("receive stdin")
        .write_all(body)
        .expect("pack body");
    let output = child.wait_with_output().expect("receive output");
    assert!(output.status.success());
    respond(
        stream,
        &output.stdout,
        "application/x-git-receive-pack-result",
    );
}

#[test]
#[ignore = "needs a local socket"]
fn reqwest_transport_push_sends_authorization_and_receive_pack() {
    let (local_dir, local) = init_repo("reqwest-push");
    write(&local_dir.path().join("file"), b"content");
    let sha = add_commit(local_dir.path(), "content");
    let remote_root = TempDir::new("reqwest-push-remote");
    git_ok(remote_root.path(), &["init", "--bare", "remote.git"]);
    let bare = remote_root.path().join("remote.git");
    let server_bare = bare.clone();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local HTTP listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || run_server(&listener, &server_bare));
    let transport = ReqwestTransport::new(Secret::from_text("socket-token")).expect("transport");
    let request = PushRequest {
        remote_url: format!("http://{address}/repo.git"),
        updates: vec![RefUpdate {
            src: Some(String::from("HEAD")),
            dst: String::from("refs/heads/main"),
            force: false,
        }],
    };
    push(
        &local,
        &request,
        &transport,
        &Progress,
        &CancellationToken::new(),
    )
    .expect("socket push");
    assert!(server.join().expect("server thread"));
    assert_eq!(
        String::from_utf8(git_ok(&bare, &["rev-parse", "refs/heads/main"]))
            .expect("sha")
            .trim(),
        sha
    );
}
