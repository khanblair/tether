//! Slash commands, custom command loading, mentions, and session export.

use std::fmt;

use tether_types::PermissionMode;

mod builtins;
mod custom;
mod export;
mod mentions;

pub use custom::{CustomCommand, CustomCommandIssue, load_custom_commands};
pub use export::{ExportSession, export_session_markdown, export_session_messages_markdown};
pub use mentions::{
    MentionAttachment, MentionResolution, MentionSkip, MentionSpan, context_block, parse_mentions,
    resolve_mentions,
};

/// A capability that can make a slash command available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    /// Git and GitHub operations.
    Git,
    /// Command execution.
    Run,
    /// GitHub Actions.
    Ci,
    /// Linux mode.
    Linux,
    /// Second-model review.
    Review,
    /// Cost reporting.
    Cost,
    /// Context compaction.
    Compaction,
    /// Skill discovery and management.
    Skills,
}

impl Feature {
    /// Returns the short display name used in availability messages.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Git => "Git",
            Self::Run => "Run",
            Self::Ci => "CI",
            Self::Linux => "Linux",
            Self::Review => "Review",
            Self::Cost => "Cost",
            Self::Compaction => "Compaction",
            Self::Skills => "Skills",
        }
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Capabilities enabled for one command sheet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities(u16);

impl Capabilities {
    /// Creates an empty capability set.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Enables one capability.
    #[must_use]
    pub const fn with(mut self, feature: Feature) -> Self {
        self.0 |= 1 << feature as u16;
        self
    }

    /// Returns whether a capability is enabled.
    #[must_use]
    pub const fn contains(self, feature: Feature) -> bool {
        self.0 & (1 << feature as u16) != 0
    }
}

/// A destination opened by a slash command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Model picker.
    Model,
    /// Permission mode picker.
    Mode,
    /// Diff view.
    Diff,
    /// Checkpoint list.
    Rewind,
    /// Linux shell.
    Linux,
    /// CI run list.
    Ci,
    /// Skill list.
    Skills,
    /// Memory editor.
    Memory,
    /// Vault browser.
    Vault,
    /// Cost report.
    Cost,
    /// Context report.
    Context,
    /// Help screen.
    Help,
}

/// An action handled by the core without sending a model prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreAction {
    /// Create another session in the current project.
    New,
    /// Hide earlier messages from future context.
    Clear,
    /// Set the session permission mode.
    SetMode(PermissionMode),
    /// Export the current session.
    Export,
}

/// The effect of a resolved command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandKind {
    /// A core action.
    Action(CoreAction),
    /// A prompt sent to the model.
    Prompt(String),
    /// An app screen to open.
    Open(Screen),
    /// A command execution action declared for a later phase.
    Run,
    /// A compaction action declared for a later phase.
    Compact,
}

/// One command declared in the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// Name without the leading slash.
    pub name: String,
    /// One-line description for the command sheet.
    pub description: String,
    /// Syntax hint shown after the command name.
    pub args_hint: Option<String>,
    /// Core effect.
    pub kind: CommandKind,
    /// Feature required to use the command.
    pub requires: Option<Feature>,
}

/// A command after parsing its arguments and checking availability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The declared command.
    pub command: Command,
    /// Trimmed text after the command name.
    pub arguments: String,
}

/// Why command resolution failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// The command name is not known.
    Unknown {
        /// The typed name without the leading slash.
        name: String,
        /// Closest registry names, at most three.
        suggestions: Vec<String>,
    },
    /// The command exists but its feature is disabled.
    Unavailable {
        /// The typed command name.
        name: String,
        /// The missing feature.
        requires: Feature,
        /// Plain user-facing explanation.
        message: String,
    },
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown { name, .. } => write!(formatter, "unknown command /{name}"),
            Self::Unavailable { message, .. } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for CommandError {}

/// The built-in slash command registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRegistry {
    commands: Vec<Command>,
}

impl CommandRegistry {
    /// Returns the declared commands in stable order.
    #[must_use]
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Resolves a slash command, including custom commands.
    ///
    /// # Errors
    /// Returns [`CommandError::Unknown`] or [`CommandError::Unavailable`].
    pub fn resolve(
        &self,
        input: &str,
        capabilities: &Capabilities,
        custom: &[CustomCommand],
    ) -> Result<Resolved, CommandError> {
        let Some(rest) = input.strip_prefix('/') else {
            return unknown(input.trim().to_owned(), self, custom);
        };
        let (raw_name, arguments) = split_name(rest);
        let name = raw_name.to_owned();
        if !valid_name(&name) {
            return unknown(name, self, custom);
        }
        let command = self
            .commands
            .iter()
            .find(|command| command.name == name)
            .cloned()
            .or_else(|| {
                custom
                    .iter()
                    .find(|command| command.name == name)
                    .map(CustomCommand::as_command)
            });
        let Some(mut command) = command else {
            return unknown(name, self, custom);
        };
        if let Some(feature) = command.requires
            && !capabilities.contains(feature)
        {
            let message = format!(
                "/{name} needs {}, which is not set up yet",
                feature.as_str()
            );
            return Err(CommandError::Unavailable {
                name,
                requires: feature,
                message,
            });
        }
        if let CommandKind::Prompt(template) = &mut command.kind {
            *template = template.replace("$ARGUMENTS", arguments);
        }
        Ok(Resolved {
            command,
            arguments: arguments.to_owned(),
        })
    }

    /// Renders help with available commands first and gated commands second.
    #[must_use]
    pub fn help(&self, capabilities: &Capabilities, custom: &[CustomCommand]) -> String {
        let mut available = Vec::new();
        let mut unavailable = Vec::new();
        for command in &self.commands {
            let line = help_line(command);
            let missing = command
                .requires
                .filter(|feature| !capabilities.contains(*feature));
            match missing {
                Some(feature) => unavailable.push(format!("{line} ({feature} is not set up yet)")),
                None => available.push(line),
            }
        }
        for custom_command in custom {
            let command = custom_command.as_command();
            available.push(help_line(&command));
        }
        let mut output = String::from("Available commands:\n");
        output.push_str(&available.join("\n"));
        if !unavailable.is_empty() {
            output.push_str("\n\nUnavailable commands:\n");
            output.push_str(&unavailable.join("\n"));
        }
        output
    }
}

fn init_template() -> String {
    "Inspect the repository manifests, existing docs, test layout, and build commands. Then write a TETHER.md at the project root with run commands, conventions, and things to avoid. Use short plain sentences with no filler words. Keep it under 120 lines. Write it with the normal edit tool so the user approves the change. If TETHER.md already exists, prepare a patch instead. $ARGUMENTS".into()
}

fn split_name(input: &str) -> (&str, &str) {
    input
        .trim_start()
        .split_once(char::is_whitespace)
        .map_or_else(
            || (input.trim(), ""),
            |(name, arguments)| (name, arguments.trim()),
        )
}

fn valid_name(name: &str) -> bool {
    (1..=40).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn unknown(
    name: String,
    registry: &CommandRegistry,
    custom: &[CustomCommand],
) -> Result<Resolved, CommandError> {
    let query = name.to_ascii_lowercase();
    let mut names: Vec<String> = registry
        .commands
        .iter()
        .map(|command| command.name.clone())
        .chain(custom.iter().map(|command| command.name.clone()))
        .collect();
    names.sort();
    names.dedup();
    names.sort_by_key(|candidate| (edit_distance(&query, candidate), candidate.clone()));
    names.truncate(3);
    Err(CommandError::Unknown {
        name,
        suggestions: names,
    })
}

fn help_line(command: &Command) -> String {
    command.args_hint.as_ref().map_or_else(
        || format!("/{} - {}", command.name, command.description),
        |hint| format!("/{} {} - {}", command.name, hint, command.description),
    )
}

fn edit_distance(left: &str, right: &str) -> usize {
    let mut row: Vec<usize> = (0..=right.chars().count()).collect();
    for (index, left_char) in left.chars().enumerate() {
        let mut next = vec![index + 1];
        for (column, right_char) in right.chars().enumerate() {
            let substitution = row[column] + usize::from(left_char != right_char);
            next.push(
                (next[column] + 1)
                    .min(row[column + 1] + 1)
                    .min(substitution),
            );
        }
        row = next;
    }
    row[right.chars().count()]
}

#[cfg(test)]
mod tests;
