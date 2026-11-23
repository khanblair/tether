//! GitHub Actions tools for one project.

use std::sync::Arc;

use serde_json::{Value, json};
use tether_ci::CiFailure;
use tether_providers::{BoxFuture, ToolSpec};
use tether_types::{Executor, ProjectId};
use tokio_util::sync::CancellationToken;

use crate::executors::{CiRunRequest, ExecutorError, RunReport};
use crate::permissions::Effect;
use crate::tool_loop::{ExecuteContext, PrepareContext, PreparedCall, ToolHandler, ToolOutput};

const MAX_COMMAND: usize = 2_000;

/// Request for a CI log list or log tail.
#[derive(Debug, Clone)]
pub struct CiLogsRequest {
    /// Optional workflow run id.
    pub run_id: Option<u64>,
    /// Optional job name filter.
    pub job: Option<String>,
    /// Project owning the repository.
    pub project_id: ProjectId,
    /// Turn cancellation.
    pub cancel: CancellationToken,
}

/// A result returned by the CI log service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiLogResult {
    /// Trimmed log text and parsed failure lines.
    pub text: String,
    /// Parsed failures.
    pub failures: Vec<CiFailure>,
}

/// The service boundary shared by `ci_run`, `ci_logs`, and the run executor.
pub trait CiRunService: Send + Sync {
    /// Starts one workflow and returns its final report text.
    fn run<'a>(
        &'a self,
        request: CiRunRequest,
        setup: Option<String>,
        sink: Option<&'a dyn crate::EventSink>,
    ) -> BoxFuture<'a, Result<RunReport, ExecutorError>>;
    /// Reads one workflow's logs.
    fn logs(&self, request: CiLogsRequest) -> BoxFuture<'_, Result<CiLogResult, String>>;
}

/// GitHub Actions tools bound to one project and branch.
pub struct CiTools {
    service: Arc<dyn CiRunService>,
    project_id: ProjectId,
    branch: String,
}

impl CiTools {
    /// Creates CI tools for a project with a GitHub remote.
    #[must_use]
    pub fn new(service: Arc<dyn CiRunService>, project_id: ProjectId, branch: String) -> Self {
        Self {
            service,
            project_id,
            branch,
        }
    }
}

impl ToolHandler for CiTools {
    fn specs(&self) -> Vec<ToolSpec> {
        vec![
            ToolSpec {
                name: "ci_run".into(),
                description: "Push the current branch and run a command on GitHub Actions. Use it when local executors cannot run the command.".into(),
                input_schema_json: json!({"type":"object","properties":{"command":{"type":"string","minLength":1,"maxLength":MAX_COMMAND},"setup":{"type":"string","enum":["","python","node","rust","java"]}},"required":["command"],"additionalProperties":false}).to_string(),
            },
            ToolSpec {
                name: "ci_logs".into(),
                description: "Read the latest GitHub Actions logs, or logs for a run id. Use it to inspect CI failures.".into(),
                input_schema_json: json!({"type":"object","properties":{"run_id":{"type":"integer","minimum":1},"job":{"type":"string","maxLength":200}},"additionalProperties":false}).to_string(),
            },
        ]
    }

    fn prepare(
        &self,
        name: &str,
        args_json: &str,
        _context: &PrepareContext<'_>,
    ) -> Result<PreparedCall, String> {
        let value: Value = serde_json::from_str(args_json).map_err(|_| "invalid JSON arguments")?;
        let object = value
            .as_object()
            .ok_or("tool arguments must be an object")?;
        match name {
            "ci_run" => self.prepare_run(object),
            "ci_logs" => Self::prepare_logs(object),
            _ => Err("unknown tool".into()),
        }
    }

    fn execute<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> BoxFuture<'a, Result<ToolOutput, String>> {
        Box::pin(async move { self.execute_call(call, context).await })
    }
}

impl CiTools {
    fn prepare_run(&self, object: &serde_json::Map<String, Value>) -> Result<PreparedCall, String> {
        let command = object
            .get("command")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty() && value.chars().count() <= MAX_COMMAND)
            .ok_or("command must be between 1 and 2000 characters")?;
        let setup = setup_value(object)?;
        if object
            .keys()
            .any(|key| !["command", "setup"].contains(&key.as_str()))
        {
            return Err("unknown ci_run argument".into());
        }
        Ok(PreparedCall {
            tool: "ci_run".into(),
            effect: Effect::Push,
            target: format!(
                "push {} to origin, then run \"{command}\" on GitHub Actions",
                self.branch
            ),
            executor: Some(Executor::Ci),
            payload: json!({"command": command, "setup": setup}),
        })
    }

    fn prepare_logs(object: &serde_json::Map<String, Value>) -> Result<PreparedCall, String> {
        if object
            .keys()
            .any(|key| !["run_id", "job"].contains(&key.as_str()))
        {
            return Err("unknown ci_logs argument".into());
        }
        let run_id = object
            .get("run_id")
            .map(|value| value.as_u64().ok_or("run_id must be a positive integer"))
            .transpose()?;
        let job = object
            .get("job")
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or("job must be a string")
            })
            .transpose()?;
        Ok(PreparedCall {
            tool: "ci_logs".into(),
            effect: Effect::Network,
            target: "GitHub Actions logs".into(),
            executor: None,
            payload: json!({"run_id": run_id, "job": job}),
        })
    }

    async fn execute_call<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> Result<ToolOutput, String> {
        match call.tool.as_str() {
            "ci_run" => self.execute_run(call, context).await,
            "ci_logs" => self.execute_logs(call, context).await,
            _ => Err("unknown tool".into()),
        }
    }

    async fn execute_run<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> Result<ToolOutput, String> {
        let command = call
            .payload
            .get("command")
            .and_then(Value::as_str)
            .ok_or("command is missing")?;
        let setup = call
            .payload
            .get("setup")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let report = self
            .service
            .run(
                CiRunRequest {
                    command: command.into(),
                    project_id: self.project_id.clone(),
                    cancel: context.cancel,
                },
                setup,
                context.sink,
            )
            .await
            .map_err(|error| error.to_string())?;
        Ok(ToolOutput {
            text: report.output,
        })
    }

    async fn execute_logs<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> Result<ToolOutput, String> {
        let request = CiLogsRequest {
            run_id: call.payload.get("run_id").and_then(Value::as_u64),
            job: call
                .payload
                .get("job")
                .and_then(Value::as_str)
                .map(str::to_owned),
            project_id: self.project_id.clone(),
            cancel: context.cancel,
        };
        let result = self.service.logs(request).await?;
        let failures = result
            .failures
            .iter()
            .map(|failure| {
                format!(
                    "{}:{} {}",
                    failure.file.as_deref().unwrap_or("unknown"),
                    failure.line.unwrap_or_default(),
                    failure.message
                )
            })
            .collect::<Vec<_>>();
        let prefix = if failures.is_empty() {
            String::new()
        } else {
            format!("{}\n", failures.join("\n"))
        };
        Ok(ToolOutput {
            text: format!("{prefix}{}", result.text),
        })
    }
}

fn setup_value(object: &serde_json::Map<String, Value>) -> Result<Option<String>, String> {
    let Some(value) = object.get("setup") else {
        return Ok(None);
    };
    let setup = value.as_str().ok_or("setup must be a string")?;
    ["", "python", "node", "rust", "java"]
        .contains(&setup)
        .then(|| (!setup.is_empty()).then(|| setup.to_owned()))
        .ok_or_else(|| "setup must be python, node, rust, java, or empty".into())
}

#[cfg(test)]
#[path = "ci_tools_tests.rs"]
mod tests;
