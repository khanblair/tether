use std::sync::Arc;

use serde_json::{Value, json};
use tether_vcs::GithubApiResponse;
use tokio_util::sync::CancellationToken;

use super::{GithubClientFactory, PreparedCall, string, target_tool};
use crate::permissions::Effect;

const MAX_BODY: usize = 100 * 1024;
const MAX_PATH: usize = 2000;
const MAX_RESPONSE_BODY: usize = 48 * 1024;
// Credential and ownership changes need the strongest approval.
const SENSITIVE_PATHS: &[&str] = &[
    "/keys",
    "/gpg_keys",
    "/ssh_signing_keys",
    "/emails",
    "/hooks",
    "/collaborators",
    "/invitations",
    "/memberships",
    "/members",
    "/secrets",
    "/transfer",
    "/authorizations",
    "/installations",
];

/// Returns the GitHub API tool specification.
pub(super) fn spec() -> (&'static str, &'static str, Value) {
    (
        "github_api",
        "Call the GitHub API as the signed-in user, for anything on GitHub: repositories, issues, pull requests, comments, reviews, Actions, releases, search, gists, and other repositories. REST paths start with /, such as /user/repos. For GraphQL use POST /graphql with {\"query\": \"...\"}. The saved token is added for you. Writes and deletes ask the user first.",
        json!({
            "type": "object",
            "properties": {
                "method": {"type":"string", "enum":["GET","POST","PUT","PATCH","DELETE"]},
                "path": {"type":"string", "minLength":1, "maxLength":2000},
                "body": {"oneOf":[{"type":"object"},{"type":"array"}]}
            },
            "required":["method","path"],
            "additionalProperties":false
        }),
    )
}

pub(super) fn prepare(value: Value) -> Result<PreparedCall, String> {
    let method = string(&value, "method")?;
    validate_method(method)?;
    let path = string(&value, "path")?;
    validate_path(path)?;
    validate_body(method, &value)?;
    let effect = effect(method, path, &value);
    Ok(target_tool(
        "github_api",
        effect,
        format!("{method} https://api.github.com{path}"),
        value,
    ))
}

pub(super) async fn execute(
    call: &PreparedCall,
    github: &Arc<dyn GithubClientFactory>,
    cancel: &CancellationToken,
) -> Result<String, String> {
    let client = github
        .client()
        .map_err(|error| github_client_error(&error))?;
    let method = string(&call.payload, "method")?;
    let path = string(&call.payload, "path")?;
    let body = call.payload.get("body").cloned();
    let response = tokio::select! {
        result = client.api(method, path, body) => result.map_err(|error| error.to_string())?,
        () = cancel.cancelled() => return Err("GitHub request stopped".to_owned()),
    };
    if !(200..=299).contains(&response.status) {
        return Err(api_error(&response));
    }
    Ok(response_text(response))
}

fn validate_method(method: &str) -> Result<(), String> {
    if matches!(method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE") {
        Ok(())
    } else {
        Err("method must be GET, POST, PUT, PATCH, or DELETE".to_owned())
    }
}

fn validate_path(path: &str) -> Result<(), String> {
    if path.chars().count() > MAX_PATH
        || !path.starts_with('/')
        || path.starts_with("//")
        || path.contains("://")
        || path.contains('\\')
        || path.contains('#')
        || path
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err("path is invalid".to_owned());
    }
    let (path_part, query) = path.split_once('?').unwrap_or((path, ""));
    if path_part.split('/').any(|segment| segment == "..")
        || query
            .split('&')
            .any(|part| part.split_once('=').map_or(part, |(key, _)| key) == "access_token")
    {
        return Err("path is invalid".to_owned());
    }
    let decoded_path_part =
        decode_path_part(path_part).ok_or_else(|| "path is invalid".to_owned())?;
    if decoded_path_part.starts_with("//")
        || decoded_path_part.contains("://")
        || decoded_path_part.contains('\\')
        || decoded_path_part.contains('#')
        || decoded_path_part
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
        || decoded_path_part.split('/').any(|segment| segment == "..")
    {
        return Err("path is invalid".to_owned());
    }
    Ok(())
}

fn decode_path_part(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'%' {
            decoded.push(decode_escape(bytes, index)?);
            index += 3;
        } else {
            decoded.push(byte);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn decode_escape(bytes: &[u8], index: usize) -> Option<u8> {
    let high = bytes.get(index + 1).and_then(|byte| hex_value(*byte))?;
    let low = bytes.get(index + 2).and_then(|byte| hex_value(*byte))?;
    Some((high << 4) | low)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn validate_body(method: &str, value: &Value) -> Result<(), String> {
    let Some(body) = value.get("body") else {
        return Ok(());
    };
    if method == "GET" {
        return Err("GET cannot have a body".to_owned());
    }
    if !body.is_object() && !body.is_array() {
        return Err("body must be a JSON object or array".to_owned());
    }
    let size = serde_json::to_vec(body)
        .map_err(|_| "body is not valid JSON".to_owned())?
        .len();
    if size > MAX_BODY {
        Err("body is too large".to_owned())
    } else {
        Ok(())
    }
}

fn effect(method: &str, path: &str, value: &Value) -> Effect {
    if method == "GET" {
        return Effect::Network;
    }
    if method == "DELETE" {
        return Effect::ForcePush;
    }
    let path_without_query = path.split_once('?').map_or(path, |(path, _)| path);
    let decoded_path =
        decode_path_part(path_without_query).unwrap_or_else(|| path_without_query.to_owned());
    let lower_path = decoded_path.to_ascii_lowercase();
    if lower_path.starts_with("/applications")
        || SENSITIVE_PATHS
            .iter()
            .any(|sensitive| lower_path.contains(sensitive))
    {
        return Effect::ForcePush;
    }
    if decoded_path == "/graphql" {
        let query = value
            .get("body")
            .and_then(Value::as_object)
            .and_then(|body| body.get("query"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !contains_word(query, "mutation") {
            return Effect::Network;
        }
    }
    Effect::Push
}

fn contains_word(text: &str, word: &str) -> bool {
    text.to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|part| part == word)
}

fn response_text(response: GithubApiResponse) -> String {
    let body = cap_body(response.body);
    let mut lines = vec![format!("status: {}", response.status)];
    if let Some(next) = response.next {
        lines.push(format!("next: {next}"));
    }
    if let Some(remaining) = response.rate_limit_remaining {
        lines.push(format!("rate limit remaining: {remaining}"));
    }
    format!("{}\n\n{body}", lines.join("\n"))
}

fn cap_body(mut body: String) -> String {
    if body.len() <= MAX_RESPONSE_BODY {
        return body;
    }
    let mut end = MAX_RESPONSE_BODY;
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    body.truncate(end);
    body.push_str("\n[response truncated at 48 KiB; narrow the path or use per_page]");
    body
}

fn api_error(response: &GithubApiResponse) -> String {
    let message = serde_json::from_str::<Value>(&response.body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| response.body.chars().take(300).collect());
    let mut error = format!("GitHub returned {}: {message}", response.status);
    match response.status {
        401 => error.push_str(" The saved GitHub token was rejected. Ask the user to sign in again in Settings, GitHub."),
        403 | 404 => error.push_str(" The saved token may lack the scope or repository access for this call."),
        _ => {}
    }
    error
}

fn github_client_error(error: &str) -> String {
    if error.contains("not signed in") {
        "Sign in to GitHub in Settings first".to_owned()
    } else {
        error.to_owned()
    }
}

#[cfg(test)]
pub(super) fn test_cap_body(body: String) -> String {
    cap_body(body)
}
