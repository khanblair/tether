//! Git and GitHub tools bound to one project workspace.

use std::sync::Arc;

use serde_json::{Value, json};
use tether_providers::{BoxFuture, ToolSpec};
use tether_types::ToolCallId;
use tether_vcs::{GithubClient, Identity, Transport};

use super::registry::{ExecuteContext, PrepareContext, PreparedCall, ToolHandler, ToolOutput};
use crate::permissions::Effect;

mod local;
mod remote;
mod validation;

const MAX_TEXT: usize = 20_000;
const MAX_PATH: usize = 400;
const DIFF_LIMIT: usize = 64 * 1024;

/// The severity of one commit-gate finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateSeverity {
    /// Informational finding.
    Info,
    /// A finding that is reported after a successful commit.
    Warn,
    /// A finding that prevents a commit.
    Block,
}

/// The input passed to commit gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInput {
    /// The proposed commit message.
    pub message: String,
    /// The staged unified diff.
    pub staged_diff: String,
    /// Paths changed by the staged commit.
    pub changed_paths: Vec<String>,
}

/// One finding returned by a commit gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateFinding {
    /// The finding severity.
    pub severity: GateSeverity,
    /// Stable rule identifier.
    pub rule_id: String,
    /// Safe message for the model and transcript.
    pub message: String,
}

/// A synchronous check run before a commit.
pub trait CommitGate: Send + Sync {
    /// Checks a staged commit.
    fn check(&self, input: &CommitInput) -> Vec<GateFinding>;
}

/// Reads the identity used for Git commits.
pub trait GitIdentityReader: Send + Sync {
    /// Returns the saved identity, if one exists.
    fn identity(&self) -> Result<Option<Identity>, String>;
}

/// Builds a fresh push transport for one call.
pub trait TransportFactory: Send + Sync {
    /// Creates a transport without retaining a token in the handler.
    fn transport(&self) -> Result<Box<dyn Transport>, String>;
}

/// Builds a fresh GitHub client for one call.
pub trait GithubClientFactory: Send + Sync {
    /// Creates a client whose token is held only for the call.
    fn client(&self) -> Result<GithubClient, String>;
}

/// Dependencies for the Git handler.
pub struct GitToolsConfig {
    /// Saved project remote URL, if any.
    pub remote_url: Option<String>,
    /// Saved project default branch, if any.
    pub default_branch: Option<String>,
    /// Reads the current commit identity.
    pub identity: Arc<dyn GitIdentityReader>,
    /// Creates push transports.
    pub transport: Arc<dyn TransportFactory>,
    /// Creates GitHub clients.
    pub github: Arc<dyn GithubClientFactory>,
    /// Commit gates owned by later phases.
    pub gates: Vec<Arc<dyn CommitGate>>,
}

/// Handles local Git and GitHub actions for one project.
pub struct GitTools {
    config: GitToolsConfig,
}

impl GitTools {
    /// Creates a Git handler for one project.
    #[must_use]
    pub fn new(config: GitToolsConfig) -> Self {
        Self { config }
    }
}

impl ToolHandler for GitTools {
    fn specs(&self) -> Vec<ToolSpec> {
        specs()
    }

    fn prepare(
        &self,
        name: &str,
        args_json: &str,
        _context: &PrepareContext<'_>,
    ) -> Result<PreparedCall, String> {
        validation::prepare(name, args_json, self.config.default_branch.as_deref())
    }

    fn execute<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> BoxFuture<'a, Result<ToolOutput, String>> {
        Box::pin(async move { self.execute_call(call, context).await })
    }
}

impl GitTools {
    async fn execute_call<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> Result<ToolOutput, String> {
        let root = context.guard.root().to_owned();
        let cancel = context.cancel.clone();
        let identity = Arc::clone(&self.config.identity);
        let gates = self.config.gates.clone();
        let remote_url = self.config.remote_url.clone();
        let default_branch = self.config.default_branch.clone();
        match call.tool.as_str() {
            "git_status" | "git_diff" | "git_log" | "git_branch" | "git_commit" => {
                let payload = call.payload;
                let result = tokio::task::spawn_blocking(move || {
                    local::execute(&root, &call.tool, &payload, identity, gates, cancel)
                })
                .await
                .map_err(|_| "git operation stopped".to_owned())??;
                Ok(ToolOutput { text: result })
            }
            "git_push" | "pr_create" | "pr_status" => {
                let result = remote::execute(
                    &call,
                    &root,
                    remote_url.as_deref(),
                    default_branch.as_deref(),
                    &self.config.transport,
                    &self.config.github,
                    cancel,
                )
                .await?;
                Ok(ToolOutput { text: result })
            }
            _ => Err("unknown git tool".to_owned()),
        }
    }
}

fn specs() -> Vec<ToolSpec> {
    [
        (
            "git_status",
            "Read the current Git status. Use it before deciding what to change.",
            json!({"type":"object","properties":{},"additionalProperties":false}),
        ),
        (
            "git_diff",
            "Read a compact Git diff. Use staged=true for the proposed commit.",
            json!({"type":"object","properties":{"staged":{"type":"boolean"},"path":{"type":"string","maxLength":MAX_PATH},"from":{"type":"string","maxLength":200},"to":{"type":"string","maxLength":200}},"additionalProperties":false}),
        ),
        (
            "git_log",
            "Read recent Git commits, optionally limited to one path.",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":50},"path":{"type":"string","maxLength":MAX_PATH}},"additionalProperties":false}),
        ),
        (
            "git_branch",
            "Create and optionally switch to a local branch for the task.",
            json!({"type":"object","properties":{"name":{"type":"string","maxLength":100},"from":{"type":"string","maxLength":200},"switch":{"type":"boolean"}},"additionalProperties":false}),
        ),
        (
            "git_commit",
            "Stage selected changes and create a commit with the saved identity.",
            json!({"type":"object","properties":{"message":{"type":"string","minLength":1,"maxLength":MAX_TEXT},"paths":{"type":"array","maxItems":100,"items":{"type":"string","maxLength":MAX_PATH}}},"required":["message"],"additionalProperties":false}),
        ),
        (
            "git_push",
            "Push the current branch to origin after the normal remote-action guard.",
            json!({"type":"object","properties":{"branch":{"type":"string","maxLength":100},"force":{"type":"boolean"},"delete":{"type":"boolean"}},"additionalProperties":false}),
        ),
        (
            "pr_create",
            "Open a pull request from the pushed current branch.",
            json!({"type":"object","properties":{"title":{"type":"string","minLength":1,"maxLength":200},"body":{"type":"string","maxLength":20000},"base":{"type":"string","maxLength":100},"draft":{"type":"boolean"}},"required":["title"],"additionalProperties":false}),
        ),
        (
            "pr_status",
            "Read pull request checks and review comments for the current branch.",
            json!({"type":"object","properties":{"number":{"type":"integer","minimum":1}},"additionalProperties":false}),
        ),
    ]
    .into_iter()
    .map(|(name, description, schema)| ToolSpec {
        name: name.to_owned(),
        description: description.to_owned(),
        input_schema_json: schema.to_string(),
    })
    .collect()
}

/// Caps a diff while retaining an explicit marker for the model.
pub(crate) fn cap_diff(mut value: String) -> String {
    if value.len() <= DIFF_LIMIT {
        return value;
    }
    value.truncate(DIFF_LIMIT);
    value.push_str("\n[diff truncated at 64 KiB]\n");
    value
}

pub(crate) fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

pub(crate) fn optional_string<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    value
        .get(key)
        .map_or(Ok(None), |item| string(item, key).map(Some))
}

pub(crate) fn optional_bool(value: &Value, key: &str, default: bool) -> Result<bool, String> {
    value.get(key).map_or(Ok(default), |item| {
        item.as_bool()
            .ok_or_else(|| format!("{key} must be a boolean"))
    })
}

pub(crate) fn optional_usize(value: &Value, key: &str, default: usize) -> Result<usize, String> {
    value.get(key).map_or(Ok(default), |item| {
        item.as_u64()
            .and_then(|number| usize::try_from(number).ok())
            .ok_or_else(|| format!("{key} must be a nonnegative integer"))
    })
}

pub(crate) fn require_object(name: &str, args_json: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(args_json).map_err(|_| "invalid JSON arguments")?;
    if !value.is_object() {
        return Err("tool arguments must be an object".into());
    }
    let allowed = validation::allowed_fields(name).ok_or("unknown tool")?;
    if value
        .as_object()
        .is_some_and(|object| object.keys().any(|key| !allowed.contains(&key.as_str())))
    {
        return Err("unknown tool argument".into());
    }
    Ok(value)
}

pub(crate) fn target_tool(
    name: &str,
    effect: Effect,
    target: String,
    payload: Value,
) -> PreparedCall {
    PreparedCall {
        tool: name.to_owned(),
        effect,
        target,
        executor: None,
        payload,
    }
}

pub(crate) fn _tool_call_id(_context: &ExecuteContext<'_>) -> ToolCallId {
    ToolCallId::new("git")
}
