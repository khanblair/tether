//! Shell-aware routing for the three command executors.

use std::collections::BTreeSet;

use tether_types::Executor;

mod tokenizer;
pub use tokenizer::{ShellFeature, tokenize};

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
    if executor == Executor::Sandbox {
        let tokens =
            tokenize(&input.command).map_err(|feature| RouteError::ExplicitUnavailable {
                executor,
                reason: format!("sandbox cannot run {feature}"),
            })?;
        let program = tokens.first().ok_or(RouteError::InvalidCommand)?;
        if !sandbox_provides(program, &input.availability.sandbox_programs) {
            return Err(RouteError::ExplicitUnavailable {
                executor,
                reason: format!("no verified runtime pack provides {program}"),
            });
        }
        return Ok(Route {
            executor,
            argv: Some(sandbox_argv(
                &tokens,
                program,
                &input.availability.sandbox_programs,
            )),
            reason: "uses the explicitly requested executor".into(),
        });
    }
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
    if sandbox_provides(program, &input.availability.sandbox_programs) {
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
    let argv = if executor == Executor::Sandbox {
        tokenize(&input.command).ok().map(|tokens| {
            let program = tokens.first().map(String::as_str).unwrap_or_default();
            sandbox_argv(&tokens, program, &input.availability.sandbox_programs)
        })
    } else {
        None
    };
    Route {
        executor,
        argv,
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

fn sandbox_provides(program: &str, available: &BTreeSet<String>) -> bool {
    available.contains(program) || (program == "pytest" && available.contains("python"))
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
    if matches!(tokenize(command), Err(ShellFeature::InvalidSyntax)) {
        return Err(RouteError::InvalidCommand);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
