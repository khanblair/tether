//! Tool contracts and argument checks at the engine boundary.

use serde_json::{Value, json};
use std::path::PathBuf;
use tether_providers::ToolSpec;
use tether_tools::EditSpec;

use super::ToolLoop;

#[expect(
    clippy::too_many_lines,
    reason = "the tool contract table keeps fields together"
)]
pub(super) fn specs() -> Vec<ToolSpec> {
    [
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
    ]
    .into_iter()
    .map(|(name, description, fields, required)| ToolSpec {
        name: name.into(),
        description: description.into(),
        input_schema_json: schema(&fields, required).to_string(),
    })
    .collect()
}

fn schema(fields: &Value, required: &[&str]) -> Value {
    let properties = fields
        .as_object()
        .expect("static schema object")
        .iter()
        .map(|(key, kind)| {
            let field = if key == "edits" {
                json!({
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 100,
                    "items": {
                        "type": "object",
                        "properties": {"old": {"type": "string"}, "new": {"type": "string"}},
                        "required": ["old", "new"],
                        "additionalProperties": false
                    }
                })
            } else {
                json!({"type": kind})
            };
            (key.clone(), field)
        })
        .collect::<serde_json::Map<_, _>>();
    json!({"type": "object", "properties": properties, "required": required, "additionalProperties": false})
}

#[expect(
    clippy::too_many_lines,
    clippy::excessive_nesting,
    reason = "each tool arm checks arguments before dispatch"
)]
pub(super) fn run(
    tools: &ToolLoop,
    name: &str,
    args: &str,
    approve: &(dyn Fn(&str, &str) -> bool + Send + Sync),
    before_edit: &mut dyn FnMut(&[PathBuf]) -> Result<(), String>,
) -> Result<String, String> {
    let value: Value = serde_json::from_str(args).map_err(|_| "invalid JSON arguments")?;
    let map = value
        .as_object()
        .ok_or("tool arguments must be an object")?;
    let allowed: &[&str] = match name {
        "read_file" => &["path", "start_line", "end_line"],
        "list_dir" | "delete_file" => &["path"],
        "glob" => &["pattern"],
        "grep" => &["pattern", "limit"],
        "write_file" => &["path", "text"],
        "edit_file" => &["path", "old", "new"],
        "multi_edit" => &["path", "edits"],
        "move_file" => &["from", "to"],
        _ => return Err("unknown tool".into()),
    };
    if map.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("unknown tool argument".into());
    }
    match name {
        "read_file" => {
            let path = string(&value, "path")?;
            let start = optional_usize(&value, "start_line")?;
            let end = optional_usize(&value, "end_line")?;
            if start.is_some() != end.is_some() {
                return Err("start_line and end_line must be supplied together".into());
            }
            let file = tools
                .read
                .read_file(path, start.zip(end))
                .map_err(|error| error.to_string())?;
            Ok(format!("{}:{}\n{}", path, file.start_line, file.text))
        }
        "list_dir" => {
            let path = optional_string(&value, "path")?.unwrap_or("");
            let entries = tools
                .read
                .list_dir(path)
                .map_err(|error| error.to_string())?;
            Ok(entries
                .into_iter()
                .map(|entry| {
                    let suffix = if entry.is_dir { "/" } else { "" };
                    format!("{}{suffix}", entry.path.display())
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "glob" => Ok(tools
            .read
            .glob(string(&value, "pattern")?)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|file| file.path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")),
        "grep" => {
            let limit = optional_usize(&value, "limit")?.unwrap_or(100);
            let hits = tools
                .read
                .grep(string(&value, "pattern")?, limit)
                .map_err(|error| error.to_string())?;
            Ok(hits
                .into_iter()
                .map(|hit| format!("{}:{}:{}", hit.path.display(), hit.line, hit.text))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "write_file" => {
            let path = string(&value, "path")?;
            require_approval(approve, name, path)?;
            before_edit(&[PathBuf::from(path)])?;
            tools
                .edit
                .write(path, string(&value, "text")?)
                .map_err(|error| format!("{error:?}"))?;
            Ok(format!("wrote {path}"))
        }
        "edit_file" => {
            let path = string(&value, "path")?;
            require_approval(approve, name, path)?;
            before_edit(&[PathBuf::from(path)])?;
            tools
                .edit
                .edit(
                    path,
                    &EditSpec {
                        old: string(&value, "old")?.into(),
                        new: string(&value, "new")?.into(),
                    },
                )
                .map_err(|error| format!("{error:?}"))?;
            Ok(format!("edited {path}"))
        }
        "multi_edit" => {
            let path = string(&value, "path")?;
            let edits = value
                .get("edits")
                .and_then(Value::as_array)
                .ok_or("edits must be an array")?;
            if edits.is_empty() || edits.len() > 100 {
                return Err("edits must contain 1 to 100 replacements".into());
            }
            let specs = edits
                .iter()
                .map(|edit| {
                    let object = edit.as_object().ok_or("each edit must be an object")?;
                    if object.len() != 2
                        || !object.contains_key("old")
                        || !object.contains_key("new")
                    {
                        return Err("each edit needs only old and new".into());
                    }
                    Ok(EditSpec {
                        old: string(edit, "old")?.into(),
                        new: string(edit, "new")?.into(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            require_approval(approve, name, path)?;
            before_edit(&[PathBuf::from(path)])?;
            tools
                .edit
                .multi_edit(path, &specs)
                .map_err(|error| format!("{error:?}"))?;
            Ok(format!("edited {path}"))
        }
        "move_file" => {
            let from = string(&value, "from")?;
            let to = string(&value, "to")?;
            require_approval(approve, name, &format!("{from} -> {to}"))?;
            before_edit(&[PathBuf::from(from), PathBuf::from(to)])?;
            tools
                .edit
                .move_file(from, to)
                .map_err(|error| format!("{error:?}"))?;
            Ok(format!("moved {from} to {to}"))
        }
        "delete_file" => {
            let path = string(&value, "path")?;
            require_approval(approve, name, path)?;
            before_edit(&[PathBuf::from(path)])?;
            tools
                .edit
                .delete(path)
                .map_err(|error| format!("{error:?}"))?;
            Ok(format!("deleted {path}"))
        }
        _ => Err("unknown tool".into()),
    }
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn optional_string<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    match value.get(key) {
        None => Ok(None),
        Some(_) => string(value, key).map(Some),
    }
}

fn optional_usize(value: &Value, key: &str) -> Result<Option<usize>, String> {
    match value.get(key) {
        None => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|number| usize::try_from(number).ok())
            .map(Some)
            .ok_or_else(|| format!("{key} must be a nonnegative integer")),
    }
}

fn require_approval(
    approve: &(dyn Fn(&str, &str) -> bool + Send + Sync),
    name: &str,
    target: &str,
) -> Result<(), String> {
    if approve(name, target) {
        Ok(())
    } else {
        Err("tool approval denied".into())
    }
}
