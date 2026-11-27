//! Agent tools for session processes and Preview requests.

use std::fs;
use std::sync::Arc;

use serde_json::{Value, json};
use tether_providers::{BoxFuture, ToolSpec};
use tether_types::{PreviewTarget, ProcessStatus};

use super::background::{BackgroundProcesses, ProcessSnapshot};
use crate::permissions::Effect;
use crate::tool_loop::{ExecuteContext, PrepareContext, PreparedCall, ToolHandler, ToolOutput};

/// Handles process inspection, process stopping, and preview requests.
pub struct ProcessTools {
    processes: Arc<BackgroundProcesses>,
}

impl ProcessTools {
    /// Creates the process tools for one session.
    #[must_use]
    pub fn new(processes: Arc<BackgroundProcesses>) -> Self {
        Self { processes }
    }
}

impl ToolHandler for ProcessTools {
    fn specs(&self) -> Vec<ToolSpec> {
        vec![
            spec(
                "process_list",
                "List session background processes.",
                &json!({
                    "type":"object", "properties":{}, "additionalProperties":false
                }),
            ),
            spec(
                "process_logs",
                "Read recent output from a session background process.",
                &json!({
                    "type":"object", "properties": {
                        "id":{"type":"string"}, "tail":{"type":"integer","minimum":1,"maximum":200}
                    }, "required":["id"], "additionalProperties":false
                }),
            ),
            spec(
                "process_stop",
                "Stop a session background process.",
                &json!({
                    "type":"object", "properties":{"id":{"type":"string"}},
                    "required":["id"], "additionalProperties":false
                }),
            ),
            spec(
                "preview",
                "Open a project HTML file or owned local server in Preview.",
                &json!({
                    "type":"object", "properties": {
                        "path":{"type":"string"}, "url":{"type":"string"}
                    }, "additionalProperties":false
                }),
            ),
        ]
    }

    fn prepare(
        &self,
        name: &str,
        args_json: &str,
        context: &PrepareContext<'_>,
    ) -> Result<PreparedCall, String> {
        let value: Value = serde_json::from_str(args_json).map_err(|_| "invalid JSON arguments")?;
        let object = value
            .as_object()
            .ok_or("tool arguments must be an object")?;
        let allowed = match name {
            "process_list" => &[][..],
            "process_logs" => &["id", "tail"][..],
            "process_stop" => &["id"][..],
            "preview" => &["path", "url"][..],
            _ => return Err("unknown tool".into()),
        };
        if object.keys().any(|key| !allowed.contains(&key.as_str())) {
            return Err("unknown process tool argument".into());
        }
        match name {
            "process_list" => Ok(call(name, Effect::Read, "processes", value)),
            "process_logs" | "process_stop" => prepare_process(name, value),
            "preview" => prepare_preview(value, context),
            _ => Err("unknown tool".into()),
        }
    }

    fn execute<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> BoxFuture<'a, Result<ToolOutput, String>> {
        Box::pin(async move { self.execute_call(&call, &context) })
    }
}

impl ProcessTools {
    fn execute_call(
        &self,
        call: &PreparedCall,
        context: &ExecuteContext<'_>,
    ) -> Result<ToolOutput, String> {
        match call.tool.as_str() {
            "process_list" => Ok(ToolOutput {
                text: list_text(&self.processes.list()?),
            }),
            "process_logs" => {
                let id = string(&call.payload, "id")?;
                let tail = call
                    .payload
                    .get("tail")
                    .and_then(Value::as_u64)
                    .unwrap_or(80);
                let tail = usize::try_from(tail).map_err(|_| "tail is too large")?;
                Ok(ToolOutput {
                    text: self.processes.logs(id, tail)?,
                })
            }
            "process_stop" => {
                let id = string(&call.payload, "id")?;
                let snapshot = self.processes.stop(id)?;
                Ok(ToolOutput {
                    text: format_snapshot(&snapshot),
                })
            }
            "preview" => execute_preview(&self.processes, call, context),
            _ => Err("unknown tool".into()),
        }
    }
}

fn prepare_process(name: &str, value: Value) -> Result<PreparedCall, String> {
    let id = string(&value, "id")?.to_owned();
    if id.is_empty() {
        return Err("id must be a non-empty string".into());
    }
    if name == "process_logs"
        && let Some(tail) = value.get("tail")
    {
        let tail = tail.as_u64().ok_or("tail must be an integer")?;
        if !(1..=200).contains(&tail) {
            return Err("tail must be between 1 and 200".into());
        }
    }
    let effect = if name == "process_stop" {
        Effect::RunHost
    } else {
        Effect::Read
    };
    Ok(call(name, effect, &id, value))
}

fn prepare_preview(value: Value, context: &PrepareContext<'_>) -> Result<PreparedCall, String> {
    let path = value.get("path").and_then(Value::as_str);
    let url = value.get("url").and_then(Value::as_str);
    if path.is_some() == url.is_some() {
        return Err("preview needs exactly one of path or url".into());
    }
    if let Some(path) = path {
        if !path.to_ascii_lowercase().ends_with(".html") {
            return Err("preview path must be an HTML file".into());
        }
        let normalized = context
            .guard
            .resolve(path)
            .map_err(|_| "preview path must stay inside the workspace")?
            .strip_prefix(context.guard.root())
            .map_err(|_| "preview path must stay inside the workspace")?
            .to_string_lossy()
            .replace('\\', "/");
        let mut payload = value;
        payload["path"] = Value::String(normalized.clone());
        return Ok(call("preview", Effect::Read, &normalized, payload));
    }
    let url = url
        .ok_or("preview needs exactly one of path or url")?
        .to_owned();
    parse_loopback_url(&url)?;
    Ok(call("preview", Effect::Read, &url, value))
}

fn execute_preview(
    processes: &BackgroundProcesses,
    call: &PreparedCall,
    context: &ExecuteContext<'_>,
) -> Result<ToolOutput, String> {
    if let Some(path) = call.payload.get("path").and_then(Value::as_str) {
        let resolved = context
            .guard
            .resolve(path)
            .map_err(|_| "preview path must stay inside the workspace")?;
        if !fs::metadata(resolved).is_ok_and(|metadata| metadata.is_file()) {
            return Err("preview path does not exist".into());
        }
        processes.emit_preview(PreviewTarget::File { path: path.into() });
    } else {
        let url = string(&call.payload, "url")?;
        let port = parse_loopback_url(url)?;
        let process_id = processes.owner_for_port(port)?;
        processes.emit_preview(PreviewTarget::Server {
            url: url.into(),
            process_id,
        });
    }
    Ok(ToolOutput {
        text: "Preview opened in the app.".into(),
    })
}

fn parse_loopback_url(url: &str) -> Result<u16, String> {
    let prefix = "http://127.0.0.1:";
    let port_text = url
        .strip_prefix(prefix)
        .ok_or("preview URL must use http://127.0.0.1")?;
    let authority_end = port_text.find('/').unwrap_or(port_text.len());
    let port = port_text[..authority_end]
        .parse::<u16>()
        .map_err(|_| "preview URL must contain a valid port")?;
    if port == 0
        || (authority_end < port_text.len() && !port_text[authority_end..].starts_with('/'))
    {
        return Err("preview URL must use a project server URL".into());
    }
    Ok(port)
}

fn call(name: &str, effect: Effect, target: &str, payload: Value) -> PreparedCall {
    PreparedCall {
        tool: name.into(),
        effect,
        target: target.into(),
        executor: None,
        payload,
    }
}

fn spec(name: &str, description: &str, schema: &Value) -> ToolSpec {
    ToolSpec {
        name: name.into(),
        description: description.into(),
        input_schema_json: schema.to_string(),
    }
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn list_text(processes: &[ProcessSnapshot]) -> String {
    if processes.is_empty() {
        return "No background processes.".into();
    }
    processes
        .iter()
        .map(format_snapshot)
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_snapshot(process: &ProcessSnapshot) -> String {
    let status = match process.status {
        ProcessStatus::Running => "running".into(),
        ProcessStatus::Exited(code) => format!("exited ({code})"),
        ProcessStatus::Stopped => "stopped".into(),
    };
    let port = process
        .port
        .map_or_else(|| "-".into(), |port| port.to_string());
    format!(
        "{}: {} [{}] port {}",
        process.id, process.command, status, port
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::parse_loopback_url;
    use crate::tool_loop::{PrepareContext, ToolHandler};
    use tether_workspace::PathGuard;

    use super::{BackgroundProcesses, ProcessTools};

    #[test]
    fn preview_urls_accept_only_loopback_http() {
        assert_eq!(parse_loopback_url("http://127.0.0.1:5173/"), Ok(5173));
        assert!(parse_loopback_url("http://localhost:5173/").is_err());
        assert!(parse_loopback_url("https://127.0.0.1:5173/").is_err());
        assert!(parse_loopback_url("http://127.0.0.1:5173.evil/").is_err());
        assert!(parse_loopback_url("http://0.0.0.0:5173/").is_err());
    }

    #[test]
    fn preview_preparation_guards_files_and_hosts() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("index.html"), "<h1>ok</h1>").unwrap();
        let guard = PathGuard::new(root.path()).unwrap();
        let handler = ProcessTools::new(Arc::new(BackgroundProcesses::new(
            tether_types::SessionId::new("session"),
            Arc::new(NoopSink),
        )));

        let file = handler
            .prepare(
                "preview",
                r#"{"path":"index.html"}"#,
                &PrepareContext { guard: &guard },
            )
            .unwrap();
        assert_eq!(file.target, "index.html");
        assert!(
            handler
                .prepare(
                    "preview",
                    r#"{"path":"../x.html"}"#,
                    &PrepareContext { guard: &guard },
                )
                .is_err()
        );
        assert!(
            handler
                .prepare(
                    "preview",
                    r#"{"url":"http://localhost:5173/"}"#,
                    &PrepareContext { guard: &guard },
                )
                .is_err()
        );
    }

    struct NoopSink;

    impl crate::EventSink for NoopSink {
        fn on_event(&self, _event: tether_types::CoreEvent) {}
    }
}
