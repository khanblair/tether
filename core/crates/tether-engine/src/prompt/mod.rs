//! Stable system prompt assembly and session-scoped prompt sources.

use std::path::PathBuf;

use tether_providers::ToolSpec;
use tether_store::PromptSnapshot;
use tether_types::{Executor, PermissionMode};
use thiserror::Error;

mod base;
mod builder;
mod instructions;
mod vault;

#[cfg(test)]
mod tests;

pub use builder::PromptBuilder;
pub use instructions::{
    InstructionBlocks, ProjectInstruction, ProjectInstructionRead, load_instructions,
    load_project_instructions,
};
pub use vault::load_vault_slice;

/// Paths and optional environment values used to build a session prompt.
#[derive(Debug, Clone, Default)]
pub struct PromptSources {
    /// Display name of the active project.
    pub project_name: String,
    /// Project root used for guarded instruction reads.
    pub project_root: Option<PathBuf>,
    /// Root containing `MEMORY.md` and `USER.md`.
    pub memory_root: Option<PathBuf>,
    /// Root containing the private Markdown vault.
    pub vault_root: Option<PathBuf>,
    /// Global instructions file.
    pub global_instructions_path: Option<PathBuf>,
    /// Executors available to the session.
    pub executors: Vec<Executor>,
    /// Current repository branch, when known.
    pub branch: Option<String>,
    /// Pre-built skill index, when discovery is available.
    pub skills_index: Option<String>,
    /// Test or platform supplied date.
    pub today: Option<String>,
}

/// Values used to assemble one deterministic system prompt.
#[derive(Debug, Clone)]
pub struct PromptInputs<'a> {
    /// Display name of the active project.
    pub project_name: &'a str,
    /// Available executors.
    pub executors: &'a [Executor],
    /// Current branch, when known.
    pub branch: Option<&'a str>,
    /// Permission mode for this turn.
    pub mode: PermissionMode,
    /// Tools registered for this session.
    pub tool_specs: &'a [ToolSpec],
    /// Global instructions, already capped and labeled by the caller.
    pub global_instructions: &'a str,
    /// Project instructions, already capped and labeled by the caller.
    pub project_instructions: &'a str,
    /// Frozen memory system block.
    pub memory_block: &'a str,
    /// Frozen vault system block.
    pub vault_block: &'a str,
    /// Optional skills index.
    pub skills_index: Option<&'a str>,
    /// Date line value.
    pub today: &'a str,
}

/// One prompt section and its context estimate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSection {
    /// Stable section name.
    pub name: String,
    /// Character count of the section body.
    pub chars: u32,
    /// Conservative token estimate.
    pub approx_tokens: u32,
}

/// A complete system prompt and its section accounting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPrompt {
    /// Joined system prompt text.
    pub text: String,
    /// Sections in cache-friendly order.
    pub sections: Vec<PromptSection>,
}

/// A prompt source could not be captured.
#[derive(Debug, Error)]
pub enum PromptError {
    /// The memory files could not be read.
    #[error("memory snapshot unavailable")]
    Memory,
}

/// Estimates tokens by rounding characters divided by four up.
#[must_use]
pub fn estimate_tokens(text: &str) -> u32 {
    let chars = text.chars().count();
    u32::try_from(chars.saturating_add(3) / 4).unwrap_or(u32::MAX)
}

/// Returns the current UTC date as `YYYY-MM-DD`.
#[must_use]
pub fn current_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() / 86_400);
    let z = i64::try_from(days).unwrap_or(i64::MAX) + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day = z - era * 146_097;
    let year_day = (day - day / 1_460 + day / 36_524 - day / 146_096) / 365;
    let year = year_day + era * 400;
    let day_of_year = day - (365 * year_day + year_day / 4 - year_day / 100);
    let month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month + 2) / 5 + 1;
    let month = month + if month < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Captures memory and vault blocks for the first turn of a session.
///
/// # Errors
/// Returns an error when memory files cannot be read.
pub fn capture_snapshot(sources: &PromptSources) -> Result<PromptSnapshot, PromptError> {
    let memory_block = if let Some(root) = &sources.memory_root {
        let store = tether_memory::MemoryStore::new(root);
        tether_memory::FrozenMemorySnapshot::load(&store)
            .map_err(|_| PromptError::Memory)?
            .system_block()
    } else {
        String::new()
    };
    let vault_block = sources
        .vault_root
        .as_deref()
        .map_or_else(String::new, |root| {
            load_vault_slice(root, &sources.project_name)
        });
    Ok(PromptSnapshot {
        memory_block,
        vault_block,
        skills_block: sources.skills_index.clone().unwrap_or_default(),
        created_at: 0,
    })
}
