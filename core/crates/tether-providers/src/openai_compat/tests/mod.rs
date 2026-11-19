//! Tests for the OpenAI-compatible Chat Completions adapter.
//!
//! These fixtures are hand-written from the providers' documented streaming format, not recorded.

mod body;
mod http;
mod live;
mod mapping;

use tether_types::{Message, MessageId, MessagePart, Role, SessionId};
use tokio::sync::mpsc;

use crate::{ChatRequest, StreamEvent};

pub(super) const KEY: &str = "test-key-123";

pub(super) fn request() -> ChatRequest {
    ChatRequest {
        model_id: "test-model".to_owned(),
        system: String::new(),
        messages: vec![message(Role::User, vec![text("Hello")])],
        tools: Vec::new(),
        max_output_tokens: 64,
        reasoning: None,
    }
}

pub(super) fn message(role: Role, parts: Vec<MessagePart>) -> Message {
    Message {
        id: MessageId::new("msg"),
        session_id: SessionId::new("session"),
        role,
        seq: 0,
        created_at_ms: 0,
        rewound: false,
        parts,
    }
}

pub(super) fn text(value: &str) -> MessagePart {
    MessagePart::Text {
        text: value.to_owned(),
    }
}

pub(super) async fn collect(mut receiver: mpsc::Receiver<StreamEvent>) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    while let Some(event) = receiver.recv().await {
        events.push(event);
    }
    events
}
