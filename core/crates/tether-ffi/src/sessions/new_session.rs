use tether_engine::prompt::{ProjectInstructionRead, load_project_instructions};
use tether_memory::{MemoryStore, MemoryTarget};
use tether_store::{SessionRecord, project_dir};
use tether_types::ProjectId;

use super::{SessionRuntime, SessionRuntimeError};
use crate::NewSessionPreviewView;

#[uniffi::export]
impl SessionRuntime {
    /// Returns the data shown before a new session starts in one project.
    ///
    /// # Errors
    /// Returns `NotFound` when the project is absent, or `Unavailable` when a source cannot be
    /// read.
    pub fn new_session_preview(
        &self,
        project_id: String,
    ) -> Result<NewSessionPreviewView, SessionRuntimeError> {
        let store = self.store()?;
        let id = ProjectId::new(project_id);
        let project = store
            .load_project(&id)
            .map_err(|_| SessionRuntimeError::Unavailable)?
            .ok_or(SessionRuntimeError::NotFound)?;
        let workspace = project_dir(&self.files_root.join("workspaces"), &project);
        let (instructions_file, instructions_lines) =
            instruction_preview(load_project_instructions(&workspace));
        let memory = MemoryStore::new(self.files_root.join("memory"));
        let memory_entries = memory
            .count(MemoryTarget::Memory)
            .and_then(|memory_count| {
                memory
                    .count(MemoryTarget::User)
                    .map(|user_count| memory_count.saturating_add(user_count))
            })
            .map_err(|_| SessionRuntimeError::Unavailable)?;
        let catalog = super::open::skill_catalog(&store, Some(&workspace), &self.files_root, &id)?;
        let last_session = newest_session(
            store
                .list_sessions(&id)
                .map_err(|_| SessionRuntimeError::Unavailable)?,
        );
        Ok(NewSessionPreviewView {
            instructions_file,
            instructions_lines,
            memory_entries,
            skills_available: u32::try_from(catalog.skills.len()).unwrap_or(u32::MAX),
            last_session_title: last_session.as_ref().map(|value| value.title.clone()),
            last_session_updated_at_ms: last_session.map(|value| value.updated_at),
        })
    }
}

/// Returns the file name and line count to preview. A skipped file is not shown, because the
/// agent does not use it.
fn instruction_preview(read: ProjectInstructionRead) -> (Option<String>, u32) {
    match read {
        ProjectInstructionRead::Found(value) => (Some(value.file_name), line_count(&value.text)),
        ProjectInstructionRead::Skipped { .. } | ProjectInstructionRead::Missing => (None, 0),
    }
}

fn line_count(text: &str) -> u32 {
    u32::try_from(text.lines().count()).unwrap_or(u32::MAX)
}

fn newest_session(mut sessions: Vec<SessionRecord>) -> Option<SessionRecord> {
    sessions.sort_unstable_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.id.as_str().cmp(left.id.as_str()))
    });
    sessions.into_iter().next()
}

#[cfg(test)]
mod new_session_tests;
