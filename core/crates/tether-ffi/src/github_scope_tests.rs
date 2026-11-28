use std::collections::BTreeMap;
use std::future::{Future, ready};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tether_vcs::{HttpClient, HttpClientError, HttpRequest, HttpResponse};

use crate::{GitRuntime, SecretVault, SecretsPlatform, SecretsPlatformFailure};

struct PlainPlatform;

impl SecretsPlatform for PlainPlatform {
    fn wrap(
        &self,
        plaintext: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, SecretsPlatformFailure> {
        Ok(plaintext)
    }

    fn unwrap(
        &self,
        wrapped: Vec<u8>,
        _context: Vec<u8>,
    ) -> Result<Vec<u8>, SecretsPlatformFailure> {
        Ok(wrapped)
    }
}

struct RecordingHttp {
    request: Mutex<Option<HttpRequest>>,
}

impl HttpClient for RecordingHttp {
    fn send(
        &self,
        request: HttpRequest,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpClientError>> + Send + '_>> {
        *self.request.lock().expect("request lock") = Some(request);
        Box::pin(ready(Ok(HttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: br#"{"device_code":"device","user_code":"ABCD","verification_uri":"https://github.com/login/device","expires_in":600,"interval":5}"#.to_vec(),
        })))
    }
}

fn runtime(http: Arc<RecordingHttp>) -> (tempfile::TempDir, Arc<GitRuntime>) {
    let dir = tempfile::tempdir().expect("fixture directory");
    let vault = SecretVault::new(
        dir.path().join("secrets").to_string_lossy().into_owned(),
        Arc::new(PlainPlatform),
    );
    let runtime = GitRuntime::new(
        dir.path()
            .join("tether.db")
            .to_str()
            .expect("database path"),
        dir.path()
            .join("workspaces")
            .to_str()
            .expect("workspace path"),
        vault,
    )
    .expect("runtime");
    runtime.set_http_for_tests(http as Arc<dyn HttpClient>);
    (dir, runtime)
}

#[test]
fn device_flow_requests_the_scopes_used_by_github_api_tools() {
    let http = Arc::new(RecordingHttp {
        request: Mutex::new(None),
    });
    let (_dir, runtime) = runtime(Arc::clone(&http));
    runtime.configure_github("client").expect("client id");
    runtime.github_sign_in_start().expect("device grant");

    let request = http
        .request
        .lock()
        .expect("request lock")
        .clone()
        .expect("recorded request");
    assert_eq!(request.method, "POST");
    assert_eq!(request.url, "https://github.com/login/device/code");
    assert_eq!(
        request.body.expect("form body"),
        b"client_id=client&scope=repo+workflow+gist+read%3Aorg+read%3Auser+user%3Aemail+notifications"
    );
}
