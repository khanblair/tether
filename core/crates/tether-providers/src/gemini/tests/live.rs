//! Optional live Gemini smoke test.

use tether_types::Role;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{collect, message, request, text};
use crate::{ChatProvider, GeminiConfig, GeminiProvider, StreamEvent};

const DEFAULT_MODEL: &str = "gemini-2.5-flash";

#[tokio::test]
#[ignore = "calls the live Gemini API and needs GEMINI_API_KEY"]
async fn live_smoke() {
    let api_key =
        std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| panic!("GEMINI_API_KEY is not set"));
    let model = std::env::var("GEMINI_SMOKE_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_owned());
    let provider = GeminiProvider::new(reqwest::Client::new(), GeminiConfig::new(api_key));
    let mut chat = request();
    chat.model_id = model;
    chat.messages = vec![message(
        Role::User,
        vec![text("Reply with the single word: ok")],
    )];
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
            .any(|event| matches!(event, StreamEvent::Usage(usage) if usage.output_tokens > 0))
    );
    assert!(matches!(events.last(), Some(StreamEvent::Finished { .. })));
}
