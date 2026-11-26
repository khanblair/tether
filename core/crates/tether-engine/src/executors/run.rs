//! The model-facing `run` tool and executor adapters.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tether_linux::{LinuxRunner, RunCommand, RunRequest};
use tether_providers::{BoxFuture, ToolSpec};
use tether_sandbox::{
    ExtraMount, ModuleCache, RunLimits, RunSpec, SandboxRunner, installed_programs, program_for,
};
use tether_types::{Executor, ProjectId, SessionId};
use tokio_util::sync::CancellationToken;

use super::{CiExecutor, CiRunRequest, RouteInput, Router, RunReport};
use crate::ToolHandler;
use crate::permissions::Effect;
use crate::tool_loop::{ExecuteContext, PrepareContext, PreparedCall, ToolOutput};

const OUTPUT_HEAD: usize = 4_000;
const OUTPUT_TAIL: usize = 12_000;

mod run_support;
use run_support::{
    append_user_args, format_result, guest_cwd, parse_cwd, parse_executor, parse_timeout,
    run_blocking, string,
};

/// Dependencies used by the `run` tool.
pub struct RunTools {
    packs_root: PathBuf,
    cache: Arc<ModuleCache>,
    linux_off: Option<Arc<LinuxRunner>>,
    linux_on: Option<Arc<LinuxRunner>>,
    ci: Arc<dyn CiExecutor>,
    ci_ready: bool,
    project_default: Option<Executor>,
    network_allowed: Arc<dyn Fn(&SessionId) -> bool + Send + Sync>,
    project_id: Arc<dyn Fn(&SessionId) -> ProjectId + Send + Sync>,
}

struct RunContext<'a> {
    session_id: SessionId,
    tool_call_id: tether_types::ToolCallId,
    cancel: CancellationToken,
    sink: Option<&'a dyn crate::EventSink>,
    workspace: PathBuf,
}

impl RunTools {
    /// Creates a sandbox-only handler with CI left unconfigured.
    ///
    /// # Errors
    /// Returns an error when the Wasmtime engine cannot start.
    pub fn new(packs_root: PathBuf) -> Result<Self, String> {
        let cache = ModuleCache::new().map_err(|error| format!("{error:?}"))?;
        Ok(Self {
            packs_root,
            cache: Arc::new(cache),
            linux_off: None,
            linux_on: None,
            ci: Arc::new(super::NotConfigured),
            ci_ready: false,
            project_default: None,
            network_allowed: Arc::new(|_| false),
            project_id: Arc::new(|session| ProjectId::new(session.as_str())),
        })
    }

    /// Adds Linux, CI, project-default, and network dependencies to a handler.
    #[must_use]
    pub fn with_executors(
        mut self,
        linux: Option<Arc<LinuxRunner>>,
        ci: Arc<dyn CiExecutor>,
        project_default: Option<Executor>,
        network_allowed: Arc<dyn Fn(&SessionId) -> bool + Send + Sync>,
    ) -> Self {
        self.linux_off.clone_from(&linux);
        self.linux_on = linux;
        self.ci_ready = ci.is_configured();
        self.ci = ci;
        self.project_default = project_default;
        self.network_allowed = network_allowed;
        self
    }

    /// Adds separate Linux runners for network-off and network-on projects.
    #[must_use]
    pub fn with_network_runners(
        mut self,
        // (network-off runner, network-on runner)
        runners: (Option<Arc<LinuxRunner>>, Option<Arc<LinuxRunner>>),
        ci: Arc<dyn CiExecutor>,
        project_default: Option<Executor>,
        network_allowed: Arc<dyn Fn(&SessionId) -> bool + Send + Sync>,
    ) -> Self {
        (self.linux_off, self.linux_on) = runners;
        self.ci_ready = ci.is_configured();
        self.ci = ci;
        self.project_default = project_default;
        self.network_allowed = network_allowed;
        self
    }

    /// Supplies the project lookup used by remote executor requests.
    #[must_use]
    pub fn with_project_id(
        mut self,
        project_id: Arc<dyn Fn(&SessionId) -> ProjectId + Send + Sync>,
    ) -> Self {
        self.project_id = project_id;
        self
    }

    /// Returns current verified pack programs and executor readiness.
    #[must_use]
    pub fn availability(&self) -> super::Availability {
        super::Availability {
            sandbox_programs: installed_programs(&self.packs_root),
            linux_ready: self.linux_off.is_some() || self.linux_on.is_some(),
            ci_ready: self.ci_ready,
        }
    }
}

impl ToolHandler for RunTools {
    fn specs(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            name: "run".into(),
            description: "Run a command through the executor router. Use it for tests, scripts, and local build commands.".into(),
            input_schema_json: json!({
                "type":"object",
                "properties": {
                    "command":{"type":"string"},
                    "executor":{"type":"string","enum":["sandbox","linux","ci"]},
                    "cwd":{"type":"string"},
                    "timeout_seconds":{"type":"integer","minimum":1,"maximum":600}
                },
                "required":["command"],
                "additionalProperties":false
            }).to_string(),
        }]
    }

    fn prepare(
        &self,
        name: &str,
        args_json: &str,
        context: &PrepareContext<'_>,
    ) -> Result<PreparedCall, String> {
        let (mut payload, command, cwd, timeout, route) =
            self.prepare_payload(name, args_json, context)?;
        let target_command = route
            .argv
            .as_ref()
            .map_or_else(|| command.clone(), |argv| argv.join(" "));
        let payload_object = payload
            .as_object_mut()
            .ok_or("tool arguments must be an object")?;
        if let Some(entry) = payload_object.get_mut("cwd") {
            *entry = Value::String(cwd.clone());
        }
        payload_object.insert("_cwd".into(), Value::String(cwd));
        payload_object.insert(
            "_timeout".into(),
            timeout.map_or(Value::Null, |value| json!(value)),
        );
        payload_object.insert("_argv".into(), json!(route.argv));
        payload_object.insert("_reason".into(), Value::String(route.reason.clone()));
        Ok(PreparedCall {
            tool: "run".into(),
            effect: if route.executor == Executor::Sandbox {
                Effect::RunSandbox
            } else {
                Effect::RunHost
            },
            target: target_command,
            executor: Some(route.executor),
            payload,
        })
    }

    fn execute<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> BoxFuture<'a, Result<ToolOutput, String>> {
        Box::pin(async move { self.execute_call(call, context).await })
    }
}

impl RunTools {
    fn prepare_payload(
        &self,
        name: &str,
        args_json: &str,
        context: &PrepareContext<'_>,
    ) -> Result<(Value, String, String, Option<u64>, super::Route), String> {
        if name != "run" {
            return Err("unknown tool".into());
        }
        let value: Value = serde_json::from_str(args_json).map_err(|_| "invalid JSON arguments")?;
        let object = value
            .as_object()
            .ok_or("tool arguments must be an object")?;
        let allowed = ["command", "executor", "cwd", "timeout_seconds"];
        if object.keys().any(|key| !allowed.contains(&key.as_str())) {
            return Err("unknown run argument".into());
        }
        let command = object
            .get("command")
            .and_then(Value::as_str)
            .filter(|command| !command.is_empty())
            .ok_or("command must be a non-empty string")?
            .to_owned();
        let requested = object.get("executor").map(parse_executor).transpose()?;
        let cwd = parse_cwd(object, context)?;
        let timeout = parse_timeout(object)?;
        let route = Router::route(&RouteInput {
            command: command.clone(),
            requested,
            project_default: self.project_default,
            availability: self.availability(),
        })
        .map_err(|error| error.to_string())?;
        Ok((value, command, cwd, timeout, route))
    }

    async fn execute_call<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> Result<ToolOutput, String> {
        let run_context = RunContext {
            session_id: context.session_id.clone(),
            tool_call_id: context.tool_call_id.clone(),
            cancel: context.cancel.clone(),
            sink: context.sink,
            workspace: context.guard.root().to_owned(),
        };
        let executor = call.executor.ok_or("run executor was not selected")?;
        let command = string(&call.payload, "command")?;
        let cwd = string(&call.payload, "_cwd")?;
        let timeout = call.payload.get("_timeout").and_then(Value::as_u64);
        let output = match executor {
            Executor::Sandbox => self.run_sandbox(&call, cwd, timeout, &run_context).await?,
            Executor::Linux => self.run_linux(command, cwd, timeout, &run_context).await?,
            Executor::Ci => self.run_ci(command, &run_context).await?,
        };
        Ok(ToolOutput {
            text: format_result(executor, &call, &output),
        })
    }

    async fn run_sandbox(
        &self,
        call: &PreparedCall,
        cwd: &str,
        timeout: Option<u64>,
        context: &RunContext<'_>,
    ) -> Result<RunReport, String> {
        let argv = call
            .payload
            .get("_argv")
            .and_then(Value::as_array)
            .ok_or("sandbox argv was not prepared")?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or("sandbox argv is invalid")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let program = argv.first().ok_or("sandbox command is empty")?;
        let pack = program_for(&self.packs_root, program).map_err(|error| format!("{error}"))?;
        let (module_bytes, prefix, mounts, env) = pack
            .run_spec_parts(program)
            .map_err(|error| format!("{error}"))?;
        let module = self
            .cache
            .load(&module_bytes)
            .map_err(|error| format!("{error:?}"))?;
        let workspace = tether_sandbox::WasiContextSpec::new(&context.workspace)
            .map_err(|error| format!("sandbox workspace failed: {error:?}"))?;
        let extra_mounts = mounts
            .into_iter()
            .map(|mount| ExtraMount {
                host: pack.root().join(mount.path),
                guest: mount.guest,
            })
            .collect();
        let spec = RunSpec {
            module,
            argv: append_user_args(prefix, &argv),
            workspace,
            cwd: guest_cwd(cwd),
            pack_root: pack.root().to_owned(),
            extra_mounts,
            extra_env: env,
            limits: RunLimits {
                timeout: Duration::from_secs(timeout.unwrap_or(60)),
                ..RunLimits::default()
            },
        };
        let cancel = context.cancel.clone();
        let runner = SandboxRunner::new(Arc::clone(&self.cache));
        run_blocking(move |sink| runner.run(&spec, sink, &cancel), context).await
    }

    async fn run_linux(
        &self,
        command: &str,
        cwd: &str,
        timeout: Option<u64>,
        context: &RunContext<'_>,
    ) -> Result<RunReport, String> {
        let network = (self.network_allowed)(&context.session_id);
        let runner = if network {
            self.linux_on.as_ref().or(self.linux_off.as_ref())
        } else {
            self.linux_off.as_ref().or(self.linux_on.as_ref())
        }
        .ok_or("Linux mode is not set up")?
        .clone();
        let request = RunRequest {
            command: RunCommand::Shell(command.into()),
            cwd: guest_cwd(cwd),
            extra_env: Vec::new(),
            timeout: Some(Duration::from_secs(timeout.unwrap_or(120))),
            output_cap_bytes: 1024 * 1024,
        };
        let cancel = context.cancel.clone();
        let result = run_blocking(move |sink| runner.run(&request, sink, &cancel), context).await?;
        let mut result = result;
        if !network && tether_linux::classify_network_failure(&result.output) {
            result.output.push_str("\nThe network is off for this project. Tell the user they can turn it on in the project settings, then retry.");
        }
        Ok(result)
    }

    async fn run_ci(&self, command: &str, context: &RunContext<'_>) -> Result<RunReport, String> {
        let request = CiRunRequest {
            command: command.into(),
            project_id: (self.project_id)(&context.session_id),
            cancel: context.cancel.clone(),
        };
        self.ci
            .run(request, context.sink)
            .await
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests;
