use std::path::Path;

use serde_json::{Value, json};
use tether_vcs::Repo;

use super::{
    MAX_PATH, MAX_TEXT, optional_bool, optional_string, optional_usize, require_object, string,
    target_tool,
};
use crate::permissions::Effect as PermissionEffect;
use crate::tool_loop::PreparedCall;

pub(super) fn allowed_fields(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "git_status" => &[],
        "git_init" => &["default_branch"],
        "git_diff" => &["staged", "path", "from", "to"],
        "git_log" => &["limit", "path"],
        "git_branch" => &["name", "from", "switch"],
        "git_commit" => &["message", "paths"],
        "git_push" => &["branch", "force", "delete"],
        "pr_create" => &["title", "body", "base", "draft"],
        "pr_status" => &["number"],
        "github_repo_create" => &["name", "private", "description"],
        _ => return None,
    })
}

pub(super) fn prepare(
    name: &str,
    args_json: &str,
    default_branch: Option<&str>,
    github_owner: Option<&str>,
    root: &Path,
) -> Result<PreparedCall, String> {
    let value = require_object(name, args_json)?;
    match name {
        "git_status" => Ok(target_tool(
            name,
            PermissionEffect::Read,
            "status".into(),
            value,
        )),
        "git_init" => git_init(value),
        "git_diff" => diff(value),
        "git_log" => log(value),
        "git_branch" => branch(value),
        "git_commit" => commit(value),
        "git_push" => push(value, default_branch, root),
        "pr_create" => pr_create(value, default_branch, root),
        "pr_status" => pr_status(value),
        "github_repo_create" => github_repo_create(value, github_owner),
        _ => Err("unknown tool".into()),
    }
}

fn git_init(mut value: Value) -> Result<PreparedCall, String> {
    let branch = optional_string(&value, "default_branch")?
        .unwrap_or("main")
        .to_owned();
    bounded(&branch, 100, "default_branch")?;
    tether_vcs::validate_branch_name(&branch)
        .map_err(|_| "default_branch is invalid".to_owned())?;
    if value.get("default_branch").is_none() {
        value["default_branch"] = Value::String(branch.clone());
    }
    Ok(target_tool(
        "git_init",
        PermissionEffect::Edit,
        format!("initialize a Git repository on {branch}"),
        value,
    ))
}

fn diff(value: Value) -> Result<PreparedCall, String> {
    optional_bool(&value, "staged", false)?;
    let from = optional_string(&value, "from")?;
    let to = optional_string(&value, "to")?;
    if from.is_some() != to.is_some() {
        return Err("from and to must be supplied together".into());
    }
    if let Some(from) = from {
        bounded(from, 200, "from")?;
    }
    if let Some(to) = to {
        bounded(to, 200, "to")?;
    }
    let path = optional_string(&value, "path")?
        .map(validate_path)
        .transpose()?;
    let target = path.map_or_else(|| "diff".into(), |path| format!("diff {path}"));
    Ok(target_tool(
        "git_diff",
        PermissionEffect::Read,
        target,
        value,
    ))
}

fn log(value: Value) -> Result<PreparedCall, String> {
    let limit = optional_usize(&value, "limit", 10)?;
    if !(1..=50).contains(&limit) {
        return Err("limit must be between 1 and 50".into());
    }
    if let Some(path) = optional_string(&value, "path")? {
        validate_path(path)?;
    }
    Ok(target_tool(
        "git_log",
        PermissionEffect::Read,
        "log".into(),
        value,
    ))
}

fn branch(value: Value) -> Result<PreparedCall, String> {
    let name = optional_string(&value, "name")?.unwrap_or("");
    if name.len() > 100 || (name.is_empty() && value.get("name").is_some()) {
        return Err("name is invalid".into());
    }
    if let Some(from) = optional_string(&value, "from")? {
        bounded(from, 200, "from")?;
    }
    Ok(target_tool(
        "git_branch",
        PermissionEffect::Edit,
        if name.is_empty() {
            "create branch".into()
        } else {
            format!("create branch {name}")
        },
        value,
    ))
}

fn commit(value: Value) -> Result<PreparedCall, String> {
    let message = string(&value, "message")?;
    bounded(message, MAX_TEXT, "message")?;
    if message.trim().is_empty() {
        return Err("commit message is empty".into());
    }
    if let Some(paths) = value.get("paths") {
        let paths = paths.as_array().ok_or("paths must be an array")?;
        if paths.len() > 100 {
            return Err("paths must contain at most 100 items".into());
        }
        for path in paths {
            validate_path(path.as_str().ok_or("paths must contain strings")?)?;
        }
    }
    Ok(target_tool(
        "git_commit",
        PermissionEffect::Edit,
        "commit".into(),
        value,
    ))
}

/// Reads the branch that is checked out now, so an approved target names a real branch.
fn checked_out_branch(root: &Path) -> Result<String, String> {
    Repo::open(root)
        .map_err(|_| super::no_repository_error())?
        .current_branch()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "current branch is unavailable".to_owned())
}

/// Stores a resolved value in the call payload so execution uses exactly what was approved.
fn freeze(value: &mut Value, key: &str, resolved: &str) {
    if let Some(object) = value.as_object_mut() {
        object.insert(key.to_owned(), json!(resolved));
    }
}

fn push(
    mut value: Value,
    default_branch: Option<&str>,
    root: &Path,
) -> Result<PreparedCall, String> {
    if let Some(branch) = optional_string(&value, "branch")? {
        bounded(branch, 100, "branch")?;
    }
    let force = optional_bool(&value, "force", false)?;
    let delete = optional_bool(&value, "delete", false)?;
    if force && delete {
        return Err("force and delete cannot be combined".into());
    }
    let branch = match optional_string(&value, "branch")? {
        Some(branch) => branch.to_owned(),
        None => checked_out_branch(root)?,
    };
    freeze(&mut value, "branch", &branch);
    let target = if delete {
        format!("delete branch {branch} on origin")
    } else if force {
        format!("FORCE push {branch} to origin (rewrites history)")
    } else if default_branch == Some(branch.as_str()) {
        format!("push to the default branch {branch} on origin")
    } else {
        format!("push {branch} to origin")
    };
    Ok(target_tool(
        "git_push",
        if force {
            PermissionEffect::ForcePush
        } else {
            PermissionEffect::Push
        },
        target,
        value,
    ))
}

fn pr_create(
    mut value: Value,
    default_branch: Option<&str>,
    root: &Path,
) -> Result<PreparedCall, String> {
    let title = string(&value, "title")?.to_owned();
    bounded(&title, 200, "title")?;
    if title.trim().is_empty() {
        return Err("title must not be empty".into());
    }
    if let Some(body) = optional_string(&value, "body")? {
        bounded(body, MAX_TEXT, "body")?;
    }
    let base = optional_string(&value, "base")?
        .unwrap_or(default_branch.unwrap_or("main"))
        .to_owned();
    bounded(&base, 100, "base")?;
    let head = checked_out_branch(root)?;
    freeze(&mut value, "head", &head);
    let target = format!("open pull request \"{title}\" from {head} into {base}");
    Ok(target_tool(
        "pr_create",
        PermissionEffect::Push,
        target,
        value,
    ))
}

fn pr_status(value: Value) -> Result<PreparedCall, String> {
    if let Some(number) = value.get("number").and_then(Value::as_u64) {
        if number == 0 {
            return Err("number must be a positive integer".into());
        }
    } else if value.get("number").is_some() {
        return Err("number must be a positive integer".into());
    }
    Ok(target_tool(
        "pr_status",
        PermissionEffect::Network,
        "pull request status".into(),
        value,
    ))
}

fn github_repo_create(value: Value, owner: Option<&str>) -> Result<PreparedCall, String> {
    let name = string(&value, "name")?;
    validate_repository_name(name)?;
    let private = optional_bool(&value, "private", true)?;
    let visibility = if private { "private" } else { "public" };
    let repository = owner.map_or_else(|| name.to_owned(), |owner| format!("{owner}/{name}"));
    Ok(target_tool(
        "github_repo_create",
        PermissionEffect::Push,
        format!("create GitHub repository {repository} ({visibility})"),
        value,
    ))
}

fn validate_repository_name(name: &str) -> Result<(), String> {
    let length = name.chars().count();
    if !(1..=100).contains(&length)
        || matches!(name, "." | "..")
        || !name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(
            "repository name must be 1 to 100 letters, digits, '-', '_' or '.' and not '.' or '..'"
                .into(),
        );
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<String, String> {
    bounded(path, MAX_PATH, "path")?;
    if path.is_empty() || path.starts_with('/') || path.split('/').any(|part| part == "..") {
        return Err("path is invalid".into());
    }
    Ok(path.to_owned())
}

fn bounded(value: &str, max: usize, key: &str) -> Result<(), String> {
    if value.chars().count() > max {
        Err(format!("{key} is too long"))
    } else {
        Ok(())
    }
}
