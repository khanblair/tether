//! Bounded provider and tool exchange with saved messages and edit checkpoints.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tether_providers::{
    ChatProvider, ChatRequest, ResponseUsage, StopReason, StreamEvent, ToolSpec,
};
use tether_store::{BlobStore, Store};
use tether_tools::{EditTools, ReadTools};
use tether_types::{CoreError, Message, MessagePart, Role, SessionId, ToolCallId, UsageSnapshot};
use tether_workspace::PathGuard;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

mod dispatch;
#[cfg(test)]
mod tests;

const MAX_EXCHANGES: usize = 12;
const MAX_CALLS_PER_EXCHANGE: usize = 16;
const MAX_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_RESULT_CHARS: usize = 16 * 1024;

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
    read: ReadTools,
    edit: EditTools,
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
    /// Called with a mutating tool and its exact target before it changes files.
    pub approve: &'a (dyn Fn(&str, &str) -> bool + Send + Sync),
}

impl ToolLoop {
    /// Binds all tool calls to one validated workspace.
    #[must_use]
    pub fn new(guard: PathGuard) -> Self {
        Self {
            read: ReadTools::new(guard.clone()),
            edit: EditTools::new(guard.clone()),
            guard,
        }
    }

    /// Tool contracts sent to the provider.
    #[must_use]
    pub fn specs() -> Vec<ToolSpec> {
        dispatch::specs()
    }

    /// Runs responses until the model ends its turn or the exchange cap is reached.
    /// The context's approval callback receives the exact target before a mutation.
    ///
    /// # Errors
    /// Returns provider, cancellation, or malformed-stream errors. Individual tool
    /// failures become tool results so the model can recover.
    #[expect(
        clippy::too_many_lines,
        clippy::excessive_nesting,
        reason = "the exchange loop keeps response and tool ordering visible"
    )]
    pub async fn run(
        &self,
        provider: Arc<dyn ChatProvider>,
        mut request: ChatRequest,
        context: &mut ToolLoopContext<'_>,
    ) -> Result<ToolLoopResult, CoreError> {
        request.tools = Self::specs();
        let mut usage = UsageSnapshot::default();
        for exchange in 1..=MAX_EXCHANGES {
            let response = response(
                Arc::clone(&provider),
                request.clone(),
                context.cancel.clone(),
            )
            .await?;
            usage.tokens_in = usage.tokens_in.saturating_add(response.usage.tokens_in);
            usage.tokens_out = usage.tokens_out.saturating_add(response.usage.tokens_out);
            let mut parts = Vec::new();
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
            let assistant = context
                .store
                .append_message(context.session_id, Role::Assistant, &parts, now_ms())
                .map_err(|_| storage_error())?;
            request.messages.push(assistant.clone());
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
            for call in response.calls {
                if context.cancel.is_cancelled() {
                    return Err(cancelled());
                }
                let mut before_edit = |paths: &[PathBuf]| {
                    crate::capture_checkpoint(
                        context.store,
                        context.session_id,
                        Some(&assistant.id),
                        &self.guard,
                        context.blobs,
                        paths,
                        now_ms(),
                    )
                    .map(|_| ())
                    .map_err(|_| "could not save checkpoint".to_owned())
                };
                let result = dispatch::run(
                    self,
                    &call.name,
                    &call.args,
                    context.approve,
                    &mut before_edit,
                );
                let (is_error, text) = match result {
                    Ok(text) => (false, truncate(&text)),
                    Err(error) => (true, truncate(&error)),
                };
                let tool_message = context
                    .store
                    .append_message(
                        context.session_id,
                        Role::Tool,
                        &[MessagePart::ToolResult {
                            tool_call_id: call.id,
                            is_error,
                            text,
                            result_ref: None,
                        }],
                        now_ms(),
                    )
                    .map_err(|_| storage_error())?;
                request.messages.push(tool_message);
            }
        }
        Err(invalid("tool exchange limit reached"))
    }
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

struct Call {
    id: ToolCallId,
    name: String,
    args: String,
    complete: bool,
}

struct Response {
    text: String,
    calls: Vec<Call>,
    stop: StopReason,
    usage: UsageSnapshot,
}

async fn response(
    provider: Arc<dyn ChatProvider>,
    request: ChatRequest,
    cancel: CancellationToken,
) -> Result<Response, CoreError> {
    let (tx, mut rx) = mpsc::channel(32);
    let token = cancel.clone();
    let task = tokio::spawn(async move { provider.stream_chat(&request, tx, token).await });
    let mut text = String::new();
    let mut calls: Vec<Call> = Vec::new();
    let mut stop = None;
    let mut usage = UsageSnapshot::default();
    loop {
        let event = tokio::select! {
            () = cancel.cancelled() => {
                task.abort();
                return Err(cancelled());
            }
            event = rx.recv() => event,
        };
        let Some(event) = event else { break };
        if stop.is_some() {
            task.abort();
            return Err(invalid("provider sent an event after finish"));
        }
        if let Err(error) = accept(event, &mut text, &mut calls, &mut stop, &mut usage) {
            task.abort();
            return Err(error);
        }
    }
    task.await
        .map_err(|_| invalid("provider task stopped"))?
        .map_err(CoreError::from)?;
    let stop = stop.ok_or_else(|| invalid("provider stream ended without finish"))?;
    if calls.iter().any(|call| !call.complete) {
        return Err(invalid("provider left a tool call unfinished"));
    }
    Ok(Response {
        text,
        calls,
        stop,
        usage,
    })
}

fn accept(
    event: StreamEvent,
    text: &mut String,
    calls: &mut Vec<Call>,
    stop: &mut Option<StopReason>,
    usage: &mut UsageSnapshot,
) -> Result<(), CoreError> {
    match event {
        StreamEvent::TextDelta { text: delta } => text.push_str(&delta),
        StreamEvent::ThinkingDelta { .. } => {}
        StreamEvent::Usage(value) => *usage = usage_snapshot(value),
        StreamEvent::ToolCallStarted { id, name } => {
            if calls.len() >= MAX_CALLS_PER_EXCHANGE || calls.iter().any(|call| call.id == id) {
                return Err(invalid("too many or duplicate tool calls"));
            }
            calls.push(Call {
                id,
                name,
                args: String::new(),
                complete: false,
            });
        }
        StreamEvent::ToolCallArgsDelta { id, json_fragment } => {
            let call = calls
                .iter_mut()
                .find(|call| call.id == id && !call.complete)
                .ok_or_else(|| invalid("tool arguments have no active call"))?;
            if call.args.len().saturating_add(json_fragment.len()) > MAX_ARGUMENT_BYTES {
                return Err(invalid("tool arguments exceed the limit"));
            }
            call.args.push_str(&json_fragment);
        }
        StreamEvent::ToolCallFinished { id } => {
            let call = calls
                .iter_mut()
                .find(|call| call.id == id && !call.complete)
                .ok_or_else(|| invalid("tool finish has no active call"))?;
            call.complete = true;
        }
        StreamEvent::Finished { stop_reason } => *stop = Some(stop_reason),
    }
    Ok(())
}

fn truncate(value: &str) -> String {
    if value.chars().count() <= MAX_RESULT_CHARS {
        return value.to_owned();
    }
    let mut output: String = value.chars().take(MAX_RESULT_CHARS).collect();
    output.push_str("\n[output truncated]");
    output
}

fn usage_snapshot(value: ResponseUsage) -> UsageSnapshot {
    UsageSnapshot {
        tokens_in: value
            .input_tokens
            .saturating_add(value.cache_read_tokens)
            .saturating_add(value.cache_write_tokens),
        tokens_out: value.output_tokens,
        cost_micros: 0,
    }
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
