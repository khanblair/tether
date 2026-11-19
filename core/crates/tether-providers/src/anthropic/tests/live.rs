//! A smoke test against the real Anthropic API. It runs only when asked for.

use std::time::Duration;

use tether_types::Role;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{collect, message, request, text};
use crate::{AnthropicConfig, AnthropicProvider, ChatProvider, ChatRequest, StreamEvent};

/// The model used when `ANTHROPIC_SMOKE_MODEL` is not set.
const DEFAULT_MODEL: &str = "claude-haiku-5-5";

fn smoke_request() -> ChatRequest {
    let model = std::env::var("ANTHROPIC_SMOKE_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_owned());
    ChatRequest {
        model_id: model,
        messages: vec![message(
            Role::User,
            vec![text("Reply with the single word: ok")],
        )],
        max_output_tokens: 32,
        ..request()
    }
}

/// Run with `cargo test -p tether-providers -- --ignored live_smoke` and `ANTHROPIC_API_KEY` set.
#[tokio::test]
#[ignore = "calls the live Anthropic API and needs ANTHROPIC_API_KEY"]
async fn live_smoke() {
    let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") else {
        panic!("ANTHROPIC_API_KEY is not set");
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();
    let provider = AnthropicProvider::new(client, AnthropicConfig::new(api_key));
    let chat = smoke_request();
    let (sender, receiver) = mpsc::channel(64);
    let stream = provider.stream_chat(&chat, sender, CancellationToken::new());
    let (result, events) = tokio::join!(stream, collect(receiver));

    assert_eq!(result, Ok(()));
    let has_text = events
        .iter()
        .any(|event| matches!(event, StreamEvent::TextDelta { .. }));
    let output_tokens = events.iter().rev().find_map(|event| match event {
        StreamEvent::Usage(usage) => Some(usage.output_tokens),
        _ => None,
    });
    assert!(has_text, "no text arrived");
    assert!(output_tokens > Some(0), "no output tokens were counted");
    assert!(matches!(events.last(), Some(StreamEvent::Finished { .. })));
}
