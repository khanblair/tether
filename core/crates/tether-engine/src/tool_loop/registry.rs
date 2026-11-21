//! Prepared tool calls and the extensible handler registry.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use tether_providers::{BoxFuture, ToolSpec};
use tether_types::{Executor, SessionId, ToolCallId};
use tether_workspace::PathGuard;
use tokio_util::sync::CancellationToken;

use super::file_tools::FileTools;
use crate::EventSink;
use crate::permissions::Effect;

/// Context available while a handler validates a call without touching disk.
pub struct PrepareContext<'a> {
    /// The workspace boundary used to resolve file targets.
    pub guard: &'a PathGuard,
}

/// A validated call whose target and arguments are ready to execute.
#[derive(Debug, Clone)]
pub struct PreparedCall {
    /// Registered tool name.
    pub tool: String,
    /// Effect reported by the handler.
    pub effect: Effect,
    /// Guarded workspace-relative display target.
    pub target: String,
    /// Executor selected by the handler, if any.
    pub executor: Option<Executor>,
    /// Already-parsed arguments. Execution never parses raw provider text.
    pub payload: Value,
}

/// The final result returned by a tool handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutput {
    /// Compact result text sent back to the model.
    pub text: String,
}

/// Context available while a prepared call runs.
pub struct ExecuteContext<'a> {
    /// Session that owns the call.
    pub session_id: &'a SessionId,
    /// Provider call identifier.
    pub tool_call_id: &'a ToolCallId,
    /// Turn cancellation token.
    pub cancel: CancellationToken,
    /// Event sink for streamed tool output.
    pub sink: Option<&'a dyn EventSink>,
    /// Workspace boundary for the final execution check.
    pub guard: &'a PathGuard,
    /// Captures file bytes before a mutation.
    pub before_edit: &'a mut (dyn FnMut(&[PathBuf]) -> Result<(), String> + Send),
}

/// A handler owns a group of tool specifications and their prepared execution.
pub trait ToolHandler: Send + Sync {
    /// Returns the provider-facing contracts owned by this handler.
    fn specs(&self) -> Vec<ToolSpec>;
    /// Validates and prepares one call without touching disk or the network.
    ///
    /// # Errors
    /// Returns a validation error when the arguments do not match the tool.
    fn prepare(
        &self,
        name: &str,
        args_json: &str,
        context: &PrepareContext<'_>,
    ) -> Result<PreparedCall, String>;
    /// Executes a previously prepared call.
    fn execute<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> BoxFuture<'a, Result<ToolOutput, String>>;
}

/// A duplicate tool name was offered by more than one handler.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RegistryError {
    /// The name already belongs to another handler.
    #[error("duplicate tool: {0}")]
    DuplicateTool(String),
}

/// Routes provider calls to registered tool handlers.
pub struct ToolRegistry {
    handlers: Vec<Arc<dyn ToolHandler>>,
    owners: std::collections::HashMap<String, usize>,
}

impl ToolRegistry {
    /// Builds a registry and rejects duplicate provider-facing names.
    ///
    /// # Errors
    /// Returns [`RegistryError::DuplicateTool`] when two handlers claim one name.
    pub fn new(handlers: Vec<Arc<dyn ToolHandler>>) -> Result<Self, RegistryError> {
        let mut owners = std::collections::HashMap::new();
        for (index, handler) in handlers.iter().enumerate() {
            insert_specs(&mut owners, handler.specs(), index)?;
        }
        Ok(Self { handlers, owners })
    }

    pub(crate) fn from_single(handler: Arc<dyn ToolHandler>) -> Self {
        let mut owners = std::collections::HashMap::new();
        for spec in handler.specs() {
            owners.insert(spec.name, 0);
        }
        Self {
            handlers: vec![handler],
            owners,
        }
    }

    /// Returns every registered provider-facing specification.
    #[must_use]
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.handlers
            .iter()
            .flat_map(|handler| handler.specs())
            .collect()
    }

    /// Prepares a call, or returns an Unknown call for an unowned name.
    ///
    /// # Errors
    /// Returns handler validation errors.
    pub fn prepare(
        &self,
        name: &str,
        args_json: &str,
        context: &PrepareContext<'_>,
    ) -> Result<PreparedCall, String> {
        let Some(index) = self.owners.get(name) else {
            return Ok(PreparedCall {
                tool: name.to_owned(),
                effect: Effect::Unknown,
                target: String::new(),
                executor: None,
                payload: Value::Null,
            });
        };
        self.handlers[*index].prepare(name, args_json, context)
    }

    /// Executes a prepared call through its owning handler.
    ///
    /// # Errors
    /// Returns an error for an unregistered call or handler failure.
    pub fn execute<'a>(
        &'a self,
        call: PreparedCall,
        context: ExecuteContext<'a>,
    ) -> BoxFuture<'a, Result<ToolOutput, String>> {
        let Some(index) = self.owners.get(&call.tool).copied() else {
            return unknown_tool();
        };
        self.handlers[index].execute(call, context)
    }
}

fn insert_specs(
    owners: &mut std::collections::HashMap<String, usize>,
    specs: Vec<ToolSpec>,
    index: usize,
) -> Result<(), RegistryError> {
    for spec in specs {
        if owners.insert(spec.name.clone(), index).is_some() {
            return Err(RegistryError::DuplicateTool(spec.name));
        }
    }
    Ok(())
}

fn unknown_tool() -> BoxFuture<'static, Result<ToolOutput, String>> {
    Box::pin(std::future::ready(Err("unknown tool".to_owned())))
}

/// Creates the core file tool registry. Later phases pass their handlers to
/// `ToolLoop::with_handlers` through `ToolRuntime::handlers`; nothing edits this file.
pub(crate) fn default_registry(guard: PathGuard) -> ToolRegistry {
    ToolRegistry::from_single(Arc::new(FileTools::new(guard)))
}

/// The file tools plus the handlers a session was opened with.
pub(crate) fn registry_with(
    guard: PathGuard,
    extra: Vec<Arc<dyn ToolHandler>>,
) -> Result<ToolRegistry, RegistryError> {
    let mut handlers: Vec<Arc<dyn ToolHandler>> = vec![Arc::new(FileTools::new(guard))];
    handlers.extend(extra);
    ToolRegistry::new(handlers)
}

#[cfg(test)]
mod tests;
