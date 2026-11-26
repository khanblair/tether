//! The guarded file tool handler.

use serde_json::{Value, json};
use tether_providers::{BoxFuture, ToolSpec};
use tether_tools::{EditTools, ReadTools};

use super::registry::{ExecuteContext, PrepareContext, PreparedCall, ToolHandler, ToolOutput};
use crate::path_utils::lexical_path;
use crate::permissions::Effect;

mod file_execute;

const FILE_EDIT_TOOLS: [&str; 5] = [
    "write_file",
    "edit_file",
    "multi_edit",
    "move_file",
    "delete_file",
];

/// Owns the read and edit tools that operate inside one workspace.
pub(super) struct FileTools {
    read: ReadTools,
    edit: EditTools,
}

impl FileTools {
    /// Creates the file handler for one guarded workspace.
    #[must_use]
    pub(super) fn new(guard: tether_workspace::PathGuard) -> Self {
        Self {
            read: ReadTools::new(guard.clone()),
            edit: EditTools::new(guard),
        }
    }
}

impl ToolHandler for FileTools {
    fn specs(&self) -> Vec<ToolSpec> {
        read_specs().into_iter().chain(edit_specs()).collect()
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
        let allowed = allowed_fields(name).ok_or("unknown tool")?;
        if object.keys().any(|key| !allowed.contains(&key.as_str())) {
            return Err("unknown tool argument".into());
        }
        let mut payload = value;
        let target = match name {
            "read_file" => {
                let path = normalized_path(context, string(&payload, "path")?)?;
                line_range(&payload)?;
                set_string(&mut payload, "path", &path);
                path
            }
            "list_dir" => prepare_directory(&mut payload, context)?,
            "glob" | "grep" => {
                let _ = string(&payload, "pattern")?;
                optional_usize(&payload, "limit")?;
                string(&payload, "pattern")?.to_owned()
            }
            "write_file" | "edit_file" | "multi_edit" | "delete_file" => {
                let path = normalized_path(context, string(&payload, "path")?)?;
                validate_edit_payload(name, &payload)?;
                set_string(&mut payload, "path", &path);
                path
            }
            "move_file" => {
                let from = normalized_path(context, string(&payload, "from")?)?;
                let to = normalized_path(context, string(&payload, "to")?)?;
                set_string(&mut payload, "from", &from);
                set_string(&mut payload, "to", &to);
                format!("{from} -> {to}")
            }
            _ => return Err("unknown tool".into()),
        };
        let effect = if FILE_EDIT_TOOLS.contains(&name) {
            Effect::Edit
        } else {
            Effect::Read
        };
        Ok(PreparedCall {
            tool: name.to_owned(),
            effect,
            target,
            executor: None,
            payload,
        })
    }

    fn execute<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> BoxFuture<'a, Result<ToolOutput, String>> {
        file_execute::execute(self, call, context)
    }
}

fn schema(fields: &Value, required: &[&str]) -> Value {
    let properties = fields
        .as_object()
        .map(|fields| {
            fields
                .iter()
                .map(|(key, kind)| (key.clone(), schema_field(key, kind)))
                .collect::<serde_json::Map<_, _>>()
        })
        .unwrap_or_default();
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

fn read_specs() -> Vec<ToolSpec> {
    make_specs([
        (
            "read_file",
            "Read UTF-8 file text or an inclusive line range",
            json!({"path": "string", "start_line": "integer", "end_line": "integer"}),
            &["path"][..],
        ),
        (
            "list_dir",
            "List direct children of a workspace folder",
            json!({"path": "string"}),
            &[][..],
        ),
        (
            "glob",
            "Find workspace files with a simple star pattern",
            json!({"pattern": "string"}),
            &["pattern"][..],
        ),
        (
            "grep",
            "Find regex matches in indexed workspace files",
            json!({"pattern": "string", "limit": "integer"}),
            &["pattern"][..],
        ),
    ])
}

fn edit_specs() -> Vec<ToolSpec> {
    make_specs([
        (
            "write_file",
            "Write UTF-8 text to a workspace file",
            json!({"path": "string", "text": "string"}),
            &["path", "text"][..],
        ),
        (
            "edit_file",
            "Replace one exact string in a workspace file",
            json!({"path": "string", "old": "string", "new": "string"}),
            &["path", "old", "new"][..],
        ),
        (
            "multi_edit",
            "Apply several exact replacements atomically",
            json!({"path": "string", "edits": "array"}),
            &["path", "edits"][..],
        ),
        (
            "move_file",
            "Move a workspace file to an unused path",
            json!({"from": "string", "to": "string"}),
            &["from", "to"][..],
        ),
        (
            "delete_file",
            "Delete one workspace file",
            json!({"path": "string"}),
            &["path"][..],
        ),
    ])
}

fn make_specs<const N: usize>(specs: [(&str, &str, Value, &[&str]); N]) -> Vec<ToolSpec> {
    specs
        .into_iter()
        .map(|(name, description, fields, required)| ToolSpec {
            name: name.into(),
            description: description.into(),
            input_schema_json: schema(&fields, required).to_string(),
        })
        .collect()
}

fn schema_field(key: &str, kind: &Value) -> Value {
    if key == "edits" {
        json!({"type":"array","minItems":1,"maxItems":100,"items":{"type":"object","properties":{"old":{"type":"string"},"new":{"type":"string"}},"required":["old","new"],"additionalProperties":false}})
    } else {
        json!({"type": kind})
    }
}

fn prepare_directory(payload: &mut Value, context: &PrepareContext<'_>) -> Result<String, String> {
    let raw = optional_string(payload, "path")?.unwrap_or_default();
    let lexical = lexical_path(raw)?;
    if lexical.as_os_str().is_empty() {
        set_string(payload, "path", "");
        return Ok(".".to_owned());
    }
    let path = normalized_path(context, raw)?;
    set_string(payload, "path", &path);
    Ok(path)
}

fn allowed_fields(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "read_file" => &["path", "start_line", "end_line"],
        "list_dir" | "delete_file" => &["path"],
        "glob" => &["pattern"],
        "grep" => &["pattern", "limit"],
        "write_file" => &["path", "text"],
        "edit_file" => &["path", "old", "new"],
        "multi_edit" => &["path", "edits"],
        "move_file" => &["from", "to"],
        _ => return None,
    })
}

fn normalized_path(context: &PrepareContext<'_>, raw: &str) -> Result<String, String> {
    let path = lexical_path(raw)?;
    if path.as_os_str().is_empty() {
        return Err("path is empty".into());
    }
    let resolved = context
        .guard
        .resolve(&path)
        .map_err(|_| "path leaves the workspace".to_owned())?;
    resolved
        .strip_prefix(context.guard.root())
        .map_err(|_| "path leaves the workspace".to_owned())
        .and_then(|relative| {
            let value = relative.to_string_lossy().replace('\\', "/");
            (!value.is_empty())
                .then_some(value)
                .ok_or_else(|| "path is empty".to_owned())
        })
}

fn line_range(value: &Value) -> Result<(), String> {
    let start = optional_usize(value, "start_line")?;
    let end = optional_usize(value, "end_line")?;
    if start.is_some() != end.is_some() {
        return Err("start_line and end_line must be supplied together".into());
    }
    Ok(())
}

fn validate_edit_payload(name: &str, value: &Value) -> Result<(), String> {
    if name != "multi_edit" {
        if name == "write_file" {
            let _ = string(value, "text")?;
        } else if name == "edit_file" {
            let _ = string(value, "old")?;
            let _ = string(value, "new")?;
        }
        return Ok(());
    }
    let edits = value
        .get("edits")
        .and_then(Value::as_array)
        .ok_or("edits must be an array")?;
    if edits.is_empty() || edits.len() > 100 {
        return Err("edits must contain 1 to 100 replacements".into());
    }
    for edit in edits {
        let object = edit.as_object().ok_or("each edit must be an object")?;
        if object.len() != 2 || !object.contains_key("old") || !object.contains_key("new") {
            return Err("each edit needs only old and new".into());
        }
        let _ = string(edit, "old")?;
        let _ = string(edit, "new")?;
    }
    Ok(())
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn optional_string<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    value.get(key).map_or(Ok(None), |item| {
        item.as_str()
            .map(Some)
            .ok_or_else(|| format!("{key} must be a string"))
    })
}

fn optional_usize(value: &Value, key: &str) -> Result<Option<usize>, String> {
    value.get(key).map_or(Ok(None), |value| {
        value
            .as_u64()
            .and_then(|number| usize::try_from(number).ok())
            .map(Some)
            .ok_or_else(|| format!("{key} must be a nonnegative integer"))
    })
}

fn set_string(value: &mut Value, key: &str, text: &str) {
    if let Some(object) = value.as_object_mut() {
        object.insert(key.to_owned(), Value::String(text.to_owned()));
    }
}
