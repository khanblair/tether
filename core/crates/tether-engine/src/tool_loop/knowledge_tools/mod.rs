//! Skill, memory, session search, and vault tools.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tether_providers::{BoxFuture, ToolSpec};
use tether_skills::{SkillSource, SkillsCatalog};
use tether_store::Store;

use super::registry::{ExecuteContext, PrepareContext, PreparedCall, ToolHandler, ToolOutput};
use crate::permissions::Effect;

mod execute;
mod support;

use support::{
    count_lines, object, string, valid_skill_name, valid_vault_path, validate_reference_content,
    validate_skill_content,
};

pub(super) const SKILLS_LIST: &str = "skills_list";
pub(super) const SKILL_VIEW: &str = "skill_view";
pub(super) const SKILL_MANAGE: &str = "skill_manage";
pub(super) const MEMORY: &str = "memory";
pub(super) const SESSION_SEARCH: &str = "session_search";

/// Dependencies for the knowledge tools in one session.
pub struct KnowledgeTools {
    catalog: SkillsCatalog,
    user_dir: PathBuf,
    memory_root: PathBuf,
    vault_root: PathBuf,
    store: Arc<Mutex<Store>>,
}

impl KnowledgeTools {
    /// Creates the knowledge handlers for one frozen session catalog.
    #[must_use]
    pub fn new(
        catalog: SkillsCatalog,
        user_dir: PathBuf,
        memory_root: PathBuf,
        vault_root: PathBuf,
        store: Arc<Mutex<Store>>,
    ) -> Self {
        Self {
            catalog,
            user_dir,
            memory_root,
            vault_root,
            store,
        }
    }
}

impl ToolHandler for KnowledgeTools {
    fn specs(&self) -> Vec<ToolSpec> {
        skill_specs()
            .into_iter()
            .chain(memory_specs())
            .chain(vault_specs())
            .collect()
    }

    fn prepare(
        &self,
        name: &str,
        args_json: &str,
        _context: &PrepareContext<'_>,
    ) -> Result<PreparedCall, String> {
        let value = object(args_json)?;
        let (target, effect) = match name {
            SKILLS_LIST => ("skills".into(), Effect::Read),
            SKILL_VIEW => Self::prepare_skill_view(&value)?,
            SKILL_MANAGE => self.prepare_skill_manage(&value)?,
            MEMORY => self.prepare_memory(&value)?,
            SESSION_SEARCH => Self::prepare_session_search(&value)?,
            "vault_search" | "vault_read" | "vault_links" | "vault_map" | "vault_orphans"
            | "vault_write" | "vault_append" => self.prepare_vault(name, &value)?,
            _ => return Err("unknown tool".into()),
        };
        Ok(PreparedCall {
            tool: name.into(),
            effect,
            target,
            executor: None,
            payload: value,
        })
    }

    fn execute<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> BoxFuture<'a, Result<ToolOutput, String>> {
        Box::pin(async move { self.execute_call(&call, &context) })
    }

    fn blocks_in_plan(&self, call: &PreparedCall) -> bool {
        call.tool == SKILL_MANAGE
    }
}

impl KnowledgeTools {
    fn prepare_skill_view(value: &Value) -> Result<(String, Effect), String> {
        let name = string(value, "name")?;
        Ok((
            value
                .get("file")
                .and_then(Value::as_str)
                .map_or_else(|| name.into(), str::to_owned),
            Effect::Read,
        ))
    }

    fn prepare_skill_manage(&self, value: &Value) -> Result<(String, Effect), String> {
        let action = string(value, "action")?;
        let name = string(value, "name")?;
        valid_skill_name(name)?;
        if self
            .catalog
            .skills
            .iter()
            .any(|skill| skill.name == name && skill.source != SkillSource::User)
        {
            return Err("a higher-precedence skill already has this name".into());
        }
        match action {
            "create" if value.get("file").is_none() => {
                validate_skill_content(name, string(value, "content")?)?;
            }
            "create" => validate_reference_content(string(value, "content")?)?,
            "patch" => self.validate_patch(value, name)?,
            "delete" => {}
            _ => return Err("action must be create, patch, or delete".into()),
        }
        let effect = if self.setting("skill_write_approval")? {
            Effect::Edit
        } else {
            Effect::Internal
        };
        let lines = if action == "create" {
            count_lines(string(value, "content")?)
        } else {
            2
        };
        let target = match action {
            "create" => format!("skill create {name} (+{lines} lines)"),
            "patch" => format!("skill patch {name} (+2 -1 lines)"),
            _ => format!("skill delete {name}"),
        };
        Ok((target, effect))
    }

    fn validate_patch(&self, value: &Value, name: &str) -> Result<(), String> {
        let current = self.read_user_file(name, value.get("file").and_then(Value::as_str))?;
        let patched = current.replacen(string(value, "old")?, string(value, "new")?, 1);
        if patched == current {
            return Err("old text was not found".into());
        }
        if value.get("file").is_none() {
            validate_skill_content(name, &patched)?;
        } else {
            validate_reference_content(&patched)?;
        }
        Ok(())
    }

    fn prepare_memory(&self, value: &Value) -> Result<(String, Effect), String> {
        let target = string(value, "target")?;
        if !matches!(target, "memory" | "user") {
            return Err("target must be memory or user".into());
        }
        let preview = value
            .get("entry")
            .or_else(|| value.get("new"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let effect = if self.setting("memory_write_approval")? {
            Effect::Edit
        } else {
            Effect::Internal
        };
        Ok((
            format!("memory: {}", support::cut_chars(preview, 60)),
            effect,
        ))
    }

    fn prepare_session_search(value: &Value) -> Result<(String, Effect), String> {
        let query = string(value, "query")?;
        if !(2..=200).contains(&query.chars().count()) {
            return Err("query must be 2 to 200 characters".into());
        }
        Ok((query.into(), Effect::Read))
    }

    fn prepare_vault(&self, name: &str, value: &Value) -> Result<(String, Effect), String> {
        if matches!(name, "vault_orphans" | "vault_search") {
            return Ok((name.into(), Effect::Read));
        }
        let path = value
            .get("path")
            .or_else(|| value.get("start"))
            .and_then(Value::as_str)
            .ok_or("vault path is required")?;
        valid_vault_path(path)?;
        if name == "vault_map"
            && value
                .get("token_budget")
                .and_then(Value::as_u64)
                .unwrap_or(1_000)
                > 4_000
        {
            return Err("token_budget must be at most 4000".into());
        }
        let write = matches!(name, "vault_write" | "vault_append");
        let effect = if !write {
            Effect::Read
        } else if self.setting("vault_write_approval")? {
            Effect::Edit
        } else {
            Effect::Internal
        };
        Ok((path.into(), effect))
    }

    fn read_user_file(&self, name: &str, file: Option<&str>) -> Result<String, String> {
        support::read_user_file(&self.user_dir, name, file)
    }
    fn setting(&self, key: &str) -> Result<bool, String> {
        support::setting(&self.store, key)
    }
}

fn skill_specs() -> Vec<ToolSpec> {
    vec![
        spec(
            SKILLS_LIST,
            "List available skills and their descriptions.",
            &json!({}),
            &[],
        ),
        spec(
            SKILL_VIEW,
            "Load a skill body or one of its reference files when its instructions are needed.",
            &json!({"name":"string","file":"string"}),
            &["name"],
        ),
        spec(
            SKILL_MANAGE,
            "Create, patch, or delete a user skill. create without file writes SKILL.md: content must be a full file starting with a --- frontmatter block containing name (the skill name, lowercase letters, digits, hyphens, at most 64 characters) and description (what it does and when to use it, at most 1,024 characters), followed by Markdown instructions. create with file adds a reference or script such as references/guide.md or scripts/run.sh; patch replaces exact old with new; delete removes the skill or the named file.",
            &json!({"action":"string","name":"string","content":"string","old":"string","new":"string","file":"string"}),
            &["action", "name"],
        ),
    ]
}

#[cfg(test)]
mod spec_tests;

fn memory_specs() -> Vec<ToolSpec> {
    vec![
        spec(
            MEMORY,
            "Add, patch, or remove one durable memory entry.",
            &json!({"target":"string","action":"string","entry":"string","old":"string","new":"string"}),
            &["target", "action"],
        ),
        spec(
            SESSION_SEARCH,
            "Search past session messages for an exact phrase without making a model call.",
            &json!({"query":"string","limit":"integer"}),
            &["query"],
        ),
    ]
}

fn vault_specs() -> Vec<ToolSpec> {
    vec![
        spec(
            "vault_search",
            "Search durable vault notes by text, tags, or description.",
            &json!({"query":"string","limit":"integer"}),
            &["query"],
        ),
        spec(
            "vault_read",
            "Read one Markdown note from the durable vault.",
            &json!({"path":"string"}),
            &["path"],
        ),
        spec(
            "vault_links",
            "Show outgoing links and backlinks for one vault note.",
            &json!({"path":"string"}),
            &["path"],
        ),
        spec(
            "vault_map",
            "Render a bounded neighborhood around a vault note.",
            &json!({"start":"string","depth":"integer","token_budget":"integer"}),
            &["start"],
        ),
        spec(
            "vault_orphans",
            "List vault notes with no incoming links.",
            &json!({}),
            &[],
        ),
        spec(
            "vault_write",
            "Replace one vault note after validating its description and links.",
            &json!({"path":"string","content":"string"}),
            &["path", "content"],
        ),
        spec(
            "vault_append",
            "Append text to one validated vault note.",
            &json!({"path":"string","content":"string"}),
            &["path", "content"],
        ),
    ]
}

fn spec(name: &str, description: &str, properties: &Value, required: &[&str]) -> ToolSpec {
    let fields = properties
        .as_object()
        .into_iter()
        .flat_map(|object| object.iter())
        .map(|(key, value)| {
            (
                key.clone(),
                json!({"type": if value == "integer" { "integer" } else { "string" }}),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    ToolSpec { name: name.into(), description: description.into(), input_schema_json: json!({"type":"object","properties":fields,"required":required,"additionalProperties":false}).to_string() }
}

/// Creates a staged record for a skill write.
#[must_use]
pub fn staged_skill_change(
    session_id: &tether_types::SessionId,
    tool_call_id: &tether_types::ToolCallId,
    call: &PreparedCall,
) -> tether_store::SkillChange {
    support::staged_skill_change(session_id, tool_call_id, call)
}
