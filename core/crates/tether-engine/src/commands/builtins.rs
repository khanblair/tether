use super::{Command, CommandKind, CommandRegistry, CoreAction, Feature, Screen, init_template};
use tether_types::PermissionMode;

impl CommandRegistry {
    /// Declares every command in product-scope F7, in help order.
    #[must_use]
    pub fn builtin() -> Self {
        let commands = [
            core_commands(),
            git_commands(),
            executor_commands(),
            view_commands(),
            final_commands(),
        ]
        .into_iter()
        .flatten()
        .collect();
        Self { commands }
    }
}

fn core_commands() -> Vec<Command> {
    vec![
        command(
            "new",
            "Start a new session in this project",
            None,
            CoreAction::New,
            None,
        ),
        command(
            "clear",
            "Clear context but keep the session record",
            None,
            CoreAction::Clear,
            None,
        ),
        Command {
            name: "compact".into(),
            description: "Summarize the context now".into(),
            args_hint: Some("[focus]".into()),
            kind: CommandKind::Compact,
            requires: Some(Feature::Compaction),
        },
        open("model", "Open the model picker", Screen::Model, None),
        open(
            "mode",
            "Open the permission mode picker",
            Screen::Mode,
            None,
        ),
        command(
            "plan",
            "Switch to Plan mode",
            None,
            CoreAction::SetMode(PermissionMode::Plan),
            None,
        ),
    ]
}

fn git_commands() -> Vec<Command> {
    vec![
        open("diff", "Open uncommitted changes", Screen::Diff, None),
        open("rewind", "Open the checkpoint list", Screen::Rewind, None),
        prompt(
            "commit",
            "Ask the agent to write a commit message and commit",
            "Write a commit message for the current changes, then commit them. $ARGUMENTS",
            Some(Feature::Git),
        ),
        prompt(
            "push",
            "Push the current branch",
            "Push the current branch. $ARGUMENTS",
            Some(Feature::Git),
        ),
        prompt(
            "pr",
            "Draft and open a pull request",
            "Draft and open a pull request for the current changes. $ARGUMENTS",
            Some(Feature::Git),
        ),
    ]
}

fn executor_commands() -> Vec<Command> {
    vec![
        Command {
            name: "run".into(),
            description: "Run a command yourself in the default executor".into(),
            args_hint: Some("<cmd>".into()),
            kind: CommandKind::Run,
            requires: Some(Feature::Run),
        },
        open(
            "linux",
            "Open a Linux mode shell for this project",
            Screen::Linux,
            Some(Feature::Linux),
        ),
        open("ci", "Show recent CI runs", Screen::Ci, Some(Feature::Ci)),
        prompt(
            "review",
            "Second-model review of uncommitted changes",
            "Review the uncommitted changes with a second model. $ARGUMENTS",
            Some(Feature::Review),
        ),
        open(
            "cost",
            "Show cost for this session and project",
            Screen::Cost,
            Some(Feature::Cost),
        ),
    ]
}

fn view_commands() -> Vec<Command> {
    vec![
        open(
            "context",
            "Show what fills the context window",
            Screen::Context,
            None,
        ),
        open(
            "skills",
            "List skills; open one; install one",
            Screen::Skills,
            Some(Feature::Skills),
        ),
        open(
            "memory",
            "Show MEMORY.md and USER.md; edit them by hand",
            Screen::Memory,
            None,
        ),
        open(
            "vault",
            "Open the vault; /vault lint lists orphan notes and unresolved links",
            Screen::Vault,
            None,
        ),
    ]
}

fn final_commands() -> Vec<Command> {
    vec![
        prompt(
            "init",
            "Generate a TETHER.md for this project",
            &init_template(),
            None,
        ),
        command(
            "export",
            "Export the session as Markdown",
            None,
            CoreAction::Export,
            None,
        ),
        open("help", "List commands", Screen::Help, None),
    ]
}

fn command(
    name: &str,
    description: &str,
    args_hint: Option<&str>,
    action: CoreAction,
    requires: Option<Feature>,
) -> Command {
    Command {
        name: name.into(),
        description: description.into(),
        args_hint: args_hint.map(str::to_owned),
        kind: CommandKind::Action(action),
        requires,
    }
}

fn open(name: &str, description: &str, screen: Screen, requires: Option<Feature>) -> Command {
    Command {
        name: name.into(),
        description: description.into(),
        args_hint: None,
        kind: CommandKind::Open(screen),
        requires,
    }
}

fn prompt(name: &str, description: &str, template: &str, requires: Option<Feature>) -> Command {
    Command {
        name: name.into(),
        description: description.into(),
        args_hint: None,
        kind: CommandKind::Prompt(template.into()),
        requires,
    }
}
