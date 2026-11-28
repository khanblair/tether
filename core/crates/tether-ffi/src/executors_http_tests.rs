use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use reqwest::Url;

use super::http::{
    Download, client_with, copy_download, fetch_bytes, fetch_failure, fetch_manifest,
    redirect_allowed,
};
use super::{ExecutorPaths, ExecutorRuntimeError, rootfs_image, rootfs_key};
use tokio_util::sync::CancellationToken;

#[test]
fn download_urls_require_https() {
    assert!(super::http::require_https("http://example.test/manifest.json").is_err());
    assert!(super::http::require_https("https://example.test/manifest.json").is_ok());
}

#[test]
fn redirect_policy_allows_github_hosts_through_five_hops() {
    for url in [
        "https://github.com/x",
        "https://objects.githubusercontent.com/x",
        "https://release-assets.githubusercontent.com/x",
        "https://githubusercontent.com/x",
    ] {
        assert!(redirect_allowed(&Url::parse(url).unwrap(), 0));
        assert!(redirect_allowed(&Url::parse(url).unwrap(), 4));
    }
}

#[test]
fn redirect_policy_rejects_non_github_hosts_and_sixth_hop() {
    for url in [
        "http://github.com/x",
        "https://evilgithub.com/x",
        "https://github.com.evil.com/x",
        "https://githubusercontent.com.evil.org/x",
        "https://notgithubusercontent.com/x",
        "https://127.0.0.1/x",
    ] {
        assert!(!redirect_allowed(&Url::parse(url).unwrap(), 0));
    }
    assert!(!redirect_allowed(
        &Url::parse("https://github.com/x").unwrap(),
        5
    ));
}

#[test]
fn rootfs_image_reports_verification_failure_for_bad_manifest() {
    let result = rootfs_image(b"garbage", &[], &[0_u8; 32]);

    assert!(matches!(
        result,
        Err(ExecutorRuntimeError::VerificationFailed)
    ));
}

#[test]
fn rootfs_image_accepts_a_correctly_signed_manifest() {
    let manifest = br#"{"format":1,"alpine_version":"3.20","images":[{"arch":"aarch64","url":"https://example.invalid/aarch64.tar.gz","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size_bytes":1,"max_extracted_bytes":1},{"arch":"x86_64","url":"https://example.invalid/x86_64.tar.gz","sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","size_bytes":1,"max_extracted_bytes":1}],"base_packages":[]}"#;
    let signature = decode_hex::<64>(
        "69da343fb2158fa90842433aa72281e60076054060b2294f62812136143ac3703bb3e6a8c447b3af4a6a0cb2976314ed84ac126ea1d5dca6b3baf84c9fe1eb0e",
    );
    let key = decode_hex::<32>("fd99f5fb7c6f86c75bbda4aa753b0bae91f4e11bdd8e4525ad5ee7a0b5dac003");

    let image = rootfs_image(manifest, &signature, &key).unwrap();

    assert_eq!(image.size_bytes, 1);
}

#[test]
fn missing_rootfs_key_is_invalid_configuration() {
    let paths = ExecutorPaths {
        packs_root: "/packs".into(),
        linux_root: "/linux".into(),
        proot_path: None,
        loader_path: "/loader".into(),
        pack_key: None,
        rootfs_key: None,
    };

    assert!(matches!(
        rootfs_key(&paths),
        Err(ExecutorRuntimeError::InvalidConfiguration)
    ));
}

#[test]
fn loopback_redirect_downloads_the_body() {
    let (url, server) = start_server(ServerResponse::Redirect);
    let client = client_with(|_, hops| hops < 5).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let result = runtime.block_on(fetch_bytes(&client, &url, 1024, "manifest"));
    server.join().unwrap();

    assert_eq!(result.unwrap(), b"downloaded body");
}

#[test]
fn loopback_redirect_omits_referer_header() {
    let (url, server) = start_server(ServerResponse::Redirect);
    let client = client_with(|_, hops| hops < 5).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let result = runtime.block_on(fetch_bytes(&client, &url, 1024, "manifest"));
    server.join().unwrap();

    assert_eq!(result.unwrap(), b"downloaded body");
}

#[test]
fn loopback_rejected_redirect_is_an_error() {
    let (url, server) = start_server(ServerResponse::RedirectOnly);
    let client = client_with(|_, _| false).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let result = runtime.block_on(fetch_bytes(&client, &url, 1024, "manifest"));
    server.join().unwrap();

    assert!(result.is_err());
}

#[test]
fn loopback_final_404_is_an_error() {
    let (url, server) = start_server(ServerResponse::NotFound);
    let client = client_with(|_, hops| hops < 5).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let result = runtime.block_on(fetch_bytes(&client, &url, 1024, "manifest"));
    server.join().unwrap();

    assert!(result.is_err());
}

#[test]
fn loopback_manifest_503_is_a_setup_failure() {
    let (url, server) = start_server(ServerResponse::Unavailable);
    let client = client_with(|_, _| false).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let result = runtime.block_on(fetch_manifest(&client, &url));
    server.join().unwrap();

    assert!(matches!(
        result,
        Err(ExecutorRuntimeError::SetupFailed { reason })
            if reason.contains("manifest: HTTP 503")
    ));
}

#[test]
fn loopback_refused_manifest_reports_connection_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let client = client_with(|_, _| false).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let url = format!("http://{address}/manifest");

    let result = runtime.block_on(fetch_manifest(&client, &url));

    assert!(matches!(
        result,
        Err(ExecutorRuntimeError::SetupFailed { reason })
            if reason.contains("manifest: could not connect")
    ));
}

#[test]
fn signature_fetch_failures_are_prefixed() {
    let error = fetch_failure("signature", "timed out");

    assert!(matches!(
        error,
        ExecutorRuntimeError::SetupFailed { reason }
            if reason == "signature: timed out"
    ));
}

#[test]
fn loopback_http_503_reports_the_status() {
    let (url, server) = start_server(ServerResponse::Unavailable);
    let client = client_with(|_, _| false).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let cancel = CancellationToken::new();
    let download = Download {
        client: &client,
        url: &url,
        limit: 1,
        cancel: &cancel,
    };
    let mut output = Vec::new();

    let result = runtime.block_on(copy_download(&download, &mut output, &|_, _| {}));
    server.join().unwrap();

    assert!(result.unwrap_err().contains("HTTP 503"));
}

#[test]
fn loopback_mid_body_reports_connection_loss() {
    let (url, server) = start_server(ServerResponse::MidBody);
    let client = client_with(|_, _| false).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let cancel = CancellationToken::new();
    let download = Download {
        client: &client,
        url: &url,
        limit: 15,
        cancel: &cancel,
    };
    let mut output = Vec::new();

    let result = runtime.block_on(copy_download(&download, &mut output, &|_, _| {}));
    server.join().unwrap();

    assert!(result.unwrap_err().contains("connection lost"));
}

enum ServerResponse {
    /// A redirect, then the file on a second connection.
    Redirect,
    /// A redirect on one connection only, for a client that does not follow it.
    RedirectOnly,
    NotFound,
    Unavailable,
    MidBody,
}

fn start_server(response: ServerResponse) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let url = format!("http://{address}/start");
    let thread = thread::spawn(move || serve_response(&listener, address, &response));
    (url, thread)
}

fn serve_response(
    listener: &TcpListener,
    address: std::net::SocketAddr,
    response: &ServerResponse,
) {
    match response {
        ServerResponse::Redirect => serve_redirect(listener, address, true),
        ServerResponse::RedirectOnly => serve_redirect(listener, address, false),
        ServerResponse::NotFound => serve_single_response(
            listener,
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        ),
        ServerResponse::Unavailable => serve_single_response(
            listener,
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        ),
        ServerResponse::MidBody => serve_mid_body(listener),
    }
}

fn serve_redirect(listener: &TcpListener, address: std::net::SocketAddr, serve_file: bool) {
    let (mut first, _) = accept_with_deadline(listener);
    let _first_request = read_request(&mut first);
    write_response(
        &mut first,
        &format!(
            "HTTP/1.1 302 Found\r\nLocation: http://{address}/file\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ),
    );
    if serve_file {
        let (mut second, _) = accept_with_deadline(listener);
        let second_request = read_request(&mut second);
        assert!(!has_header(&second_request, "referer"));
        write_response(
            &mut second,
            "HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\ndownloaded body",
        );
    }
}

fn serve_single_response(listener: &TcpListener, response: &str) {
    let (mut stream, _) = accept_with_deadline(listener);
    let _request = read_request(&mut stream);
    write_response(&mut stream, response);
}

fn serve_mid_body(listener: &TcpListener) {
    let (mut stream, _) = accept_with_deadline(listener);
    let _request = read_request(&mut stream);
    write_response(
        &mut stream,
        "HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\nshort",
    );
    stream.shutdown(Shutdown::Both).unwrap();
}

fn accept_with_deadline(listener: &TcpListener) -> (TcpStream, std::net::SocketAddr) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, address)) => {
                // On some systems an accepted socket inherits the listener's non-blocking mode.
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                return (stream, address);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "the client never connected");
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("accept failed: {error}"),
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let count = stream.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..count]);
    }
    request
}

fn has_header(request: &[u8], expected: &str) -> bool {
    request.split(|byte| *byte == b'\n').any(|line| {
        let Some(colon) = line.iter().position(|byte| *byte == b':') else {
            return false;
        };
        std::str::from_utf8(&line[..colon]).is_ok_and(|name| name.eq_ignore_ascii_case(expected))
    })
}

fn write_response(stream: &mut TcpStream, response: &str) {
    stream.write_all(response.as_bytes()).unwrap();
}

fn decode_hex<const N: usize>(value: &str) -> [u8; N] {
    let mut bytes = [0_u8; N];
    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    assert_eq!(remainder, &[] as &[u8]);
    for (output, pair) in bytes.iter_mut().zip(pairs) {
        *output = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap();
    }
    bytes
}
