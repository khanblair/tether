//! View types for messages, session settings, and usage.

use tether_types::{
    BudgetScope, ConnectionStatus, Executor, Message, MessagePart, ModelRef, PermissionMode,
    Provider, Role, SessionStatus, ToolCallStatus, UsageSnapshot,
};

/// Declares a field-less view enum and its conversion from the engine type.
macro_rules! mirror_enum {
    ($(#[$meta:meta])* $view:ident <=> $engine:ident { $($variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
        pub enum $view {
            $($variant),+
        }

        impl From<$engine> for $view {
            fn from(value: $engine) -> Self {
                match value {
                    $($engine::$variant => Self::$variant),+
                }
            }
        }
    };
}

mirror_enum!(
    /// FFI view of [`Role`].
    RoleView <=> Role { User, Assistant, Tool, SystemNote }
);
mirror_enum!(
    /// FFI view of [`PermissionMode`].
    PermissionModeView <=> PermissionMode { Plan, Ask, AcceptEdits, Auto, Bypass }
);
mirror_enum!(
    /// FFI view of [`Executor`].
    ExecutorView <=> Executor { Sandbox, Linux, Ci }
);
mirror_enum!(
    /// FFI view of [`Provider`].
    ProviderView <=> Provider { Anthropic, OpenAi, Gemini, DeepSeek, OpenRouter }
);

/// Maps the FFI provider to the engine's provider. Each variant maps one to one.
impl From<ProviderView> for Provider {
    fn from(view: ProviderView) -> Self {
        match view {
            ProviderView::Anthropic => Self::Anthropic,
            ProviderView::OpenAi => Self::OpenAi,
            ProviderView::Gemini => Self::Gemini,
            ProviderView::DeepSeek => Self::DeepSeek,
            ProviderView::OpenRouter => Self::OpenRouter,
        }
    }
}

/// Maps the selected permission mode to the saved session mode.
impl From<PermissionModeView> for PermissionMode {
    fn from(view: PermissionModeView) -> Self {
        match view {
            PermissionModeView::Plan => Self::Plan,
            PermissionModeView::Ask => Self::Ask,
            PermissionModeView::AcceptEdits => Self::AcceptEdits,
            PermissionModeView::Auto => Self::Auto,
            PermissionModeView::Bypass => Self::Bypass,
        }
    }
}
mirror_enum!(
    /// FFI view of [`SessionStatus`].
    SessionStatusView <=> SessionStatus {
        Idle, Requesting, Streaming, CheckingTool, RunningTool, AwaitingApproval, Paused,
        Interrupted, Failed
    }
);
mirror_enum!(
    /// FFI view of [`ToolCallStatus`].
    ToolCallStatusView <=> ToolCallStatus {
        Proposed, Allowed, AwaitingApproval, Running, Denied, Blocked, Succeeded, Failed,
        TimedOut, Cancelled
    }
);
mirror_enum!(
    /// FFI view of [`ConnectionStatus`].
    ConnectionStatusView <=> ConnectionStatus {
        Unset, Testing, Ok, BadKey, NetworkError, Blocked
    }
);
mirror_enum!(
    /// FFI view of [`BudgetScope`].
    BudgetScopeView <=> BudgetScope { Session, Day }
);

/// FFI view of [`ModelRef`].
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ModelRefView {
    /// The provider that serves the model.
    pub provider: ProviderView,
    /// The provider's own model identifier.
    pub model_id: String,
}

impl From<ModelRef> for ModelRefView {
    fn from(model: ModelRef) -> Self {
        Self {
            provider: model.provider.into(),
            model_id: model.model_id,
        }
    }
}

/// FFI view of [`UsageSnapshot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct UsageSnapshotView {
    /// Input tokens sent to the provider.
    pub tokens_in: u64,
    /// Output tokens received from the provider.
    pub tokens_out: u64,
    /// Cost in millionths of a US dollar.
    pub cost_micros: u64,
}

impl From<UsageSnapshot> for UsageSnapshotView {
    fn from(usage: UsageSnapshot) -> Self {
        Self {
            tokens_in: usage.tokens_in,
            tokens_out: usage.tokens_out,
            cost_micros: usage.cost_micros,
        }
    }
}

/// FFI view of [`MessagePart`]. Ids are plain strings.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum MessagePartView {
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
        id: String,
        /// Tool name.
        tool: String,
        /// Arguments as JSON text.
        args_json: String,
    },
    /// The result of a tool call.
    ToolResult {
        /// The call this result answers.
        tool_call_id: String,
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

impl From<MessagePart> for MessagePartView {
    fn from(part: MessagePart) -> Self {
        match part {
            MessagePart::Text { text } => Self::Text { text },
            MessagePart::ImageRef {
                blob_hash,
                media_type,
            } => Self::ImageRef {
                blob_hash,
                media_type,
            },
            MessagePart::ToolCall {
                id,
                tool,
                args_json,
            } => Self::ToolCall {
                id: id.into(),
                tool,
                args_json,
            },
            MessagePart::ToolResult {
                tool_call_id,
                is_error,
                text,
                result_ref,
            } => Self::ToolResult {
                tool_call_id: tool_call_id.into(),
                is_error,
                text,
                result_ref,
            },
            MessagePart::Thinking { text } => Self::Thinking { text },
            MessagePart::CompactionSummary { text } => Self::CompactionSummary { text },
        }
    }
}

/// FFI view of [`Message`].
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MessageView {
    /// Message identifier.
    pub id: String,
    /// The session the message belongs to.
    pub session_id: String,
    /// Who wrote it.
    pub role: RoleView,
    /// Position in the session, starting at 0.
    pub seq: u32,
    /// Creation time in Unix milliseconds.
    pub created_at_ms: i64,
    /// True when a rewind removed this message from the live transcript.
    pub rewound: bool,
    /// The parts in order.
    pub parts: Vec<MessagePartView>,
}

impl From<Message> for MessageView {
    fn from(message: Message) -> Self {
        Self {
            id: message.id.into(),
            session_id: message.session_id.into(),
            role: message.role.into(),
            seq: message.seq,
            created_at_ms: message.created_at_ms,
            rewound: message.rewound,
            parts: message.parts.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use tether_types::{MessageId, SessionId, ToolCallId};

    use super::*;

    #[test]
    fn enum_view_keeps_the_variant() {
        assert_eq!(
            PermissionModeView::from(PermissionMode::AcceptEdits),
            PermissionModeView::AcceptEdits
        );
        assert_eq!(
            ToolCallStatusView::from(ToolCallStatus::TimedOut),
            ToolCallStatusView::TimedOut
        );
    }

    #[test]
    fn every_permission_mode_maps_to_and_from_the_store_type() {
        for mode in [
            PermissionMode::Plan,
            PermissionMode::Ask,
            PermissionMode::AcceptEdits,
            PermissionMode::Auto,
            PermissionMode::Bypass,
        ] {
            let view = PermissionModeView::from(mode);
            assert_eq!(PermissionMode::from(view), mode);
        }
    }

    #[test]
    fn message_view_turns_ids_into_strings() {
        let view = MessageView::from(Message {
            id: MessageId::new("m1"),
            session_id: SessionId::new("s1"),
            role: Role::Assistant,
            seq: 2,
            created_at_ms: 1_700_000_000_000,
            rewound: false,
            parts: vec![MessagePart::ToolCall {
                id: ToolCallId::new("c1"),
                tool: "read_file".into(),
                args_json: "{}".into(),
            }],
        });
        assert_eq!(view.id, "m1");
        assert_eq!(view.session_id, "s1");
        assert_eq!(
            view.parts,
            vec![MessagePartView::ToolCall {
                id: "c1".into(),
                tool: "read_file".into(),
                args_json: "{}".into()
            }]
        );
    }

    #[test]
    fn every_part_kind_converts_with_its_ids_as_strings() {
        let parts = vec![
            MessagePart::Text { text: "t".into() },
            MessagePart::ImageRef {
                blob_hash: "h".into(),
                media_type: "image/png".into(),
            },
            MessagePart::ToolResult {
                tool_call_id: ToolCallId::new("c1"),
                is_error: true,
                text: "boom".into(),
                result_ref: Some("blob".into()),
            },
            MessagePart::Thinking { text: "hm".into() },
            MessagePart::CompactionSummary { text: "s".into() },
        ];
        let views: Vec<MessagePartView> = parts.into_iter().map(Into::into).collect();
        assert_eq!(
            views,
            vec![
                MessagePartView::Text { text: "t".into() },
                MessagePartView::ImageRef {
                    blob_hash: "h".into(),
                    media_type: "image/png".into()
                },
                MessagePartView::ToolResult {
                    tool_call_id: "c1".into(),
                    is_error: true,
                    text: "boom".into(),
                    result_ref: Some("blob".into())
                },
                MessagePartView::Thinking { text: "hm".into() },
                MessagePartView::CompactionSummary { text: "s".into() },
            ]
        );
    }
}
