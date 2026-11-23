//! Shared validation and storage helpers for knowledge tools.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tether_memory::MemoryError;
use tether_skills::parse_skill;
use tether_store::{SkillChange, Store};
use tether_types::{SessionId, ToolCallId};

use super::PreparedCall;

pub(super) fn object(args: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(args).map_err(|_| "invalid JSON arguments")?;
    if value.as_object().is_none() {
        return Err("tool arguments must be an object".into());
    }
    Ok(value)
}
pub(super) fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("{key} must be a non-empty string"))
}
pub(super) fn valid_skill_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        Err("invalid skill name".into())
    } else {
        Ok(())
    }
}
pub(super) fn validate_skill_content(name: &str, content: &str) -> Result<(), String> {
    if content.len() > 100 * 1024 {
        return Err("skill exceeds 100 KiB".into());
    }
    parse_skill(name, content)
        .map(|_| ())
        .map_err(|error| format!("invalid skill: {error:?}"))
}
pub(super) fn validate_reference_content(content: &str) -> Result<(), String> {
    if content.len() > 256 * 1024 {
        Err("reference file exceeds 256 KiB".into())
    } else {
        Ok(())
    }
}
pub(super) fn count_lines(content: &str) -> usize {
    content.lines().count()
}
pub(super) fn valid_vault_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || !Path::new(path)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        || Path::new(path)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        Err("invalid vault path".into())
    } else {
        Ok(())
    }
}
pub(super) fn cut_chars(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        value.into()
    } else {
        value.chars().take(limit).collect::<String>() + "…"
    }
}

pub(super) fn user_target(root: &Path, name: &str, file: Option<&str>) -> Result<PathBuf, String> {
    valid_skill_name(name)?;
    if fs::symlink_metadata(root).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err("symlinked skills directory".into());
    }
    let relative = file.map_or_else(
        || PathBuf::from(name).join("SKILL.md"),
        |file| PathBuf::from(name).join(file),
    );
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("invalid skill path".into());
    }
    if file.is_some()
        && !matches!(relative.components().nth(1), Some(Component::Normal(folder)) if ["references", "templates", "scripts", "assets"].contains(&folder.to_string_lossy().as_ref()))
    {
        return Err("reference file must be under an allowed folder".into());
    }
    let mut path = root.to_owned();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err("invalid skill path".into());
        };
        path.push(part);
        if fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err("symlinked skill path".into());
        }
    }
    Ok(path)
}
pub(super) fn read_user_file(
    root: &Path,
    name: &str,
    file: Option<&str>,
) -> Result<String, String> {
    fs::read_to_string(user_target(root, name, file)?).map_err(|error| error.to_string())
}
pub(super) fn write_user_file(path: &Path, content: &str) -> Result<(), String> {
    let limit = if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
        100 * 1024
    } else {
        256 * 1024
    };
    if content.len() > limit {
        return Err(if limit == 100 * 1024 {
            "skill exceeds 100 KiB".into()
        } else {
            "reference file exceeds 256 KiB".into()
        });
    }
    fs::create_dir_all(path.parent().ok_or("invalid skill path")?)
        .map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}
pub(super) fn list_skills(catalog: &tether_skills::SkillsCatalog) -> String {
    catalog
        .listed()
        .into_iter()
        .map(|skill| {
            format!(
                "{} [{}]: {}",
                skill.name,
                skill.source.as_str(),
                skill.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
pub(super) fn skill_error(error: &tether_skills::SkillError) -> String {
    format!("skill error: {error:?}")
}
pub(super) fn memory_error(error: &MemoryError) -> String {
    format!("memory error: {error:?}")
}
pub(super) fn vault_error(error: &tether_memory::vault::VaultError) -> String {
    format!("vault error: {error:?}")
}
pub(super) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}
pub(super) fn setting(store: &Arc<Mutex<Store>>, key: &str) -> Result<bool, String> {
    let store = store.lock().map_err(|_| "store lock poisoned")?;
    match key {
        "skill_write_approval" => store.skill_write_approval(),
        "memory_write_approval" => store.memory_write_approval(),
        "vault_write_approval" => store.vault_write_approval(),
        _ => Ok(true),
    }
    .map_err(|_| "could not read tool setting".into())
}
pub(super) fn staged_skill_change(
    session_id: &SessionId,
    tool_call_id: &ToolCallId,
    call: &PreparedCall,
) -> SkillChange {
    let name = call
        .payload
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let action = call
        .payload
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let diff = match action {
        "create" => format!(
            "+{}",
            call.payload
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
        ),
        "patch" => format!(
            "-{}\n+{}",
            call.payload
                .get("old")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            call.payload
                .get("new")
                .and_then(Value::as_str)
                .unwrap_or_default()
        ),
        _ => format!("- skill {name}"),
    };
    SkillChange {
        id: format!("{session_id}-{tool_call_id}"),
        session_id: session_id.clone(),
        tool_call_id: tool_call_id.clone(),
        action: action.into(),
        name: name.into(),
        diff,
        created_at: now_ms(),
    }
}
