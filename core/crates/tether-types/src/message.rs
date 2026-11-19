//! Messages and the parts a message is made of.

use serde::{Deserialize, Serialize};

use crate::text::text_enum;
use crate::{MessageId, SessionId, ToolCallId};

/// Who wrote a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The user.
    User,
    /// The model.
    Assistant,
    /// A tool result sent back to the model.
    Tool,
    /// A note from Tether, such as a rewind marker. It is never sent to the model.
    SystemNote,
}

text_enum!(Role {
    User => "user",
    Assistant => "assistant",
    Tool => "tool",
    SystemNote => "system_note",
});

/// One piece of a message. The store keeps each part as `body_json` with its `kind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessagePart {
    /// Plain text.
    Text {
        /// The text.
        text: String,
    },
    /// An image held in the blob store.
    ImageRef {
        /// BLAKE3 hash that names the blob.
        blob_hash: String,
        /// Media type, such as `image/png`.
        media_type: String,
    },
    /// A tool call the model proposed.
    ToolCall {
        /// The call this part describes.
        id: ToolCallId,
        /// Tool name, such as `edit_file`.
        tool: String,
        /// Arguments as a JSON object, kept as text so the core does not reshape them.
        args_json: String,
    },
    /// The result of a tool call.
    ToolResult {
        /// The call this result answers.
        tool_call_id: ToolCallId,
        /// True when the tool failed.
        is_error: bool,
        /// Result text, cut when it is large.
        text: String,
        /// Blob hash of the full output when the text was cut.
        result_ref: Option<String>,
    },
    /// Model reasoning shown to the user.
    Thinking {
        /// The reasoning text.
        text: String,
    },
    /// A summary that replaced older messages during compaction.
    CompactionSummary {
        /// The summary text.
        text: String,
    },
}

impl MessagePart {
    /// The part kind kept in the `kind` column. It equals the `kind` tag in the JSON form.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text",
            Self::ImageRef { .. } => "image_ref",
            Self::ToolCall { .. } => "tool_call",
            Self::ToolResult { .. } => "tool_result",
            Self::Thinking { .. } => "thinking",
            Self::CompactionSummary { .. } => "compaction_summary",
        }
    }
}

/// One message in a session transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Message identifier.
    pub id: MessageId,
    /// The session the message belongs to.
    pub session_id: SessionId,
    /// Who wrote it.
    pub role: Role,
    /// Position in the session, starting at 0.
    pub seq: u32,
    /// Creation time in Unix milliseconds.
    pub created_at_ms: i64,
    /// True when a rewind removed this message from the live transcript.
    pub rewound: bool,
    /// The parts in order.
    pub parts: Vec<MessagePart>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_json_carries_its_kind() {
        let part = MessagePart::ToolCall {
            id: ToolCallId::new("01J0CALL"),
            tool: "read_file".into(),
            args_json: "{\"path\":\"a.rs\"}".into(),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("\"kind\":\"tool_call\""));
        assert_eq!(serde_json::from_str::<MessagePart>(&json).unwrap(), part);
    }

    #[test]
    fn kind_equals_the_json_tag_for_every_part() {
        let id = ToolCallId::new("c1");
        let parts = [
            MessagePart::Text { text: "t".into() },
            MessagePart::ImageRef {
                blob_hash: "h".into(),
                media_type: "image/png".into(),
            },
            MessagePart::ToolCall {
                id: id.clone(),
                tool: "x".into(),
                args_json: "{}".into(),
            },
            MessagePart::ToolResult {
                tool_call_id: id,
                is_error: false,
                text: String::new(),
                result_ref: None,
            },
            MessagePart::Thinking { text: "t".into() },
            MessagePart::CompactionSummary { text: "s".into() },
        ];
        for part in parts {
            let json = serde_json::to_value(&part).unwrap();
            assert_eq!(json["kind"], part.kind());
        }
    }

    #[test]
    fn roles_keep_a_stable_text_form() {
        for &role in Role::ALL {
            assert_eq!(role.as_str().parse::<Role>().unwrap(), role);
        }
        assert_eq!(Role::SystemNote.as_str(), "system_note");
    }

    #[test]
    fn tool_result_keeps_an_optional_blob_reference() {
        let part = MessagePart::ToolResult {
            tool_call_id: ToolCallId::new("01J0CALL"),
            is_error: false,
            text: "cut".into(),
            result_ref: Some("abc123".into()),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert_eq!(serde_json::from_str::<MessagePart>(&json).unwrap(), part);
    }
}
