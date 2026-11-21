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
    let mut files = Vec::new();
    let mut issues = Vec::new();
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
        let relative = format!(".tether/commands/{}.md", name);
        files.push((relative, name.to_owned()));
    }
    files.sort();
    let mut commands = Vec::new();
    if skipped_after_cap {
        issues.push(issue(
            ".tether/commands",
            "more than 100 files were skipped",
        ));
    }
    let builtins = CommandRegistry::builtin();
    for (relative, name) in files {
        if !valid_name(&name) {
            issues.push(issue(&relative, "invalid command name"));
            continue;
        }
        if builtins
            .commands()
            .iter()
            .any(|command| command.name == name)
        {
            issues.push(issue(&relative, "built-in command wins"));
            continue;
        }
        let Ok(path) = guard.resolve(&relative) else {
            issues.push(issue(&relative, "unsafe or symlinked file"));
            continue;
        };
        let Ok(bytes) = fs::read(&path) else {
            issues.push(issue(&relative, "file could not be read"));
            continue;
        };
        if bytes.len() > MAX_FILE_BYTES {
            issues.push(issue(&relative, "file exceeds 16 KiB"));
            continue;
        }
        let Ok(text) = String::from_utf8(bytes) else {
            issues.push(issue(&relative, "file is not UTF-8"));
            continue;
        };
        let (description, template) = frontmatter(&text);
        commands.push(CustomCommand {
            name,
            description,
            template,
        });
    }
    (commands, issues)
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
