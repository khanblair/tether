//! Context accounting and message shaping for long sessions.

use tether_providers::ToolSpec;
use tether_types::{Message, MessagePart, Role};

use crate::prompt::{PromptSection, estimate_tokens};

/// One named part of the current model context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextSection {
    /// Display name for the section.
    pub name: String,
    /// Approximate token count.
    pub approx_tokens: u32,
}

/// The current context estimate and its section breakdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextBreakdown {
    /// Model context capacity when the catalog reports it.
    pub window: Option<u32>,
    /// Last reported input tokens, or the local estimate.
    pub used_tokens: u32,
    /// Percentage of the context window used when the window is known.
    pub percent: Option<u8>,
    /// Prompt, tools, attachments, and conversation sections.
    pub sections: Vec<ContextSection>,
}

/// Counts the current prompt and conversation using the provider's last input total when given.
#[must_use]
pub fn breakdown(
    window: Option<u32>,
    prompt: &[PromptSection],
    tools: &[ToolSpec],
    messages: &[Message],
    reported_input_tokens: Option<u32>,
) -> ContextBreakdown {
    let mut sections = prompt
        .iter()
        .map(|section| ContextSection {
            name: section.name.clone(),
            approx_tokens: section.approx_tokens,
        })
        .collect::<Vec<_>>();
    sections.push(tool_section(tools));
    sections.push(attached_section(messages));
    sections.extend(role_sections(messages));
    let estimate = sections.iter().map(|section| section.approx_tokens).sum();
    let used_tokens = reported_input_tokens.unwrap_or(estimate);
    let percent = window.map(|window| {
        let ratio = u64::from(used_tokens).saturating_mul(100) / u64::from(window.max(1));
        u8::try_from(ratio.min(100)).unwrap_or(100)
    });
    if let Some(reported) = reported_input_tokens
        && estimate > 0
        && reported != estimate
    {
        distribute(&mut sections, reported, estimate);
    }
    ContextBreakdown {
        window,
        used_tokens,
        percent,
        sections,
    }
}

fn tool_section(tools: &[ToolSpec]) -> ContextSection {
    let tool_text = tools
        .iter()
        .flat_map(|tool| {
            [
                tool.name.as_str(),
                tool.description.as_str(),
                tool.input_schema_json.as_str(),
            ]
        })
        .collect::<String>();
    ContextSection {
        name: "Tool definitions".into(),
        approx_tokens: estimate_tokens(&tool_text),
    }
}

fn attached_section(messages: &[Message]) -> ContextSection {
    let attached = messages
        .iter()
        .flat_map(|message| message.parts.iter())
        .filter_map(|part| match part {
            MessagePart::Text { text } if text.contains("Attached file") => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    ContextSection {
        name: "Attached files".into(),
        approx_tokens: estimate_tokens(&attached),
    }
}

fn role_sections(messages: &[Message]) -> Vec<ContextSection> {
    [Role::User, Role::Assistant, Role::Tool]
        .into_iter()
        .map(|role| {
            let text = messages
                .iter()
                .filter(|message| message.role == role)
                .flat_map(|message| message.parts.iter())
                .map(part_text)
                .collect::<String>();
            ContextSection {
                name: role.as_str().to_owned(),
                approx_tokens: estimate_tokens(&text),
            }
        })
        .collect()
}

fn part_text(part: &MessagePart) -> &str {
    match part {
        MessagePart::Text { text }
        | MessagePart::Thinking { text }
        | MessagePart::CompactionSummary { text }
        | MessagePart::ToolResult { text, .. } => text,
        MessagePart::ImageRef { .. } | MessagePart::ToolCall { .. } => "",
    }
}

fn distribute(sections: &mut [ContextSection], reported: u32, estimate: u32) {
    let mut assigned = 0;
    let length = sections.len();
    for (index, section) in sections.iter_mut().enumerate() {
        if index + 1 == length {
            section.approx_tokens = reported.saturating_sub(assigned);
        } else {
            let scaled =
                u64::from(section.approx_tokens) * u64::from(reported) / u64::from(estimate.max(1));
            section.approx_tokens = u32::try_from(scaled).unwrap_or(u32::MAX);
            assigned = assigned.saturating_add(section.approx_tokens);
        }
    }
}

/// Estimates the size of a message list.
#[must_use]
pub fn estimate_messages(messages: &[Message]) -> u32 {
    messages
        .iter()
        .flat_map(|message| message.parts.iter())
        .map(part_text)
        .map(estimate_tokens)
        .sum()
}

#[cfg(test)]
mod tests {
    use tether_types::{MessageId, SessionId};

    use super::*;

    fn message(role: Role, text: &str) -> Message {
        Message {
            id: MessageId::new(text),
            session_id: SessionId::new("session"),
            role,
            seq: 0,
            created_at_ms: 0,
            rewound: false,
            parts: vec![MessagePart::Text { text: text.into() }],
        }
    }

    #[test]
    fn breakdown_uses_the_estimate_without_reported_usage() {
        let result = breakdown(
            Some(100),
            &[PromptSection {
                name: "Instructions".into(),
                chars: 4,
                approx_tokens: 1,
            }],
            &[],
            &[message(Role::User, "12345678")],
            None,
        );

        assert_eq!(result.used_tokens, 3);
        assert_eq!(result.percent, Some(3));
        assert_eq!(result.sections[0].name, "Instructions");
        assert_eq!(result.sections[3].name, "user");
    }

    #[test]
    fn breakdown_distributes_reported_usage_over_estimated_sections() {
        let result = breakdown(
            None,
            &[PromptSection {
                name: "Instructions".into(),
                chars: 4,
                approx_tokens: 1,
            }],
            &[],
            &[message(Role::User, "12345678")],
            Some(20),
        );

        assert_eq!(result.used_tokens, 20);
        assert_eq!(
            result
                .sections
                .iter()
                .map(|section| section.approx_tokens)
                .collect::<Vec<_>>(),
            vec![6, 0, 0, 13, 0, 1]
        );
    }
}
