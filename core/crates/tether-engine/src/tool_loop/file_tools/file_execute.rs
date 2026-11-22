//! Execution half of the guarded file handler.

use std::path::PathBuf;

use serde_json::Value;
use tether_providers::BoxFuture;
use tether_tools::{EditSpec, EditTools, ReadTools};

use super::super::registry::{ExecuteContext, PreparedCall, ToolOutput};
use super::{FileTools, optional_string, optional_usize, string};

pub(super) fn execute<'a>(
    tools: &'a FileTools,
    call: PreparedCall,
    mut context: ExecuteContext<'a>,
) -> BoxFuture<'a, Result<ToolOutput, String>> {
    Box::pin(async move {
        if context.cancel.is_cancelled() {
            return Err("tool cancelled before execution".into());
        }
        let result = match call.tool.as_str() {
            "read_file" => read(&tools.read, &call.payload),
            "list_dir" => list(&tools.read, &call.payload),
            "glob" => glob(&tools.read, &call.payload),
            "grep" => grep(&tools.read, &call.payload),
            "write_file" => write(&tools.edit, &mut context, &call.payload),
            "edit_file" => edit(&tools.edit, &mut context, &call.payload),
            "multi_edit" => multi_edit(&tools.edit, &mut context, &call.payload),
            "move_file" => move_file(&tools.edit, &mut context, &call.payload),
            "delete_file" => delete(&tools.edit, &mut context, &call.payload),
            _ => Err("unknown tool".into()),
        }?;
        Ok(ToolOutput { text: result })
    })
}

fn read(read: &ReadTools, value: &Value) -> Result<String, String> {
    let path = string(value, "path")?;
    let lines = optional_usize(value, "start_line")?.zip(optional_usize(value, "end_line")?);
    let file = read
        .read_file(path, lines)
        .map_err(|error| error.to_string())?;
    Ok(format!("{path}:{}\n{}", file.start_line, file.text))
}

fn list(read: &ReadTools, value: &Value) -> Result<String, String> {
    let path = optional_string(value, "path")?.unwrap_or_default();
    let entries = read.list_dir(path).map_err(|error| error.to_string())?;
    Ok(entries
        .into_iter()
        .map(|entry| {
            format!(
                "{}{}",
                entry.path.display(),
                if entry.is_dir { "/" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn glob(read: &ReadTools, value: &Value) -> Result<String, String> {
    Ok(read
        .glob(string(value, "pattern")?)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|file| file.path.display().to_string())
        .collect::<Vec<_>>()
        .join("\n"))
}

fn grep(read: &ReadTools, value: &Value) -> Result<String, String> {
    let limit = optional_usize(value, "limit")?.unwrap_or(100);
    Ok(read
        .grep(string(value, "pattern")?, limit)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|hit| format!("{}:{}:{}", hit.path.display(), hit.line, hit.text))
        .collect::<Vec<_>>()
        .join("\n"))
}

fn checked_path(context: &ExecuteContext<'_>, path: &str) -> Result<(), String> {
    context
        .guard
        .resolve(path)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn before(context: &mut ExecuteContext<'_>, paths: &[PathBuf]) -> Result<(), String> {
    (context.before_edit)(paths)
}

fn write(
    edit: &EditTools,
    context: &mut ExecuteContext<'_>,
    value: &Value,
) -> Result<String, String> {
    let path = string(value, "path")?;
    checked_path(context, path)?;
    before(context, &[PathBuf::from(path)])?;
    edit.write(path, string(value, "text")?)
        .map_err(|error| format!("{error:?}"))?;
    Ok(format!("wrote {path}"))
}

fn edit(
    edit: &EditTools,
    context: &mut ExecuteContext<'_>,
    value: &Value,
) -> Result<String, String> {
    let path = string(value, "path")?;
    checked_path(context, path)?;
    before(context, &[PathBuf::from(path)])?;
    edit.edit(
        path,
        &EditSpec {
            old: string(value, "old")?.into(),
            new: string(value, "new")?.into(),
        },
    )
    .map_err(|error| format!("{error:?}"))?;
    Ok(format!("edited {path}"))
}

fn multi_edit(
    edit: &EditTools,
    context: &mut ExecuteContext<'_>,
    value: &Value,
) -> Result<String, String> {
    let path = string(value, "path")?;
    let values = value
        .get("edits")
        .and_then(Value::as_array)
        .ok_or("edits must be an array")?;
    let specs = values
        .iter()
        .map(|value| {
            Ok(EditSpec {
                old: string(value, "old")?.into(),
                new: string(value, "new")?.into(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    checked_path(context, path)?;
    before(context, &[PathBuf::from(path)])?;
    edit.multi_edit(path, &specs)
        .map_err(|error| format!("{error:?}"))?;
    Ok(format!("edited {path}"))
}

fn move_file(
    edit: &EditTools,
    context: &mut ExecuteContext<'_>,
    value: &Value,
) -> Result<String, String> {
    let from = string(value, "from")?;
    let to = string(value, "to")?;
    checked_path(context, from)?;
    checked_path(context, to)?;
    before(context, &[PathBuf::from(from), PathBuf::from(to)])?;
    edit.move_file(from, to)
        .map_err(|error| format!("{error:?}"))?;
    Ok(format!("moved {from} to {to}"))
}

fn delete(
    edit: &EditTools,
    context: &mut ExecuteContext<'_>,
    value: &Value,
) -> Result<String, String> {
    let path = string(value, "path")?;
    checked_path(context, path)?;
    before(context, &[PathBuf::from(path)])?;
    edit.delete(path).map_err(|error| format!("{error:?}"))?;
    Ok(format!("deleted {path}"))
}
