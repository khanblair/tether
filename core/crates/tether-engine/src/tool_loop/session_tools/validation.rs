//! Argument validation for session tools.

use serde_json::{Value, json};
use tether_types::{TodoItem, TodoStatus};

use super::{ASK_TOOL, PLAN_TOOL, TODO_TOOL};
use crate::tool_loop::PrepareContext;

pub(super) fn schema(name: &str) -> Value {
    match name {
        TODO_TOOL => {
            json!({"type":"object","properties":{"todos":{"type":"array","minItems":1,"maxItems":30,"items":{"type":"object","properties":{"text":{"type":"string","minLength":1,"maxLength":200},"status":{"type":"string","enum":["pending","in_progress","completed"]}},"required":["text","status"],"additionalProperties":false}}},"required":["todos"],"additionalProperties":false})
        }
        ASK_TOOL => {
            json!({"type":"object","properties":{"question":{"type":"string","minLength":1,"maxLength":500},"options":{"type":"array","maxItems":6,"items":{"type":"string","minLength":1,"maxLength":80}},"allow_free_text":{"type":"boolean"}},"required":["question"],"additionalProperties":false})
        }
        PLAN_TOOL => {
            json!({"type":"object","properties":{"summary":{"type":"string","minLength":1,"maxLength":400},"steps":{"type":"array","minItems":1,"maxItems":30,"items":{"type":"string","minLength":1,"maxLength":300}},"files":{"type":"array","maxItems":50,"items":{"type":"string"}},"commands":{"type":"array","maxItems":20,"items":{"type":"string"}},"open_questions":{"type":"array","maxItems":10,"items":{"type":"string"}}},"required":["summary","steps"],"additionalProperties":false})
        }
        _ => Value::Null,
    }
}

pub(super) fn object(args: &str) -> Result<serde_json::Map<String, Value>, String> {
    serde_json::from_str::<Value>(args)
        .map_err(|_| "invalid JSON arguments".to_owned())?
        .as_object()
        .cloned()
        .ok_or_else(|| "tool arguments must be an object".into())
}

fn check_fields(object: &serde_json::Map<String, Value>, allowed: &[&str]) -> Result<(), String> {
    object
        .keys()
        .find(|key| !allowed.contains(&key.as_str()))
        .map_or(Ok(()), |key| Err(format!("unknown tool argument: {key}")))
}

pub(super) fn required_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

pub(super) fn strings(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, String> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an array"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("each {key} item must be a string"))
        })
        .collect()
}

fn optional_strings(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, String> {
    object
        .get(key)
        .map_or(Ok(Vec::new()), |_| strings(object, key))
}

fn bounded(value: &str, key: &str, min: usize, max: usize) -> Result<(), String> {
    if (min..=max).contains(&value.chars().count()) {
        Ok(())
    } else {
        Err(format!("{key} must contain {min} to {max} characters"))
    }
}

fn bounded_list(values: &[String], key: &str, min: usize, max: usize) -> Result<(), String> {
    if (min..=max).contains(&values.len()) {
        Ok(())
    } else {
        Err(format!("{key} must contain {min} to {max} items"))
    }
}

pub(super) fn prepare_todos(
    object: &serde_json::Map<String, Value>,
) -> Result<(String, Value), String> {
    check_fields(object, &["todos"])?;
    let todos = object
        .get("todos")
        .and_then(Value::as_array)
        .ok_or("todos must be an array")?;
    if !(1..=30).contains(&todos.len()) {
        return Err("todos must contain 1 to 30 items".into());
    }
    let mut items = Vec::with_capacity(todos.len());
    let mut active = 0;
    for todo in todos {
        let item = todo.as_object().ok_or("each todo must be an object")?;
        if item.len() != 2 || !item.contains_key("text") || !item.contains_key("status") {
            return Err("each todo needs only text and status".into());
        }
        let text = required_string(item, "text")?;
        bounded(text, "todo text", 1, 200)?;
        let status = required_string(item, "status")?
            .parse::<TodoStatus>()
            .map_err(|_| "status must be pending, in_progress, or completed")?;
        if status == TodoStatus::InProgress {
            active += 1;
        }
        items.push(TodoItem {
            text: text.into(),
            status,
        });
    }
    if active > 1 {
        return Err("todos may contain at most one in_progress item".into());
    }
    let payload = json!(
        items
            .iter()
            .map(|item| json!({"text": item.text, "status": item.status.as_str()}))
            .collect::<Vec<_>>()
    );
    Ok((
        format!("checklist ({} items)", items.len()),
        json!({"todos": payload}),
    ))
}

pub(super) fn prepare_question(
    object: &serde_json::Map<String, Value>,
) -> Result<(String, Value), String> {
    check_fields(object, &["question", "options", "allow_free_text"])?;
    let question = required_string(object, "question")?;
    bounded(question, "question", 1, 500)?;
    let options = object
        .get("options")
        .map_or(Ok(Vec::new()), |_| strings(object, "options"))?;
    if options.len() > 6 {
        return Err("options must contain 0 to 6 items".into());
    }
    for option in &options {
        bounded(option, "option", 1, 80)?;
    }
    let allow = match object.get("allow_free_text") {
        Some(value) => value.as_bool().ok_or("allow_free_text must be a boolean")?,
        None => options.is_empty(),
    };
    Ok((
        question.into(),
        json!({"question":question,"options":options,"allow_free_text":allow}),
    ))
}

pub(super) fn prepare_plan(
    object: &serde_json::Map<String, Value>,
    context: &PrepareContext<'_>,
) -> Result<(String, Value), String> {
    check_fields(
        object,
        &["summary", "steps", "files", "commands", "open_questions"],
    )?;
    let summary = required_string(object, "summary")?;
    bounded(summary, "summary", 1, 400)?;
    let steps = strings(object, "steps")?;
    bounded_list(&steps, "steps", 1, 30)?;
    for step in &steps {
        bounded(step, "step", 1, 300)?;
    }
    let files = optional_strings(object, "files")?;
    if files.len() > 50 {
        return Err("files must contain 0 to 50 items".into());
    }
    let files = files
        .into_iter()
        .map(|file| guarded_path(context, &file))
        .collect::<Result<Vec<_>, _>>()?;
    let commands = optional_strings(object, "commands")?;
    if commands.len() > 20 {
        return Err("commands must contain 0 to 20 items".into());
    }
    let questions = optional_strings(object, "open_questions")?;
    if questions.len() > 10 {
        return Err("open_questions must contain 0 to 10 items".into());
    }
    Ok((
        "plan".into(),
        json!({"summary":summary,"steps":steps,"files":files,"commands":commands,"open_questions":questions}),
    ))
}

fn guarded_path(context: &PrepareContext<'_>, path: &str) -> Result<String, String> {
    if path.is_empty() || std::path::Path::new(path).is_absolute() {
        return Err("files must contain workspace-relative paths".into());
    }
    let resolved = context
        .guard
        .resolve(path)
        .map_err(|_| "files must contain workspace-relative paths".to_owned())?;
    resolved
        .strip_prefix(context.guard.root())
        .map_err(|_| "files must contain workspace-relative paths".into())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
}
