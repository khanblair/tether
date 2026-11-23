//! Context compaction: summarize old messages, count the cost, and store the result.

use std::sync::Arc;

use tether_providers::{ChatRequest, ToolSpec};
use tether_types::{CoreError, CoreEvent, MessagePart, Role};
use tokio_util::sync::CancellationToken;

use super::SessionActor;

pub(super) async fn compact_session(
    actor: &mut SessionActor,
    focus: Option<String>,
) -> Result<(), CoreError> {
    let messages = actor
        .store
        .messages(&actor.session.id)
        .map_err(super::store_error)?
        .into_iter()
        .filter(|message| !message.rewound)
        .collect::<Vec<_>>();
    let Some(cut) = compaction_cut(&messages) else {
        return Ok(());
    };
    let old = &messages[..cut];
    let (provider, request) = summary_call(actor, old, focus.as_deref())?;
    // The summary is a provider request like any other: it waits at a budget limit, an
    // interrupt can stop it, and its tokens and cost count toward the session and the day.
    let cancel = CancellationToken::new();
    if let Ok(mut active) = actor.active.lock() {
        *active = Some(cancel.clone());
    }
    let summary = summarize_within_budget(actor, provider, request, &cancel).await;
    if let Ok(mut active) = actor.active.lock() {
        *active = None;
    }
    let summary = summary?;
    store_compaction(actor, old, &summary, focus)
}

/// Checks the budget, streams the summary, and records the response usage even on failure.
async fn summarize_within_budget(
    actor: &mut SessionActor,
    provider: Arc<dyn tether_providers::ChatProvider>,
    request: ChatRequest,
    cancel: &CancellationToken,
) -> Result<String, CoreError> {
    let target = crate::budget::CheckTarget {
        session: &actor.session,
        sink: actor.sink.as_ref(),
        cancel,
    };
    actor
        .budget
        .check(&mut actor.store, target, super::now_ms())
        .await?;
    let model_id = request.model_id.clone();
    let (text, usage) = summary_request(provider, request, cancel.clone()).await;
    record_summary_usage(actor, &model_id, usage)?;
    text
}

/// Prices and saves the tokens a summary request used, then reports the new session totals.
fn record_summary_usage(
    actor: &mut SessionActor,
    model_id: &str,
    usage: tether_providers::ResponseUsage,
) -> Result<(), CoreError> {
    if usage == tether_providers::ResponseUsage::default() {
        return Ok(());
    }
    let priced = crate::usage::price_response(&actor.store, &actor.session.id, model_id, usage)
        .map_err(super::store_error)?;
    actor
        .store
        .record_response_usage(&tether_store::ResponseUsageRecord {
            session_id: actor.session.id.clone(),
            provider: actor.session.provider,
            model_id: actor.session.model_id.clone(),
            project_id: actor.session.project_id.as_str().into(),
            day: crate::usage::utc_day_from_ms(super::now_ms()),
            usage: priced.snapshot,
            price_known: priced.price_known,
            now_ms: super::now_ms(),
        })
        .map_err(super::store_error)?;
    let totals = actor
        .store
        .session_usage(&actor.session.id)
        .map_err(super::store_error)?;
    actor.sink.on_event(CoreEvent::UsageUpdated {
        session_id: actor.session.id.clone(),
        usage: totals,
    });
    Ok(())
}

/// Returns how many leading messages to compact, or `None` when the history is too short.
fn compaction_cut(messages: &[tether_types::Message]) -> Option<usize> {
    let users = messages
        .iter()
        .enumerate()
        .filter(|(_, message)| message.role == Role::User)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let cut = *users.get(users.len().saturating_sub(2))?;
    (cut >= 6).then_some(cut)
}

/// Builds the provider and summary request for the messages being compacted.
fn summary_call(
    actor: &SessionActor,
    old: &[tether_types::Message],
    focus: Option<&str>,
) -> Result<(Arc<dyn tether_providers::ChatProvider>, ChatRequest), CoreError> {
    let provider = (actor.provider_for_turn)()?;
    let request = ChatRequest {
        model_id: actor.session.model_id.clone(),
        system: summary_prompt(focus),
        messages: old.iter().map(shrink_message).collect(),
        tools: Vec::new(),
        max_output_tokens: 2048,
        reasoning: None,
    };
    Ok((provider, request))
}

/// Saves the summary as a message, records the compaction, and tells the sink.
fn store_compaction(
    actor: &mut SessionActor,
    old: &[tether_types::Message],
    text: &str,
    focus: Option<String>,
) -> Result<(), CoreError> {
    let empty_range = || CoreError::Internal {
        message: "compaction range is empty".into(),
    };
    let first = old.first().ok_or_else(empty_range)?;
    let last = old.last().ok_or_else(empty_range)?;
    let text = format!("[Conversation summary]\n{}", limit_words(text, 1_500));
    let summary = actor
        .store
        .append_message(
            &actor.session.id,
            Role::Assistant,
            &[MessagePart::CompactionSummary { text }],
            super::now_ms(),
        )
        .map_err(super::store_error)?;
    let before = crate::context::estimate_messages(old);
    let after = crate::context::estimate_messages(std::slice::from_ref(&summary));
    actor
        .store
        .insert_compaction(&tether_store::CompactionRecord {
            id: tether_store::new_compaction_id(),
            session_id: actor.session.id.clone(),
            summary_message_id: summary.id.clone(),
            first_message_id: first.id.clone(),
            last_message_id: last.id.clone(),
            tokens_before: before,
            tokens_after: after,
            created_at: super::now_ms(),
            focus,
        })
        .map_err(super::store_error)?;
    actor.sink.on_event(CoreEvent::ContextCompacted {
        session_id: actor.session.id.clone(),
        summary_message_id: summary.id,
        tokens_before: before,
        tokens_after: after,
    });
    Ok(())
}

fn shrink_message(message: &tether_types::Message) -> tether_types::Message {
    let mut copy = message.clone();
    copy.parts = copy
        .parts
        .into_iter()
        .map(|part| match part {
            tether_types::MessagePart::ToolResult {
                tool_call_id,
                is_error,
                text,
                result_ref,
            } if text.chars().count() > 300 => tether_types::MessagePart::ToolResult {
                tool_call_id,
                is_error,
                text: format!(
                    "{}\n[tool result shortened for compaction]",
                    text.chars().take(300).collect::<String>()
                ),
                result_ref,
            },
            part => part,
        })
        .collect();
    copy
}

fn summary_prompt(focus: Option<&str>) -> String {
    let focus = focus.map_or(String::new(), |value| format!(" Focus: {value}."));
    format!(
        "Write a faithful working summary of the goal, decisions and why, files touched, commands and outcomes, errors and fixes, current checklist, open questions, user preferences, and the exact next step. Keep it under 1,500 words.{focus}"
    )
}

fn limit_words(text: &str, limit: usize) -> String {
    text.split_whitespace()
        .take(limit)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Streams one summary. The usage the provider reported comes back with the result, so a
/// failed or interrupted request is still counted.
async fn summary_request(
    provider: Arc<dyn tether_providers::ChatProvider>,
    request: ChatRequest,
    cancel: CancellationToken,
) -> (Result<String, CoreError>, tether_providers::ResponseUsage) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let task = tokio::spawn(async move { provider.stream_chat(&request, tx, cancel).await });
    let mut text = String::new();
    let mut usage = tether_providers::ResponseUsage::default();
    while let Some(event) = rx.recv().await {
        match event {
            tether_providers::StreamEvent::TextDelta { text: delta } => text.push_str(&delta),
            tether_providers::StreamEvent::Usage(reported) => usage = reported,
            tether_providers::StreamEvent::Finished { .. } => break,
            _ => {}
        }
    }
    let finished = match task.await {
        Ok(result) => result.map_err(CoreError::from),
        Err(_) => Err(CoreError::Internal {
            message: "summary request stopped".into(),
        }),
    };
    (finished.map(|()| text), usage)
}

pub(super) fn should_auto_compact(
    store: &tether_store::Store,
    session: &tether_store::SessionRecord,
    prompt: &crate::prompt::SystemPrompt,
    tools: &[ToolSpec],
    messages: &[tether_types::Message],
) -> Result<bool, CoreError> {
    let window = store
        .models(session.provider)
        .map_err(super::store_error)?
        .into_iter()
        .find(|model| model.id == session.model_id)
        .and_then(|model| model.context_window);
    let Some(window) = window else {
        return Ok(false);
    };
    let threshold = store.compaction_threshold().map_err(super::store_error)?;
    let prompt_tokens = prompt
        .sections
        .iter()
        .map(|section| section.approx_tokens)
        .sum::<u32>();
    let tool_tokens = tools
        .iter()
        .map(|tool| {
            crate::prompt::estimate_tokens(&format!(
                "{}{}{}",
                tool.name, tool.description, tool.input_schema_json
            ))
        })
        .sum::<u32>();
    let estimate = prompt_tokens
        .saturating_add(tool_tokens)
        .saturating_add(crate::context::estimate_messages(messages));
    let over_threshold = u64::from(estimate).saturating_mul(100)
        >= u64::from(window).saturating_mul(u64::from(threshold));
    Ok(over_threshold && compaction_cut(messages).is_some())
}

#[cfg(test)]
mod tests {
    use tether_store::{SessionRecord, Store};
    use tether_types::{Message, MessagePart, ModelCaps, ModelInfo, Provider, Role};

    use crate::prompt::{PromptSection, SystemPrompt};
    use crate::session_tests::seeded_store;

    use super::limit_words;
    use super::should_auto_compact;

    fn prompt(tokens: u32) -> SystemPrompt {
        SystemPrompt {
            text: String::new(),
            sections: vec![PromptSection {
                name: "Instructions".into(),
                chars: tokens * 4,
                approx_tokens: tokens,
            }],
        }
    }

    fn add_empty_turn(store: &mut Store, session_id: &tether_types::SessionId) {
        for role in [Role::User, Role::Assistant] {
            store
                .append_message(
                    session_id,
                    role,
                    &[MessagePart::Text {
                        text: String::new(),
                    }],
                    1,
                )
                .unwrap();
        }
    }

    fn threshold_fixture() -> (tempfile::TempDir, Store, SessionRecord, Vec<Message>) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("sessions.db");
        let (mut store, session_id) = seeded_store(&path);
        store
            .replace_models(
                Provider::Anthropic,
                &[ModelInfo {
                    provider: Provider::Anthropic,
                    id: "test-model".into(),
                    display_name: "Test".into(),
                    context_window: Some(100),
                    max_output_tokens: None,
                    price: None,
                    caps: ModelCaps::UNKNOWN,
                }],
                1,
            )
            .unwrap();
        for _ in 0..5 {
            add_empty_turn(&mut store, &session_id);
        }
        let session = store.load_session(&session_id).unwrap().unwrap();
        let messages = store.messages(&session_id).unwrap();
        (directory, store, session, messages)
    }

    #[test]
    fn summary_text_is_capped_at_1500_words() {
        let input = (0..1_501)
            .map(|value| format!("word{value}"))
            .collect::<Vec<_>>()
            .join(" ");

        let output = limit_words(&input, 1_500);

        assert_eq!(output.split_whitespace().count(), 1_500);
        assert_eq!(output.split_whitespace().last(), Some("word1499"));
    }

    #[test]
    fn automatic_compaction_starts_at_the_default_eighty_percent_threshold() {
        let (_directory, store, session, messages) = threshold_fixture();

        assert!(!should_auto_compact(&store, &session, &prompt(79), &[], &messages).unwrap());
        assert!(should_auto_compact(&store, &session, &prompt(80), &[], &messages).unwrap());
    }
}
