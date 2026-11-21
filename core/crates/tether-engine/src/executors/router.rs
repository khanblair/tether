//! Shell-aware routing for the three command executors.

use std::collections::BTreeSet;
use std::fmt;

use tether_types::Executor;

/// Executor availability discovered by the platform runtime.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Availability {
    /// Program names exposed by verified runtime packs.
    pub sandbox_programs: BTreeSet<String>,
    /// Whether Linux mode has a usable PRoot and root filesystem.
    pub linux_ready: bool,
    /// Whether CI credentials and repository setup are ready.
    pub ci_ready: bool,
}

/// Inputs used by the automatic executor router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteInput {
    /// Command text supplied by the caller.
    pub command: String,
    /// Explicit executor requested by the caller.
    pub requested: Option<Executor>,
    /// Project default from effective configuration.
    pub project_default: Option<Executor>,
    /// Current executor availability.
    pub availability: Availability,
}

/// The selected executor and the exact sandbox argv, when applicable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// Executor selected for the command.
    pub executor: Executor,
    /// Parsed argv for a sandbox run.
    pub argv: Option<Vec<String>>,
    /// One-sentence explanation of the selection.
    pub reason: String,
}

/// An unquoted shell feature that WASI argv cannot represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellFeature {
    /// A pipe character.
    Pipe,
    /// A redirection operator.
    Redirect,
    /// The `&&` operator.
    And,
    /// The `||` operator.
    Or,
    /// A command separator.
    Semicolon,
    /// A background operator.
    Background,
    /// A subshell delimiter.
    Subshell,
    /// A glob pattern.
    Glob,
    /// Variable expansion syntax.
    Variable,
    /// An environment assignment before the command.
    EnvironmentAssignment,
    /// A here-document operator.
    HereDoc,
    /// An unterminated quote or escape.
    InvalidSyntax,
}

impl fmt::Display for ShellFeature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Pipe => "pipe",
            Self::Redirect => "redirect",
            Self::And => "&&",
            Self::Or => "||",
            Self::Semicolon => ";",
            Self::Background => "background",
            Self::Subshell => "subshell",
            Self::Glob => "glob",
            Self::Variable => "variable",
            Self::EnvironmentAssignment => "environment assignment prefix",
            Self::HereDoc => "here-doc",
            Self::InvalidSyntax => "invalid shell syntax",
        };
        formatter.write_str(name)
    }
}

/// Router errors that name the unavailable or unsupported path.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RouteError {
    /// The command is empty or exceeds the input limit.
    #[error("command must be between 1 and 2000 characters")]
    InvalidCommand,
    /// An explicit executor is not available.
    #[error("requested {executor:?} executor is unavailable: {reason}")]
    ExplicitUnavailable {
        /// Executor requested by the caller.
        executor: Executor,
        /// Why the executor is unavailable.
        reason: String,
    },
    /// No configured executor can run the command.
    #[error("no executor is available for {command}; set up Linux mode or CI")]
    NoExecutor {
        /// Command that could not be routed.
        command: String,
    },
    /// Git commands must use the dedicated git tools.
    #[error("git is not a run target; use the git tools")]
    GitTools,
    /// The command contains a shell feature and cannot be tokenized for sandbox use.
    #[error("command uses {0}")]
    Shell(ShellFeature),
}

/// Routes commands according to explicit choice, project default, and built-ins.
#[derive(Debug, Default)]
pub struct Router;

impl Router {
    /// Selects an executor and explains the choice.
    ///
    /// # Errors
    /// Returns a clear error when the command or requested executor is unavailable.
    pub fn route(input: &RouteInput) -> Result<Route, RouteError> {
        validate_command(&input.command)?;
        if let Some(executor) = input.requested {
            return explicit_route(executor, input);
        }
        if let Some(executor) = input.project_default
            && is_available(executor, &input.availability)
        {
            return Ok(route_for(
                executor,
                input,
                "uses the project default executor",
            ));
        }
        builtin_route(input)
    }
}

fn explicit_route(executor: Executor, input: &RouteInput) -> Result<Route, RouteError> {
    if is_available(executor, &input.availability) {
        return Ok(route_for(
            executor,
            input,
            "uses the explicitly requested executor",
        ));
    }
    Err(RouteError::ExplicitUnavailable {
        executor,
        reason: unavailable_reason(executor),
    })
}

fn builtin_route(input: &RouteInput) -> Result<Route, RouteError> {
    let tokens = match tokenize(&input.command) {
        Ok(tokens) => tokens,
        Err(feature) => return shell_fallback(feature, input),
    };
    let Some(program) = tokens.first() else {
        return Err(RouteError::InvalidCommand);
    };
    if program == "git" {
        return Err(RouteError::GitTools);
    }
    if is_host_tool(program) {
        return host_fallback(input, &format!("{program} requires Linux mode"));
    }
    if input.availability.sandbox_programs.contains(program) {
        let argv = sandbox_argv(&tokens, program, &input.availability.sandbox_programs);
        let reason = format!("{program} is provided by the installed runtime pack");
        return Ok(Route {
            executor: Executor::Sandbox,
            argv: Some(argv),
            reason,
        });
    }
    host_fallback(
        input,
        &format!("{program} is not provided by an installed sandbox pack"),
    )
}

fn is_host_tool(program: &str) -> bool {
    matches!(
        program,
        "npm"
            | "npx"
            | "yarn"
            | "pnpm"
            | "cargo"
            | "rustc"
            | "gradle"
            | "gradlew"
            | "make"
            | "cmake"
            | "gcc"
            | "g++"
            | "clang"
            | "go"
            | "pip"
            | "apk"
    )
}

fn shell_fallback(feature: ShellFeature, input: &RouteInput) -> Result<Route, RouteError> {
    if input.availability.linux_ready {
        return Ok(route_for(
            Executor::Linux,
            input,
            &format!("uses a {feature}, which the sandbox cannot run, so Linux mode is used"),
        ));
    }
    if input.availability.ci_ready {
        return Ok(route_for(
            Executor::Ci,
            input,
            &format!("uses a {feature}, Linux mode is not set up, so CI is used"),
        ));
    }
    Err(RouteError::NoExecutor {
        command: input.command.clone(),
    })
}

fn host_fallback(input: &RouteInput, detail: &str) -> Result<Route, RouteError> {
    if input.availability.linux_ready {
        return Ok(route_for(
            Executor::Linux,
            input,
            &format!("{detail}, so Linux mode is used"),
        ));
    }
    if input.availability.ci_ready {
        return Ok(route_for(
            Executor::Ci,
            input,
            &format!("{detail}, Linux mode is not set up, so CI is used"),
        ));
    }
    Err(RouteError::NoExecutor {
        command: input.command.clone(),
    })
}

fn route_for(executor: Executor, input: &RouteInput, reason: &str) -> Route {
    Route {
        executor,
        argv: if executor == Executor::Sandbox {
            tokenize(&input.command).ok()
        } else {
            None
        },
        reason: reason.to_owned(),
    }
}

fn sandbox_argv(tokens: &[String], program: &str, available: &BTreeSet<String>) -> Vec<String> {
    if program == "pytest" && available.contains("python") {
        let mut argv = vec!["python".into(), "-m".into(), "pytest".into()];
        argv.extend(tokens.iter().skip(1).cloned());
        argv
    } else {
        tokens.to_vec()
    }
}

fn is_available(executor: Executor, availability: &Availability) -> bool {
    match executor {
        Executor::Sandbox => !availability.sandbox_programs.is_empty(),
        Executor::Linux => availability.linux_ready,
        Executor::Ci => availability.ci_ready,
    }
}

fn unavailable_reason(executor: Executor) -> String {
    match executor {
        Executor::Sandbox => "no verified runtime pack provides the command".into(),
        Executor::Linux => "Linux mode is not set up".into(),
        Executor::Ci => "CI is not configured".into(),
    }
}

fn validate_command(command: &str) -> Result<(), RouteError> {
    if command.is_empty() || command.chars().count() > 2_000 || command.contains('\0') {
        return Err(RouteError::InvalidCommand);
    }
    Ok(())
}

/// Tokenizes simple POSIX-style words and rejects shell syntax.
///
/// # Errors
/// Returns the first unquoted shell feature or invalid quote syntax.
#[expect(
    clippy::excessive_nesting,
    clippy::too_many_lines,
    reason = "The tokenizer keeps quote and escape state in one small parser."
)]
pub fn tokenize(command: &str) -> Result<Vec<String>, ShellFeature> {
    if command.chars().count() > 2_000 || command.contains('\0') {
        return Err(ShellFeature::InvalidSyntax);
    }
    let mut tokens = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut first_word = true;
    let chars: Vec<char> = command.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if escaped {
            word.push(ch);
            escaped = false;
            index += 1;
            continue;
        }
        if quote == Some('\'') {
            if ch == '\'' {
                quote = None;
            } else {
                word.push(ch);
            }
            index += 1;
            continue;
        }
        if quote == Some('"') {
            match ch {
                '"' => quote = None,
                '\\' if chars.get(index + 1).is_some() => {
                    index += 1;
                    word.push(chars[index]);
                }
                _ => word.push(ch),
            }
            index += 1;
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '\\' => escaped = true,
            ' ' | '\t' | '\r' => flush_word(&mut tokens, &mut word, &mut first_word),
            '\n' => return Err(ShellFeature::InvalidSyntax),
            '|' if chars.get(index + 1) == Some(&'|') => return Err(ShellFeature::Or),
            '|' => return Err(ShellFeature::Pipe),
            '&' if chars.get(index + 1) == Some(&'&') => return Err(ShellFeature::And),
            '&' => return Err(ShellFeature::Background),
            ';' => return Err(ShellFeature::Semicolon),
            '<' if chars.get(index + 1) == Some(&'<') => return Err(ShellFeature::HereDoc),
            '<' | '>' => return Err(ShellFeature::Redirect),
            '(' | ')' => return Err(ShellFeature::Subshell),
            '*' | '?' | '[' => return Err(ShellFeature::Glob),
            '$' => return Err(ShellFeature::Variable),
            _ => word.push(ch),
        }
        if first_word && word.contains('=') && word.split('=').next().is_some_and(valid_env_name) {
            return Err(ShellFeature::EnvironmentAssignment);
        }
        index += 1;
    }
    if escaped || quote.is_some() {
        return Err(ShellFeature::InvalidSyntax);
    }
    flush_word(&mut tokens, &mut word, &mut first_word);
    Ok(tokens)
}

fn flush_word(tokens: &mut Vec<String>, word: &mut String, first_word: &mut bool) {
    if !word.is_empty() {
        tokens.push(std::mem::take(word));
        *first_word = false;
    }
}

fn valid_env_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(test)]
mod tests;
