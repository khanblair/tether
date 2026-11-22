//! Permission modes, conservative project rules, and their pure decisions.

use tether_types::{Executor, NewRule, PermissionMode, PermissionRule};

/// The side effect a prepared tool call can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Reads workspace or session data.
    Read,
    /// Changes files in the workspace.
    Edit,
    /// Runs code in the app sandbox.
    RunSandbox,
    /// Runs code outside the app sandbox.
    RunHost,
    /// Sends or fetches data over a network.
    Network,
    /// Pushes commits or branches.
    Push,
    /// Force pushes or performs an equivalent destructive remote action.
    ForcePush,
    /// Changes only state inside the current session.
    Internal,
    /// Is not owned by a registered handler.
    Unknown,
}

/// The result of evaluating one prepared tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// The call can run now.
    Allow,
    /// The user must answer before the call runs.
    Ask,
    /// The mode forbids the call.
    Block {
        /// A safe explanation suitable for the model and the transcript.
        reason: String,
    },
}

/// The target and executor used while matching project rules.
#[derive(Debug, Clone, Copy)]
pub struct PermissionRequest<'a> {
    /// Tool name.
    pub tool: &'a str,
    /// Guarded display target.
    pub target: &'a str,
    /// Selected executor, if the tool has one.
    pub executor: Option<Executor>,
}

/// Returns the mode decision for one prepared call.
#[must_use]
pub fn decide(
    mode: PermissionMode,
    effect: Effect,
    rules: &[PermissionRule],
    request: PermissionRequest<'_>,
) -> Decision {
    let base = table_decision(mode, effect);
    if !matches!(base, Decision::Ask) {
        return base;
    }
    if matches!(
        effect,
        Effect::Edit | Effect::RunSandbox | Effect::RunHost | Effect::Network
    ) && rules.iter().any(|rule| rule_matches(rule, request))
    {
        Decision::Allow
    } else {
        base
    }
}

/// Derives the rule created by a safe Always allow answer.
#[must_use]
pub fn rule_for(
    effect: Effect,
    tool: &str,
    target: &str,
    executor: Option<Executor>,
) -> Option<NewRule> {
    if !matches!(effect, Effect::Edit | Effect::RunSandbox | Effect::RunHost) {
        return None;
    }
    if effect == Effect::Edit {
        if !is_file_tool(tool) || tool == "move_file" || !safe_relative_path(target) {
            return None;
        }
        return Some(NewRule {
            tool: tool.to_owned(),
            pattern: target.to_owned(),
            executor: None,
        });
    }
    if tool != "run" || has_shell_character(target) {
        return None;
    }
    let program = target.split_whitespace().next()?;
    if program.is_empty() {
        return None;
    }
    Some(NewRule {
        tool: tool.to_owned(),
        pattern: program.to_owned(),
        executor,
    })
}

fn table_decision(mode: PermissionMode, effect: Effect) -> Decision {
    match effect {
        Effect::Internal | Effect::Read => Decision::Allow,
        Effect::Unknown => block(mode, "unknown tools"),
        Effect::Edit => match mode {
            PermissionMode::Plan => block(mode, "file edits"),
            PermissionMode::Ask => Decision::Ask,
            _ => Decision::Allow,
        },
        Effect::RunSandbox => match mode {
            PermissionMode::Plan => block(mode, "sandbox runs"),
            PermissionMode::Ask | PermissionMode::AcceptEdits => Decision::Ask,
            PermissionMode::Auto | PermissionMode::Bypass => Decision::Allow,
        },
        Effect::RunHost => match mode {
            PermissionMode::Plan => block(mode, "host runs"),
            PermissionMode::Bypass => Decision::Allow,
            _ => Decision::Ask,
        },
        Effect::Network => match mode {
            PermissionMode::Bypass => Decision::Allow,
            _ => Decision::Ask,
        },
        Effect::Push => match mode {
            PermissionMode::Plan => block(mode, "pushes"),
            PermissionMode::Bypass => Decision::Allow,
            _ => Decision::Ask,
        },
        Effect::ForcePush => match mode {
            PermissionMode::Plan => block(mode, "force pushes"),
            _ => Decision::Ask,
        },
    }
}

fn block(mode: PermissionMode, effect: &str) -> Decision {
    Decision::Block {
        reason: format!("{} mode blocks {effect}", mode_name(mode)),
    }
}

fn mode_name(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::Plan => "Plan",
        PermissionMode::Ask => "Ask",
        PermissionMode::AcceptEdits => "Accept edits",
        PermissionMode::Auto => "Auto",
        PermissionMode::Bypass => "Bypass",
    }
}

fn rule_matches(rule: &PermissionRule, request: PermissionRequest<'_>) -> bool {
    rule.tool == request.tool
        && rule
            .executor
            .is_none_or(|executor| Some(executor) == request.executor)
        && if request.tool == "run" {
            !has_shell_character(request.target)
                && request.target.split_whitespace().next() == Some(rule.pattern.as_str())
        } else {
            pattern_matches(&rule.pattern, request.target)
        }
}

fn pattern_matches(pattern: &str, target: &str) -> bool {
    pattern.strip_suffix("/*").map_or_else(
        || pattern == target,
        |prefix| target.starts_with(&format!("{prefix}/")),
    )
}

fn is_file_tool(tool: &str) -> bool {
    matches!(
        tool,
        "write_file" | "edit_file" | "multi_edit" | "move_file" | "delete_file"
    )
}

fn safe_relative_path(path: &str) -> bool {
    let path = std::path::Path::new(path);
    !path.is_absolute()
        && !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn has_shell_character(command: &str) -> bool {
    command.chars().any(|character| {
        matches!(
            character,
            ';' | '&' | '|' | '`' | '$' | '<' | '>' | '(' | ')' | '\n' | '\\'
        )
    })
}

#[cfg(test)]
mod tests;
