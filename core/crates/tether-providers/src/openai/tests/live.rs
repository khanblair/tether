//! Optional live OpenAI smoke test.

use tether_types::Role;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{collect, message, request, text};
use crate::{ChatProvider, OpenAiConfig, OpenAiProvider, StreamEvent};

/// The default model for the optional smoke test.
const DEFAULT_MODEL: &str = "gpt-5-mini";

#[tokio::test]
#[ignore = "calls the live OpenAI API and needs OPENAI_API_KEY"]
async fn live_smoke() {
    let api_key =
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| panic!("OPENAI_API_KEY is not set"));
    let model = std::env::var("OPENAI_SMOKE_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_owned());
    let provider = OpenAiProvider::new(reqwest::Client::new(), OpenAiConfig::new(api_key));
    let mut chat = request();
    chat.model_id = model;
    chat.messages = vec![message(
        Role::User,
        vec![text("Reply with the single word: ok")],
    )];
    chat.max_output_tokens = 64;
    let (sender, receiver) = mpsc::channel(64);
    let (result, events) = tokio::join!(
        provider.stream_chat(&chat, sender, CancellationToken::new()),
        collect(receiver)
    );
    assert_eq!(result, Ok(()));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, StreamEvent::TextDelta { .. }))
    );
    assert!(
        events
            .iter()
            .rev()
            .any(|event| matches!(event, StreamEvent::Usage(usage) if usage.output_tokens > 0))
    );
    assert!(matches!(events.last(), Some(StreamEvent::Finished { .. })));
}
