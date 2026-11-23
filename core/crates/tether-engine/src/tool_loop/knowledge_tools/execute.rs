//! Knowledge tool execution.

use std::fs;

use serde_json::Value;
use tether_memory::vault::VaultStore;
use tether_memory::{MemoryAction, MemoryStore, MemoryTarget};
use tether_types::SessionId;

use super::super::registry::{ExecuteContext, PreparedCall, ToolOutput};
use super::support::{
    cut_chars, list_skills, memory_error, now_ms, read_user_file, skill_error, string, user_target,
    vault_error, write_user_file,
};
use super::{KnowledgeTools, MEMORY, SESSION_SEARCH, SKILL_MANAGE, SKILL_VIEW, SKILLS_LIST};

impl KnowledgeTools {
    pub(super) fn execute_call(
        &self,
        call: &PreparedCall,
        context: &ExecuteContext<'_>,
    ) -> Result<ToolOutput, String> {
        match call.tool.as_str() {
            SKILLS_LIST => Ok(ToolOutput {
                text: list_skills(&self.catalog),
            }),
            SKILL_VIEW => self.execute_skill_view(call),
            SKILL_MANAGE => self.execute_skill_manage(call),
            MEMORY => self.execute_memory(call),
            SESSION_SEARCH => self.execute_session_search(call, context.session_id),
            name if name.starts_with("vault_") => self.execute_vault(name, call),
            _ => Err("unknown tool".into()),
        }
    }

    fn execute_skill_view(&self, call: &PreparedCall) -> Result<ToolOutput, String> {
        let name = string(&call.payload, "name")?;
        let text = call
            .payload
            .get("file")
            .and_then(Value::as_str)
            .map_or_else(
                || {
                    self.catalog
                        .view(name)
                        .map(|view| view.document.body)
                        .map_err(|error| skill_error(&error))
                },
                |file| {
                    self.catalog
                        .read_file(name, file)
                        .map_err(|error| skill_error(&error))
                },
            )?;
        if self
            .catalog
            .listed()
            .iter()
            .any(|skill| skill.name == name && skill.source == tether_skills::SkillSource::User)
        {
            self.store
                .lock()
                .map_err(|_| "store lock poisoned")?
                .record_skill_view(name, now_ms())
                .map_err(|_| "could not record skill usage")?;
        }
        Ok(ToolOutput { text })
    }

    fn execute_skill_manage(&self, call: &PreparedCall) -> Result<ToolOutput, String> {
        let name = string(&call.payload, "name")?;
        let action = string(&call.payload, "action")?;
        let file = call.payload.get("file").and_then(Value::as_str);
        let target = user_target(&self.user_dir, name, file)?;
        if action == "delete" {
            Self::delete_skill_file(&target, file)?;
        } else if action == "create" {
            let content = string(&call.payload, "content")?;
            write_user_file(&target, content)?;
        } else {
            Self::patch_skill_file(&self.user_dir, &target, name, file, &call.payload)?;
        }
        Ok(ToolOutput {
            text: format!("skill {action} completed: {name}"),
        })
    }

    fn delete_skill_file(target: &std::path::Path, file: Option<&str>) -> Result<(), String> {
        let result = file.map_or_else(|| fs::remove_dir_all(target), |_| fs::remove_file(target));
        result.map_err(|error| error.to_string())
    }

    fn patch_skill_file(
        user_dir: &std::path::Path,
        target: &std::path::Path,
        name: &str,
        file: Option<&str>,
        payload: &Value,
    ) -> Result<(), String> {
        let current = read_user_file(user_dir, name, file)?;
        let patched = current.replacen(string(payload, "old")?, string(payload, "new")?, 1);
        if current == patched {
            return Err("old text was not found".into());
        }
        write_user_file(target, &patched)
    }

    fn execute_memory(&self, call: &PreparedCall) -> Result<ToolOutput, String> {
        let target = if string(&call.payload, "target")? == "memory" {
            MemoryTarget::Memory
        } else {
            MemoryTarget::User
        };
        let action = match string(&call.payload, "action")? {
            "add" => MemoryAction::Add(string(&call.payload, "entry")?.into()),
            "patch" => MemoryAction::Patch {
                old: string(&call.payload, "old")?.into(),
                new: string(&call.payload, "new")?.into(),
            },
            "remove" => MemoryAction::Remove(string(&call.payload, "entry")?.into()),
            _ => return Err("action must be add, patch, or remove".into()),
        };
        let adding = matches!(&action, MemoryAction::Add(_));
        let result = MemoryStore::new(&self.memory_root)
            .apply(target, action)
            .map_err(|error| memory_error(&error))?;
        if adding && !result.changed {
            return Err("memory entry already exists".into());
        }
        Ok(ToolOutput {
            text: format!("memory updated: {}", result.changed),
        })
    }

    fn execute_session_search(
        &self,
        call: &PreparedCall,
        current: &SessionId,
    ) -> Result<ToolOutput, String> {
        let limit = call
            .payload
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(5)
            .clamp(1, 10);
        let limit = usize::try_from(limit).map_err(|_| "invalid search limit".to_owned())?;
        let hits = self
            .store
            .lock()
            .map_err(|_| "store lock poisoned")?
            .session_search_excluding(string(&call.payload, "query")?, limit, Some(current))
            .map_err(|_| "session search failed")?;
        Ok(ToolOutput {
            text: hits
                .into_iter()
                .map(|hit| {
                    format!(
                        "{} [{}] {}: {}",
                        hit.session_title,
                        hit.date,
                        hit.role,
                        cut_chars(&hit.text, 300)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        })
    }

    fn execute_vault(&self, name: &str, call: &PreparedCall) -> Result<ToolOutput, String> {
        let store = VaultStore::new(&self.vault_root);
        let text = match name {
            "vault_search" => Self::execute_vault_search(&store, call)?,
            "vault_read" => {
                store
                    .read(string(&call.payload, "path")?)
                    .map_err(|error| vault_error(&error))?
                    .ok_or("vault note not found")?
                    .source
            }
            "vault_links" => format!(
                "{:?}",
                store
                    .links(string(&call.payload, "path")?)
                    .map_err(|error| vault_error(&error))?
            ),
            "vault_map" => Self::execute_vault_map(&store, call)?,
            "vault_orphans" => store
                .orphans()
                .map_err(|error| vault_error(&error))?
                .join("\n"),
            "vault_write" => {
                store
                    .write(
                        string(&call.payload, "path")?,
                        string(&call.payload, "content")?,
                    )
                    .map_err(|error| vault_error(&error))?;
                "vault note written".into()
            }
            "vault_append" => {
                store
                    .append(
                        string(&call.payload, "path")?,
                        string(&call.payload, "content")?,
                    )
                    .map_err(|error| vault_error(&error))?;
                "vault note appended".into()
            }
            _ => return Err("unknown vault tool".into()),
        };
        Ok(ToolOutput { text })
    }

    fn execute_vault_search(store: &VaultStore, call: &PreparedCall) -> Result<String, String> {
        let limit = call
            .payload
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(5)
            .clamp(1, 10);
        let limit = usize::try_from(limit).map_err(|_| "invalid search limit".to_owned())?;
        store
            .search(string(&call.payload, "query")?, limit)
            .map_err(|error| vault_error(&error))
            .map(|notes| {
                notes
                    .into_iter()
                    .map(|note| format!("{}: {}", note.path, note.description))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
    }

    fn execute_vault_map(store: &VaultStore, call: &PreparedCall) -> Result<String, String> {
        let depth = call
            .payload
            .get("depth")
            .and_then(Value::as_u64)
            .unwrap_or(1);
        let budget = call
            .payload
            .get("token_budget")
            .and_then(Value::as_u64)
            .unwrap_or(1_000);
        let depth = usize::try_from(depth).map_err(|_| "invalid depth".to_owned())?;
        let budget = usize::try_from(budget).map_err(|_| "invalid token budget".to_owned())?;
        store
            .map(string(&call.payload, "start")?, depth, budget)
            .map_err(|error| vault_error(&error))
    }
}
