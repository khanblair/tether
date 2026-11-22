//! UniFFI views for command and mention outcomes.

use tether_engine::commands::{
    Capabilities, CommandError, CommandKind, CommandRegistry, MentionResolution,
};
use tether_engine::commands::{ExportSession, export_session_markdown};
use tether_types::SessionId;

use crate::SessionRuntime;

/// A command sheet row.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CommandView {
    /// Command name without the leading slash.
    pub name: String,
    /// One-line description.
    pub description: String,
    /// Argument hint.
    pub args_hint: Option<String>,
    /// Whether the command is available.
    pub available: bool,
    /// Missing feature, when unavailable.
    pub requires: Option<String>,
}

/// The result of resolving one command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CommandResolutionView {
    /// Command name.
    pub name: String,
    /// Trimmed arguments.
    pub arguments: String,
    /// Kind name for app routing.
    pub kind: String,
    /// Prompt text when this is a prompt command.
    pub prompt: Option<String>,
}

/// A command could not be resolved.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CommandRuntimeError {
    /// No command matched the input.
    #[error("unknown command")]
    Unknown {
        /// Typed command name.
        name: String,
        /// Closest command names.
        suggestions: Vec<String>,
    },
    /// A command requires a disabled capability.
    #[error("command is unavailable: {reason}")]
    Unavailable {
        /// Plain explanation. Not named `message`: a UniFFI error field of that name clashes
        /// with the Kotlin exception's own `message`.
        reason: String,
    },
}

/// Returns the capabilities currently enabled by the FFI runtime.
#[must_use]
pub(crate) fn current_capabilities() -> Capabilities {
    Capabilities::empty()
}

#[uniffi::export]
impl SessionRuntime {
    /// Lists built-in commands for the composer.
    #[must_use]
    pub fn command_sheet(&self) -> Vec<CommandView> {
        command_views(&CommandRegistry::builtin(), current_capabilities())
    }

    /// Resolves a built-in command for the composer.
    ///
    /// # Errors
    /// Returns unknown or unavailable command details.
    pub fn resolve_command(
        &self,
        input: &str,
    ) -> Result<CommandResolutionView, CommandRuntimeError> {
        let registry = CommandRegistry::builtin();
        registry
            .resolve(input, &current_capabilities(), &[])
            .map(command_view)
            .map_err(map_error)
    }

    /// Exports one saved session as Markdown.
    ///
    /// # Errors
    /// Returns `NotFound` for an unknown session or `Unavailable` for storage errors.
    pub fn export_session(&self, session_id: String) -> Result<String, crate::SessionRuntimeError> {
        let store = self.store()?;
        let id = SessionId::new(session_id);
        let session = store
            .load_session(&id)
            .map_err(|_| crate::SessionRuntimeError::Unavailable)?
            .ok_or(crate::SessionRuntimeError::NotFound)?;
        let project = store
            .load_project(&session.project_id)
            .map_err(|_| crate::SessionRuntimeError::Unavailable)?;
        let title = match project {
            Some(project) if session.title.is_empty() => project.name,
            _ => session.title.clone(),
        };
        let messages = store
            .messages(&id)
            .map_err(|_| crate::SessionRuntimeError::Unavailable)?;
        Ok(export_session_markdown(&ExportSession {
            id,
            title,
            model: session.model_id,
            messages,
        }))
    }
}

/// File mentions resolved while preparing one message.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SendOutcomeView {
    /// Workspace-relative paths attached to the provider request.
    pub attached_paths: Vec<String>,
    /// Workspace-relative paths that were skipped.
    pub skipped_paths: Vec<String>,
    /// Reasons corresponding to `skipped_paths`.
    pub skipped_reasons: Vec<String>,
}

impl From<MentionResolution> for SendOutcomeView {
    fn from(value: MentionResolution) -> Self {
        Self {
            attached_paths: value.attached.into_iter().map(|file| file.path).collect(),
            skipped_paths: value.skipped.iter().map(|file| file.path.clone()).collect(),
            skipped_reasons: value.skipped.into_iter().map(|file| file.reason).collect(),
        }
    }
}

fn command_views(registry: &CommandRegistry, capabilities: Capabilities) -> Vec<CommandView> {
    registry
        .commands()
        .iter()
        .map(|command| CommandView {
            name: command.name.clone(),
            description: command.description.clone(),
            args_hint: command.args_hint.clone(),
            available: command
                .requires
                .is_none_or(|feature| capabilities.contains(feature)),
            requires: command.requires.map(|feature| feature.as_str().into()),
        })
        .collect()
}

fn command_view(resolved: tether_engine::commands::Resolved) -> CommandResolutionView {
    let (kind, prompt) = match resolved.command.kind {
        CommandKind::Action(_) => ("action", None),
        CommandKind::Prompt(prompt) => ("prompt", Some(prompt)),
        CommandKind::Open(_) => ("open", None),
        CommandKind::Run => ("run", None),
        CommandKind::Compact => ("compact", None),
    };
    CommandResolutionView {
        name: resolved.command.name,
        arguments: resolved.arguments,
        kind: kind.into(),
        prompt,
    }
}

fn map_error(error: CommandError) -> CommandRuntimeError {
    match error {
        CommandError::Unknown { name, suggestions } => {
            CommandRuntimeError::Unknown { name, suggestions }
        }
        CommandError::Unavailable { message, .. } => {
            CommandRuntimeError::Unavailable { reason: message }
        }
    }
}
