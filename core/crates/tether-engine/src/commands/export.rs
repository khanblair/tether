use std::collections::HashMap;

use tether_types::{Message, MessagePart, Role, SessionId, ToolCallId};

/// The session fields needed for a deterministic Markdown export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSession {
    /// Session id.
    pub id: SessionId,
    /// Session title.
    pub title: String,
    /// Model id.
    pub model: String,
    /// Transcript messages in sequence order.
    pub messages: Vec<Message>,
}

/// Exports a session to Markdown, omitting thinking parts.
#[must_use]
pub fn export_session_markdown(session: &ExportSession) -> String {
    export_session_messages_markdown(&session.title, &session.model, &session.messages)
}

/// Exports fixed session fields and messages to Markdown.
#[must_use]
pub fn export_session_messages_markdown(title: &str, model: &str, messages: &[Message]) -> String {
    let results = tool_results(messages);
    let mut output = format!("# {title}\n\nModel: {model}\n");
    for message in messages.iter().filter(|message| !message.rewound) {
        let heading = match message.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::Tool => "Tool",
            Role::SystemNote => "System note",
        };
        output.push_str(&format!("\n## {heading}\n"));
        for part in &message.parts {
            render_part(&mut output, part, &results);
        }
    }
    output
}

fn render_part(output: &mut String, part: &MessagePart, results: &HashMap<ToolCallId, String>) {
    match part {
        MessagePart::Text { text } => {
            output.push_str(text);
            output.push('\n');
        }
        MessagePart::ToolCall {
            id,
            tool,
            args_json,
        } => {
            output.push_str("\n```tool\n");
            output.push_str("target: ");
            output.push_str(tool);
            output.push_str("\nargs: ");
            output.push_str(args_json);
            if let Some(result) = results.get(id) {
                output.push_str("\nresult: ");
                output.push_str(&truncate_result(result));
            }
            output.push_str("\n```\n");
        }
        MessagePart::ToolResult { text, .. } => {
            if !results.values().any(|result| result == text) {
                output.push_str("\n```tool-result\n");
                output.push_str(&truncate_result(text));
                output.push_str("\n```\n");
            }
        }
        MessagePart::ImageRef { .. }
        | MessagePart::Thinking { .. }
        | MessagePart::CompactionSummary { .. } => {}
    }
}

fn tool_results(messages: &[Message]) -> HashMap<ToolCallId, String> {
    messages
        .iter()
        .flat_map(|message| message.parts.iter())
        .filter_map(|part| {
            let MessagePart::ToolResult {
                tool_call_id, text, ..
            } = part
            else {
                return None;
            };
            Some((tool_call_id.clone(), text.clone()))
        })
        .collect()
}

fn truncate_result(text: &str) -> String {
    text.chars().take(2_000).collect()
}
