use std::fs;
use std::path::Path;

use super::{Command, CommandKind, CommandRegistry};
use tether_workspace::PathGuard;

const MAX_FILE_BYTES: usize = 16 * 1024;
const MAX_FILES: usize = 100;

/// A Markdown command loaded from a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomCommand {
    /// Command name without the leading slash.
    pub name: String,
    /// Optional frontmatter description or a default description.
    pub description: String,
    /// Prompt body.
    pub template: String,
}

impl CustomCommand {
    pub(crate) fn as_command(&self) -> Command {
        Command {
            name: self.name.clone(),
            description: self.description.clone(),
            args_hint: None,
            kind: CommandKind::Prompt(self.template.clone()),
            requires: None,
        }
    }
}

/// A custom command file that could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomCommandIssue {
    /// Relative path of the command file when available.
    pub path: String,
    /// Safe explanation of the issue.
    pub message: String,
}

/// Loads project custom commands through the workspace guard.
#[must_use]
pub fn load_custom_commands(root: &Path) -> (Vec<CustomCommand>, Vec<CustomCommandIssue>) {
    let Ok(guard) = PathGuard::new(root) else {
        return (
            Vec::new(),
            vec![issue(".tether/commands", "project root is unavailable")],
        );
    };
    let directory = root.join(".tether").join("commands");
    let Ok(entries) = fs::read_dir(&directory) else {
        return (Vec::new(), Vec::new());
    };
    let mut issues = Vec::new();
    let files = scan_directory(entries, &mut issues);
    let mut commands = Vec::new();
    let builtins = CommandRegistry::builtin();
    for (relative, name) in files {
        match load_command(&guard, &builtins, &relative, name) {
            Ok(command) => commands.push(command),
            Err(problem) => issues.push(issue(&relative, problem)),
        }
    }
    (commands, issues)
}

/// Lists the Markdown files in the command directory, sorted by path.
///
/// Files that are not Markdown, and files past the cap, become issues.
fn scan_directory(
    entries: fs::ReadDir,
    issues: &mut Vec<CustomCommandIssue>,
) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mut skipped_after_cap = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("md") {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("unknown");
            issues.push(issue(
                &format!(".tether/commands/{name}"),
                "file name must end in .md",
            ));
            continue;
        }
        if files.len() >= MAX_FILES {
            skipped_after_cap = true;
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        files.push((format!(".tether/commands/{name}.md"), name.to_owned()));
    }
    files.sort();
    if skipped_after_cap {
        issues.push(issue(
            ".tether/commands",
            "more than 100 files were skipped",
        ));
    }
    files
}

/// Reads one command file, or returns the reason it was skipped.
fn load_command(
    guard: &PathGuard,
    builtins: &CommandRegistry,
    relative: &str,
    name: String,
) -> Result<CustomCommand, &'static str> {
    if !valid_name(&name) {
        return Err("invalid command name");
    }
    if builtins
        .commands()
        .iter()
        .any(|command| command.name == name)
    {
        return Err("built-in command wins");
    }
    let path = guard
        .resolve(relative)
        .map_err(|_| "unsafe or symlinked file")?;
    let bytes = fs::read(&path).map_err(|_| "file could not be read")?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err("file exceeds 16 KiB");
    }
    let text = String::from_utf8(bytes).map_err(|_| "file is not UTF-8")?;
    let (description, template) = frontmatter(&text);
    Ok(CustomCommand {
        name,
        description,
        template,
    })
}

fn frontmatter(text: &str) -> (String, String) {
    let Some(rest) = text.strip_prefix("---\n") else {
        return ("Custom command".into(), text.to_owned());
    };
    let Some((header, body)) = rest.split_once("\n---\n") else {
        return ("Custom command".into(), text.to_owned());
    };
    let description = header
        .lines()
        .find_map(|line| line.strip_prefix("description:").map(str::trim))
        .filter(|value| !value.is_empty())
        .unwrap_or("Custom command")
        .to_owned();
    (description, body.to_owned())
}

fn valid_name(name: &str) -> bool {
    (1..=40).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn issue(path: &str, message: &str) -> CustomCommandIssue {
    CustomCommandIssue {
        path: path.into(),
        message: message.into(),
    }
}
