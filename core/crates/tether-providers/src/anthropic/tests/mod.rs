//! Tests for the Anthropic adapter.
//!
//! The fixtures in `fixtures/provider-streams/anthropic-*.sse` are hand-written from Anthropic's
//! documented streaming format. They are not recorded from live traffic.

mod body;
mod http;
mod live;
mod mapping;
use crate::test_support as server;

use tether_types::{Message, MessageId, MessagePart, Role, SessionId};
use tokio::sync::mpsc;

use crate::{ChatRequest, StreamEvent};

/// The fake key used by every test. It must never appear in an error or in debug output.
const KEY: &str = "test-key-123";

/// A request with one user message and no system prompt, tools, or reasoning.
fn request() -> ChatRequest {
    ChatRequest {
        model_id: "claude-test".to_owned(),
        system: String::new(),
        messages: vec![message(Role::User, vec![text("Hello")])],
        tools: Vec::new(),
        max_output_tokens: 64,
        reasoning: None,
    }
}

/// A message with the given role and parts.
fn message(role: Role, parts: Vec<MessagePart>) -> Message {
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

/// A text part.
fn text(text: &str) -> MessagePart {
    MessagePart::Text {
        text: text.to_owned(),
    }
}

/// Collects events until the sender is dropped.
async fn collect(mut receiver: mpsc::Receiver<StreamEvent>) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    while let Some(event) = receiver.recv().await {
        events.push(event);
    }
    events
}
