use std::fs;
use std::path::{Path, PathBuf};

use tether_engine::ToolLoop;
use tether_engine::prompt::{
    PromptBuilder, PromptInputs, PromptSources, capture_snapshot, load_instructions,
};
use tether_store::Store;
use tether_types::SessionId;
use tether_workspace::PathGuard;

use super::{SessionRuntime, SessionRuntimeError};
use crate::PromptSectionView;

#[uniffi::export]
impl SessionRuntime {
    /// Saves or removes the global instructions file.
    ///
    /// # Errors
    /// Returns `InstructionsTooLong` over 32 KiB or `Unavailable` for file errors.
    pub fn set_global_instructions(&self, text: Option<String>) -> Result<(), SessionRuntimeError> {
        if text.as_ref().is_some_and(|value| value.len() > 32 * 1024) {
            return Err(SessionRuntimeError::InstructionsTooLong);
        }
        let path = self.global_instructions_path();
        match text {
            Some(text) => {
                let folder = path.parent().ok_or(SessionRuntimeError::Unavailable)?;
                fs::create_dir_all(folder).map_err(|_| SessionRuntimeError::Unavailable)?;
                fs::write(path, text).map_err(|_| SessionRuntimeError::Unavailable)
            }
            None => match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(_) => Err(SessionRuntimeError::Unavailable),
            },
        }
    }

    /// Reads the current global instructions file.
    ///
    /// # Errors
    /// Returns `Unavailable` when the file exists but cannot be read as UTF-8.
    pub fn global_instructions(&self) -> Result<Option<String>, SessionRuntimeError> {
        match fs::read_to_string(self.global_instructions_path()) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(SessionRuntimeError::Unavailable),
        }
    }

    /// Returns the current prompt section counts for one session.
    ///
    /// # Errors
    /// Returns an error when the session, store, or prompt sources are unavailable.
    pub fn session_prompt_sections(
        &self,
        session_id: &str,
    ) -> Result<Vec<PromptSectionView>, SessionRuntimeError> {
        let store = self.store()?;
        let id = SessionId::new(session_id);
        let session = store
            .load_session(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .ok_or(SessionRuntimeError::NotFound)?;
        let project = store
            .load_project(&session.project_id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .ok_or(SessionRuntimeError::Unavailable)?;
        let project_root = project_root_of(self, &id)?;
        let sources = self.prompt_sources(&project.name, project_root.clone());
        let snapshot = snapshot_for(&store, &id, &sources)?;
        let instructions = load_instructions(
            sources.global_instructions_path.as_deref(),
            sources.project_root.as_deref(),
        );
        let tools = project_root
            .as_deref()
            .and_then(|root| PathGuard::new(root).ok())
            .map_or_else(Vec::new, |guard| ToolLoop::new(guard).specs());
        let today = sources
            .today
            .clone()
            .unwrap_or_else(tether_engine::prompt::current_date);
        let prompt = PromptBuilder::build(&PromptInputs {
            project_name: &sources.project_name,
            executors: &sources.executors,
            branch: sources.branch.as_deref(),
            mode: session.mode,
            tool_specs: &tools,
            global_instructions: &instructions.global,
            project_instructions: &instructions.project,
            memory_block: &snapshot.memory_block,
            vault_block: &snapshot.vault_block,
            skills_index: sources.skills_index.as_deref(),
            today: &today,
        });
        Ok(prompt
            .sections
            .into_iter()
            .map(|section| PromptSectionView {
                name: section.name,
                chars: section.chars,
                approx_tokens: section.approx_tokens,
            })
            .collect())
    }
}

fn project_root_of(
    runtime: &SessionRuntime,
    id: &SessionId,
) -> Result<Option<PathBuf>, SessionRuntimeError> {
    Ok(runtime
        .project_roots
        .lock()
        .map_err(|_| SessionRuntimeError::Unavailable)?
        .get(id)
        .cloned())
}

pub(super) fn files_root(database_path: &Path) -> PathBuf {
    let database_dir = database_path.parent().unwrap_or_else(|| Path::new("."));
    if database_dir.file_name().and_then(|name| name.to_str()) == Some("db") {
        database_dir.parent().unwrap_or(database_dir).to_owned()
    } else {
        database_dir.to_owned()
    }
}

fn snapshot_for(
    store: &Store,
    id: &SessionId,
    sources: &PromptSources,
) -> Result<tether_store::PromptSnapshot, SessionRuntimeError> {
    if let Some(snapshot) = store
        .load_prompt_snapshot(id)
        .map_err(|_| SessionRuntimeError::Unavailable)?
    {
        return Ok(snapshot);
    }
    let mut snapshot = capture_snapshot(sources).map_err(|_| SessionRuntimeError::Unavailable)?;
    snapshot.created_at = super::now_ms();
    store
        .save_prompt_snapshot(
            id,
            &snapshot.memory_block,
            &snapshot.vault_block,
            snapshot.created_at,
        )
        .map_err(|_| SessionRuntimeError::Unavailable)?;
    Ok(snapshot)
}
