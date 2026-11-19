//! Optional live provider smoke tests.

use tether_types::Role;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{collect, message, request, text};
use crate::{ChatProvider, OpenAiCompatConfig, OpenAiCompatProvider, StreamEvent};

async fn run(config: OpenAiCompatConfig, model: String) {
    let provider = OpenAiCompatProvider::new(reqwest::Client::new(), config);
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
            .any(|event| matches!(event, StreamEvent::Usage(usage) if usage.output_tokens > 0))
    );
    assert!(matches!(events.last(), Some(StreamEvent::Finished { .. })));
}

#[tokio::test]
#[ignore = "calls the live DeepSeek API and needs DEEPSEEK_API_KEY"]
async fn live_smoke_deepseek() {
    let key =
        std::env::var("DEEPSEEK_API_KEY").unwrap_or_else(|_| panic!("DEEPSEEK_API_KEY is not set"));
    let model =
        std::env::var("DEEPSEEK_SMOKE_MODEL").unwrap_or_else(|_| "deepseek-chat".to_owned());
    run(OpenAiCompatConfig::deepseek(key), model).await;
}

#[tokio::test]
#[ignore = "calls the live OpenRouter API and needs OPENROUTER_API_KEY"]
async fn live_smoke_openrouter() {
    let key = std::env::var("OPENROUTER_API_KEY")
        .unwrap_or_else(|_| panic!("OPENROUTER_API_KEY is not set"));
    let model = std::env::var("OPENROUTER_SMOKE_MODEL")
        .unwrap_or_else(|_| "deepseek/deepseek-chat".to_owned());
    run(OpenAiCompatConfig::openrouter(key), model).await;
}
