//! Bounded provider and tool exchange for a workspace fixture.

use std::sync::Arc;

use tether_providers::{ChatProvider, ChatRequest, StopReason, StreamEvent, ToolSpec};
use tether_tools::{EditTools, ReadTools};
use tether_types::{CoreError, Message, MessageId, MessagePart, Role, SessionId, ToolCallId};
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

/// The completed in-memory exchange, ready for persistence by the session actor.
#[derive(Debug)]
pub struct ToolLoopResult {
    /// Original messages and every generated assistant and tool message.
    pub messages: Vec<Message>,
    /// The final assistant reply.
    pub final_text: String,
    /// Number of provider responses.
    pub exchanges: usize,
}

/// Runs a guarded set of file tools against successive model responses.
pub struct ToolLoop {
    read: ReadTools,
    edit: EditTools,
}

impl ToolLoop {
    /// Binds all tool calls to one validated workspace.
    #[must_use]
    pub fn new(guard: PathGuard) -> Self {
        Self {
            read: ReadTools::new(guard.clone()),
            edit: EditTools::new(guard),
        }
    }

    /// Tool contracts sent to the provider.
    #[must_use]
    pub fn specs() -> Vec<ToolSpec> {
        dispatch::specs()
    }

    /// Runs responses until the model ends its turn or the exchange cap is reached.
    /// `approve` receives the tool name and exact target before a file mutation.
    ///
    /// # Errors
    /// Returns provider, cancellation, or malformed-stream errors. Individual tool
    /// failures become tool results so the model can recover.
    #[expect(clippy::too_many_lines, reason = "the exchange loop keeps response and tool ordering visible")]
    pub async fn run(
        &self,
        provider: Arc<dyn ChatProvider>,
        mut request: ChatRequest,
        session_id: SessionId,
        cancel: CancellationToken,
        approve: impl Fn(&str, &str) -> bool,
    ) -> Result<ToolLoopResult, CoreError> {
        request.tools = Self::specs();
        for exchange in 1..=MAX_EXCHANGES {
            let response = response(Arc::clone(&provider), request.clone(), cancel.clone()).await?;
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
            append_message(&mut request.messages, &session_id, Role::Assistant, parts, exchange);
            if response.stop != StopReason::ToolUse {
                return Ok(ToolLoopResult {
                    messages: request.messages,
                    final_text: response.text,
                    exchanges: exchange,
                });
            }
            if response.calls.is_empty() {
                return Err(invalid("provider requested tools without a call"));
            }
            for call in response.calls {
                if cancel.is_cancelled() {
                    return Err(cancelled());
                }
                let result = dispatch::run(self, &call.name, &call.args, &approve);
                let (is_error, text) = match result {
                    Ok(text) => (false, truncate(&text)),
                    Err(error) => (true, truncate(&error)),
                };
                append_message(
                    &mut request.messages,
                    &session_id,
                    Role::Tool,
                    vec![MessagePart::ToolResult {
                        tool_call_id: call.id,
                        is_error,
                        text,
                        result_ref: None,
                    }],
                    exchange,
                );
            }
        }
        Err(invalid("tool exchange limit reached"))
    }
}

fn append_message(
    messages: &mut Vec<Message>,
    session_id: &SessionId,
    role: Role,
    parts: Vec<MessagePart>,
    exchange: usize,
) {
    let seq = u32::try_from(messages.len()).unwrap_or(u32::MAX);
    messages.push(Message {
        id: MessageId::new(format!("tool-{exchange}-{seq}")),
        session_id: session_id.clone(),
        role,
        seq,
        created_at_ms: 0,
        rewound: false,
        parts,
    });
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
        if let Err(error) = accept(event, &mut text, &mut calls, &mut stop) {
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
    Ok(Response { text, calls, stop })
}

fn accept(
    event: StreamEvent,
    text: &mut String,
    calls: &mut Vec<Call>,
    stop: &mut Option<StopReason>,
) -> Result<(), CoreError> {
    match event {
        StreamEvent::TextDelta { text: delta } => text.push_str(&delta),
        StreamEvent::ThinkingDelta { .. } | StreamEvent::Usage(_) => {}
        StreamEvent::ToolCallStarted { id, name } => {
            if calls.len() >= MAX_CALLS_PER_EXCHANGE || calls.iter().any(|call| call.id == id) {
                return Err(invalid("too many or duplicate tool calls"));
            }
            calls.push(Call { id, name, args: String::new(), complete: false });
        }
        StreamEvent::ToolCallArgsDelta { id, json_fragment } => {
            let call = calls.iter_mut().find(|call| call.id == id && !call.complete)
                .ok_or_else(|| invalid("tool arguments have no active call"))?;
            if call.args.len().saturating_add(json_fragment.len()) > MAX_ARGUMENT_BYTES {
                return Err(invalid("tool arguments exceed the limit"));
            }
            call.args.push_str(&json_fragment);
        }
        StreamEvent::ToolCallFinished { id } => {
            let call = calls.iter_mut().find(|call| call.id == id && !call.complete)
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

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput { message: message.into() }
}

fn cancelled() -> CoreError {
    CoreError::Cancelled { message: "tool loop interrupted".into() }
}
