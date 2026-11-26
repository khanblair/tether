//! Bounded provider and tool exchange with saved messages and edit checkpoints.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tether_providers::{ChatProvider, ChatRequest, StopReason, ToolSpec};
use tether_store::{BlobStore, Store};
use tether_types::{
    CoreError, CoreEvent, Message, MessagePart, PermissionMode, Role, SessionId, SessionStatus,
    ToolCallStatus, UsageSnapshot,
};
use tether_workspace::PathGuard;
use tokio_util::sync::CancellationToken;

pub(crate) mod ci_tools;
mod file_tools;
pub(crate) mod git_tools;
pub(crate) mod knowledge_tools;
pub(crate) mod mcp_tools;
mod registry;
pub(crate) mod session_tools;
mod stream;
use crate::permissions::{self, Decision, PermissionRequest};
use crate::quality::{self, GateReport, PendingChange, QualityGate};
use crate::{ApprovalAnswer, ApprovalRequest};
pub use registry::{
    ExecuteContext, PrepareContext, PreparedCall, RegistryError, ToolHandler, ToolOutput,
    ToolRegistry,
};
use stream::response;
#[cfg(test)]
mod path_tests;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod tests;

const MAX_EXCHANGES: usize = 12;
const MAX_CALLS_PER_EXCHANGE: usize = 16;
const MAX_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_RESULT_CHARS: usize = 16 * 1024;
const TEXT_BATCH_CHARS: usize = 64;
const TEXT_BATCH_MS: u64 = 32;

/// The completed exchange, including messages already saved by the loop.
#[derive(Debug)]
pub struct ToolLoopResult {
    /// Original messages and every generated assistant and tool message.
    pub messages: Vec<Message>,
    /// The final assistant reply.
    pub final_text: String,
    /// Number of provider responses.
    pub exchanges: usize,
    /// Token usage across provider responses in this turn.
    pub usage: UsageSnapshot,
}

/// Runs a guarded set of file tools against successive model responses.
pub struct ToolLoop {
    guard: PathGuard,
    registry: ToolRegistry,
    quality: Arc<QualityGate>,
}

/// Data and policy shared by one tool turn.
pub struct ToolLoopContext<'a> {
    /// Session whose transcript and checkpoints receive the exchange.
    pub session_id: &'a SessionId,
    /// Session database used to commit every completed message.
    pub store: &'a mut Store,
    /// Private content-addressed blob folder.
    pub blobs: &'a BlobStore,
    /// Stops provider requests and prevents further tools.
    pub cancel: CancellationToken,
    /// Resolves user approval requests.
    pub approver: &'a dyn crate::Approver,
    /// Reads the actor's current permission mode for each call.
    pub mode: &'a (dyn Fn() -> PermissionMode + Send + Sync),
    /// Receives visible tool proposals and streamed reply events.
    pub sink: Option<&'a dyn crate::EventSink>,
}

struct AskInput<'a> {
    id: &'a tether_types::ToolCallId,
    project_id: &'a tether_types::ProjectId,
    anchor: &'a tether_types::MessageId,
}

impl ToolLoop {
    /// Binds all tool calls to one validated workspace.
    #[must_use]
    pub fn new(guard: PathGuard) -> Self {
        Self {
            registry: registry::default_registry(guard.clone()),
            quality: Arc::new(QualityGate::new(guard.clone())),
            guard,
        }
    }

    /// Binds all tool calls to one workspace and adds handlers from later phases.
    /// The file tools come first; a name an extra handler shares with another is an error.
    ///
    /// # Errors
    /// Returns [`registry::RegistryError`] when two handlers claim one tool name.
    pub fn with_handlers(
        guard: PathGuard,
        extra: Vec<std::sync::Arc<dyn registry::ToolHandler>>,
    ) -> Result<Self, registry::RegistryError> {
        Ok(Self {
            registry: registry::registry_with(guard.clone(), extra)?,
            quality: Arc::new(QualityGate::new(guard.clone())),
            guard,
        })
    }

    /// Tool contracts sent to the provider.
    #[must_use]
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.registry.specs()
    }

    /// Runs responses until the model ends its turn or the exchange cap is reached.
    /// The context's approval callback receives the exact target before a mutation.
    ///
    /// # Errors
    /// Returns provider, cancellation, or malformed-stream errors. Individual tool
    /// failures become tool results so the model can recover.
    pub async fn run(
        &self,
        provider: Arc<dyn ChatProvider>,
        request: ChatRequest,
        context: &mut ToolLoopContext<'_>,
    ) -> Result<ToolLoopResult, CoreError> {
        self.run_inner(provider, request, context, None).await
    }

    /// Runs a tool turn with a budget gate before each provider request.
    pub(crate) async fn run_with_budget(
        &self,
        provider: Arc<dyn ChatProvider>,
        request: ChatRequest,
        context: &mut ToolLoopContext<'_>,
        budget: &crate::budget::BudgetGate,
    ) -> Result<ToolLoopResult, CoreError> {
        self.run_inner(provider, request, context, Some(budget))
            .await
    }

    #[expect(
        clippy::too_many_lines,
        clippy::excessive_nesting,
        clippy::cognitive_complexity,
        reason = "the exchange loop keeps response and tool ordering visible"
    )]
    async fn run_inner(
        &self,
        provider: Arc<dyn ChatProvider>,
        mut request: ChatRequest,
        context: &mut ToolLoopContext<'_>,
        budget: Option<&crate::budget::BudgetGate>,
    ) -> Result<ToolLoopResult, CoreError> {
        request.tools = self.specs();
        let mut usage = UsageSnapshot::default();
        for exchange in 1..=MAX_EXCHANGES {
            if let Some(budget) = budget {
                let session = context
                    .store
                    .load_session(context.session_id)
                    .map_err(|_| storage_error())?
                    .ok_or_else(storage_error)?;
                let sink = context.sink.ok_or_else(storage_error)?;
                let target = crate::budget::CheckTarget {
                    session: &session,
                    sink,
                    cancel: &context.cancel,
                };
                budget.check(context.store, target, now_ms()).await?;
            }
            let response = response(Arc::clone(&provider), request.clone(), context).await?;
            usage.tokens_in = usage.tokens_in.saturating_add(response.usage.tokens_in);
            usage.tokens_out = usage.tokens_out.saturating_add(response.usage.tokens_out);
            let mut parts = Vec::new();
            if !response.thinking.is_empty() {
                parts.push(MessagePart::Thinking {
                    text: response.thinking.clone(),
                });
            }
            if !response.text.is_empty() {
                parts.push(MessagePart::Text {
                    text: response.text.clone(),
                });
            }
            for call in &response.calls {
                parts.push(MessagePart::ToolCall {
                    id: call.id.clone(),
                    tool: call.name.clone(),
                    args_json: call.args.clone(),
                });
            }
            let assistant = if let Some(id) = response.message_id {
                context
                    .store
                    .replace_message_parts(&id, &parts)
                    .map_err(|_| storage_error())?;
                context
                    .store
                    .messages(context.session_id)
                    .map_err(|_| storage_error())?
                    .into_iter()
                    .find(|message| message.id == id)
                    .ok_or_else(storage_error)?
            } else {
                context
                    .store
                    .append_message(context.session_id, Role::Assistant, &parts, now_ms())
                    .map_err(|_| storage_error())?
            };
            request.messages.push(assistant.clone());
            let session = context
                .store
                .load_session(context.session_id)
                .map_err(|_| storage_error())?
                .ok_or_else(storage_error)?;
            context
                .store
                .record_response_usage(&tether_store::ResponseUsageRecord {
                    session_id: context.session_id.clone(),
                    provider: session.provider,
                    model_id: request.model_id.clone(),
                    project_id: session.project_id.as_str().into(),
                    day: crate::usage::utc_day_from_ms(now_ms()),
                    usage: response.usage,
                    price_known: response.price_known,
                    now_ms: now_ms(),
                })
                .map_err(|_| storage_error())?;
            usage.cost_micros = usage.cost_micros.saturating_add(response.usage.cost_micros);
            if response.stop != StopReason::ToolUse {
                return Ok(ToolLoopResult {
                    messages: request.messages,
                    final_text: response.text,
                    exchanges: exchange,
                    usage,
                });
            }
            if response.calls.is_empty() {
                return Err(invalid("provider requested tools without a call"));
            }
            let mut exchange_changes = Vec::new();
            for (index, call) in response.calls.iter().enumerate() {
                if context.cancel.is_cancelled() {
                    save_cancelled_calls(context, &response.calls[index..])?;
                    return Err(cancelled());
                }
                let prepare = self.registry.prepare(
                    &call.name,
                    &call.args,
                    &PrepareContext { guard: &self.guard },
                );
                let prepared = match prepare {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        append_tool_result(context, &mut request, &call.id, true, &error)?;
                        continue;
                    }
                };
                if let Some(sink) = context.sink {
                    sink.on_event(CoreEvent::ToolProposed {
                        session_id: context.session_id.clone(),
                        tool_call_id: call.id.clone(),
                        tool: call.name.clone(),
                        target: prepared.display_target(),
                    });
                }
                if context.cancel.is_cancelled() {
                    save_cancelled_calls(context, &response.calls[index..])?;
                    return Err(cancelled());
                }
                set_status(context, SessionStatus::CheckingTool)?;
                let project_id = context
                    .store
                    .load_session(context.session_id)
                    .map_err(|_| storage_error())?
                    .ok_or_else(storage_error)?
                    .project_id;
                let mut rules = context
                    .store
                    .permission_rules(&project_id)
                    .map_err(|_| storage_error())?;
                rules.extend(
                    crate::config::project_allowed_command_rules(
                        context.store,
                        &project_id,
                        self.guard.root(),
                    )
                    .map_err(|_| storage_error())?,
                );
                let mode = (context.mode)();
                let staged_skill = prepared.tool == "skill_manage"
                    && prepared.effect == crate::permissions::Effect::Edit;
                if staged_skill {
                    let change = crate::tool_loop::knowledge_tools::staged_skill_change(
                        context.session_id,
                        &call.id,
                        &prepared,
                    );
                    context
                        .store
                        .stage_skill_change(&change)
                        .map_err(|_| storage_error())?;
                }
                let decision =
                    if mode == PermissionMode::Plan && self.registry.blocks_in_plan(&prepared) {
                        Decision::Block {
                            reason: "Plan mode blocks network tools".to_owned(),
                        }
                    } else {
                        if self.registry.auto_approves(&prepared) {
                            rules.push(tether_types::PermissionRule {
                                id: "mcp-auto".to_owned(),
                                tool: prepared.tool.clone(),
                                pattern: "*".to_owned(),
                                executor: None,
                            });
                        }
                        permissions::decide(
                            mode,
                            prepared.effect,
                            &rules,
                            PermissionRequest {
                                tool: &prepared.tool,
                                target: &prepared.target,
                                executor: prepared.executor,
                            },
                        )
                    };
                let result = match decision {
                    Decision::Allow => {
                        set_status(context, SessionStatus::RunningTool)?;
                        execute_call(self, prepared, &call.id, context, &assistant.id).await
                    }
                    Decision::Block { reason } => Err(reason),
                    Decision::Ask => {
                        self.ask(
                            prepared,
                            AskInput {
                                id: &call.id,
                                project_id: &project_id,
                                anchor: &assistant.id,
                            },
                            context,
                        )
                        .await
                    }
                };
                let (is_error, text, status) = match result {
                    Ok(execution) => {
                        exchange_changes.extend(execution.changes);
                        (false, truncate(&execution.text), ToolCallStatus::Succeeded)
                    }
                    Err(error)
                        if error == "denied by the user"
                            || error.starts_with("denied by the user:") =>
                    {
                        (true, truncate(&error), ToolCallStatus::Denied)
                    }
                    Err(error) => (true, truncate(&error), ToolCallStatus::Blocked),
                };
                if staged_skill {
                    context
                        .store
                        .delete_skill_change(context.session_id, &call.id)
                        .map_err(|_| storage_error())?;
                }
                append_tool_result(context, &mut request, &call.id, is_error, &text)?;
                if let Some(sink) = context.sink {
                    sink.on_event(CoreEvent::ToolFinished {
                        session_id: context.session_id.clone(),
                        tool_call_id: call.id.clone(),
                        status,
                    });
                }
                if call.name == "submit_plan" && !is_error {
                    return Ok(ToolLoopResult {
                        messages: request.messages,
                        final_text: text,
                        exchanges: exchange,
                        usage,
                    });
                }
            }
            if !exchange_changes.is_empty() && !context.cancel.is_cancelled() {
                let last_call_id = response.calls.last().map(|call| &call.id);
                if let Some(last_call_id) = last_call_id {
                    self.finish_quality(context, &mut request, exchange_changes, last_call_id)
                        .await;
                }
            }
        }
        Err(invalid("tool exchange limit reached"))
    }
}

impl ToolLoop {
    async fn finish_quality(
        &self,
        context: &mut ToolLoopContext<'_>,
        request: &mut tether_providers::ChatRequest,
        changes: Vec<PendingChange>,
        last_call_id: &tether_types::ToolCallId,
    ) {
        let result = Arc::clone(&self.quality).run(changes).await;
        let Ok(report) = result else {
            return;
        };
        if !report.report.findings.is_empty() {
            append_quality_text(context, request, &report);
        }
        emit_quality_event(context, last_call_id, &report);
    }

    async fn ask(
        &self,
        call: PreparedCall,
        input: AskInput<'_>,
        context: &mut ToolLoopContext<'_>,
    ) -> Result<ToolExecution, String> {
        let id = input.id;
        if context.approver.interactive() {
            set_status(context, SessionStatus::AwaitingApproval)
                .map_err(|_| "could not save approval state".to_owned())?;
            emit_approval_required(context, id, &call);
        }
        let request = ApprovalRequest {
            session_id: context.session_id.clone(),
            tool_call_id: id.clone(),
            tool: call.tool.clone(),
            target: call.display_target(),
        };
        let answer = context
            .approver
            .decide(request, context.cancel.clone())
            .await;
        let answer = if context.cancel.is_cancelled() {
            ApprovalAnswer::Deny {
                reason: Some("cancelled".into()),
            }
        } else {
            answer
        };
        record_approval(context, id, &call, &answer)
            .map_err(|_| "could not save approval".to_owned())?;
        match answer {
            ApprovalAnswer::Allow { always } | ApprovalAnswer::AllowFromNotification { always } => {
                save_always_rule(context, input.project_id, &call, always)?;
                set_status(context, SessionStatus::RunningTool)
                    .map_err(|_| "could not save approval state".to_owned())?;
                execute_call(self, call, id, context, input.anchor).await
            }
            ApprovalAnswer::Deny { reason } | ApprovalAnswer::DenyFromNotification { reason } => {
                set_status(context, SessionStatus::Requesting)
                    .map_err(|_| "could not save approval state".to_owned())?;
                Err(denial(reason))
            }
        }
    }
}

struct ToolExecution {
    text: String,
    changes: Vec<PendingChange>,
}

fn emit_approval_required(
    context: &ToolLoopContext<'_>,
    id: &tether_types::ToolCallId,
    call: &PreparedCall,
) {
    if let Some(sink) = context.sink {
        sink.on_event(CoreEvent::ApprovalRequired {
            session_id: context.session_id.clone(),
            tool_call_id: id.clone(),
            tool: call.tool.clone(),
            target: call.display_target(),
        });
    }
}

fn save_always_rule(
    context: &mut ToolLoopContext<'_>,
    project_id: &tether_types::ProjectId,
    call: &PreparedCall,
    always: bool,
) -> Result<(), String> {
    if !always {
        return Ok(());
    }
    let Some(rule) = permissions::rule_for(call.effect, &call.tool, &call.target, call.executor)
    else {
        return Ok(());
    };
    context
        .store
        .add_permission_rule(project_id, &rule, now_ms())
        .map_err(|_| "could not save permission rule".to_owned())?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn ask_mode() -> PermissionMode {
    PermissionMode::Ask
}

async fn execute_call(
    loop_runner: &ToolLoop,
    call: PreparedCall,
    id: &tether_types::ToolCallId,
    context: &mut ToolLoopContext<'_>,
    anchor: &tether_types::MessageId,
) -> Result<ToolExecution, String> {
    emit_tool_started(context, id);
    let session_id = context.session_id;
    let blobs = context.blobs;
    let sink = context.sink;
    let cancel = context.cancel.clone();
    let store = &mut *context.store;
    let guard = &loop_runner.guard;
    let call_tool_name = call.tool.clone();
    let mut old_texts = Vec::new();
    let mut before_edit = |paths: &[PathBuf]| {
        crate::capture_checkpoint(
            store,
            session_id,
            Some(anchor),
            guard,
            blobs,
            paths,
            now_ms(),
        )
        .map(|_| ())
        .map_err(|_| "could not save checkpoint".to_owned())?;
        old_texts.extend(
            paths
                .iter()
                .map(|path| quality::pending_change(path.clone(), quality::old_text(guard, path))),
        );
        Ok(())
    };
    let output = loop_runner
        .registry
        .execute(
            call,
            ExecuteContext {
                session_id,
                tool_call_id: id,
                cancel,
                sink,
                guard,
                before_edit: &mut before_edit,
            },
        )
        .await?;
    Ok(ToolExecution {
        text: output.text,
        changes: changes_for_tool(&call_tool_name, old_texts),
    })
}

fn emit_tool_started(context: &ToolLoopContext<'_>, id: &tether_types::ToolCallId) {
    if let Some(sink) = context.sink {
        sink.on_event(CoreEvent::ToolStarted {
            session_id: context.session_id.clone(),
            tool_call_id: id.clone(),
        });
    }
}

/// Keeps the prior file texts only for native file-edit tools.
fn changes_for_tool(tool: &str, old_texts: Vec<PendingChange>) -> Vec<PendingChange> {
    if is_native_file_edit(tool) {
        old_texts
    } else {
        Vec::new()
    }
}

fn is_native_file_edit(tool: &str) -> bool {
    matches!(
        tool,
        "write_file" | "edit_file" | "multi_edit" | "move_file" | "delete_file"
    )
}

fn append_quality_text(
    context: &mut ToolLoopContext<'_>,
    request: &mut tether_providers::ChatRequest,
    report: &GateReport,
) {
    let Some(message) = request.messages.last_mut() else {
        return;
    };
    let text = format!(
        "Quality report ({}):\n{}\nFix block findings before you commit.",
        report.report.render_summary(),
        report.report.render_for_model(20)
    );
    message.parts.push(MessagePart::Text { text });
    // Ignoring a failed save is safe: the report is already in this exchange's
    // request, and only its stored copy is lost.
    drop(
        context
            .store
            .replace_message_parts(&message.id, &message.parts),
    );
}

fn emit_quality_event(
    context: &ToolLoopContext<'_>,
    tool_call_id: &tether_types::ToolCallId,
    report: &GateReport,
) {
    let Some(sink) = context.sink else {
        return;
    };
    sink.on_event(CoreEvent::QualityReport {
        session_id: context.session_id.clone(),
        tool_call_id: tool_call_id.clone(),
        counts: tether_types::QualityCounts {
            info: count_u32(report.report.counts.info),
            warn: count_u32(report.report.counts.warn),
            block: count_u32(report.report.counts.block),
        },
        findings: quality::findings_to_records(&report.report.findings),
        skipped: report.skipped,
    });
}

fn count_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn save_cancelled_calls(
    context: &mut ToolLoopContext<'_>,
    pending: &[stream::Call],
) -> Result<(), CoreError> {
    for call in pending {
        context
            .store
            .append_message(
                context.session_id,
                Role::Tool,
                &[MessagePart::ToolResult {
                    tool_call_id: call.id.clone(),
                    is_error: true,
                    text: "tool cancelled before execution".into(),
                    result_ref: None,
                }],
                now_ms(),
            )
            .map_err(|_| storage_error())?;
    }
    Ok(())
}

fn append_tool_result(
    context: &mut ToolLoopContext<'_>,
    request: &mut tether_providers::ChatRequest,
    id: &tether_types::ToolCallId,
    is_error: bool,
    text: &str,
) -> Result<(), CoreError> {
    let tool_message = context
        .store
        .append_message(
            context.session_id,
            Role::Tool,
            &[MessagePart::ToolResult {
                tool_call_id: id.clone(),
                is_error,
                text: truncate(text),
                result_ref: None,
            }],
            now_ms(),
        )
        .map_err(|_| storage_error())?;
    request.messages.push(tool_message);
    Ok(())
}

fn set_status(context: &mut ToolLoopContext<'_>, status: SessionStatus) -> Result<(), CoreError> {
    context
        .store
        .set_session_status(context.session_id, status, now_ms())
        .map_err(|_| storage_error())?;
    if let Some(sink) = context.sink {
        sink.on_event(CoreEvent::SessionUpdated {
            session_id: context.session_id.clone(),
            status,
        });
    }
    Ok(())
}

fn record_approval(
    context: &mut ToolLoopContext<'_>,
    id: &tether_types::ToolCallId,
    call: &PreparedCall,
    answer: &ApprovalAnswer,
) -> Result<(), CoreError> {
    let (decision, reason, via) = match answer {
        ApprovalAnswer::Allow { .. } => ("allow", None, "app"),
        ApprovalAnswer::AllowFromNotification { .. } => ("allow", None, "notification"),
        ApprovalAnswer::Deny { reason } => ("deny", reason.clone(), "app"),
        ApprovalAnswer::DenyFromNotification { reason } => ("deny", reason.clone(), "notification"),
    };
    context
        .store
        .record_approval(&tether_store::ApprovalRecord {
            id: format!("{}-{}-{}", context.session_id, id, now_ms()),
            session_id: context.session_id.clone(),
            tool_call_id: id.clone(),
            tool: call.tool.clone(),
            target: call.display_target(),
            decision: decision.into(),
            reason,
            decided_at: now_ms(),
            via: via.into(),
        })
        .map_err(|_| storage_error())
}

fn denial(reason: Option<String>) -> String {
    reason.map_or_else(
        || "denied by the user".into(),
        |reason| format!("denied by the user: {reason}"),
    )
}

fn storage_error() -> CoreError {
    CoreError::Internal {
        message: "could not save the tool exchange".into(),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

fn truncate(value: &str) -> String {
    if value.chars().count() <= MAX_RESULT_CHARS {
        return value.to_owned();
    }
    let mut output: String = value.chars().take(MAX_RESULT_CHARS).collect();
    output.push_str(
        "\n[output truncated; use read_file with start_line and end_line, or grep with a narrower pattern]");
    output
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput {
        message: message.into(),
    }
}

fn cancelled() -> CoreError {
    CoreError::Cancelled {
        message: "tool loop interrupted".into(),
    }
}
