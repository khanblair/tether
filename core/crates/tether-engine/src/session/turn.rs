use std::sync::Arc;

use tether_providers::{ChatRequest, ToolSpec};
use tether_types::{CoreError, CoreEvent, MessagePart, Role, SessionStatus, UsageSnapshot};
use tokio_util::sync::CancellationToken;

use super::{SessionActor, ToolLoop, ToolLoopContext};
use crate::commands::{MentionResolution, context_block};
use crate::prompt::{PromptBuilder, PromptInputs, current_date, load_instructions};

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
    let text = summary_request(provider, request).await?;
    store_compaction(actor, old, &text, focus)
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
    let text = format!("[Conversation summary]\n{text}");
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

async fn summary_request(
    provider: Arc<dyn tether_providers::ChatProvider>,
    request: ChatRequest,
) -> Result<String, CoreError> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let cancel = CancellationToken::new();
    let task = tokio::spawn(async move { provider.stream_chat(&request, tx, cancel).await });
    let mut text = String::new();
    while let Some(event) = rx.recv().await {
        match event {
            tether_providers::StreamEvent::TextDelta { text: delta } => text.push_str(&delta),
            tether_providers::StreamEvent::Finished { .. } => break,
            _ => {}
        }
    }
    task.await
        .map_err(|_| CoreError::Internal {
            message: "summary request stopped".into(),
        })?
        .map_err(CoreError::from)?;
    Ok(text)
}

impl SessionActor {
    #[expect(
        clippy::too_many_lines,
        reason = "one serialized turn owns persistence, tool execution, usage, and status"
    )]
    pub(super) async fn turn(
        &mut self,
        text: String,
        mentions: MentionResolution,
    ) -> Result<(), CoreError> {
        let id = self.session.id.clone();
        if text.is_empty() {
            self.store
                .set_session_status(&id, SessionStatus::Requesting, super::now_ms())
                .map_err(super::store_error)?;
        } else {
            self.store
                .begin_turn(&id, &text, super::now_ms())
                .map_err(super::store_error)?;
        }
        let mut messages = self
            .store
            .context_messages(&id)
            .map_err(super::store_error)?
            .into_iter()
            .filter(|message| !message.rewound)
            .collect::<Vec<_>>();
        if !text.is_empty() {
            append_mention_context(&mut messages, &mentions);
        }
        let tool_specs = self.prompt_tool_specs()?;
        let instructions = load_instructions(
            self.prompt_sources.global_instructions_path.as_deref(),
            self.prompt_sources.project_root.as_deref(),
        );
        let snapshot = self.prompt_snapshot.as_ref();
        let today = self
            .prompt_sources
            .today
            .clone()
            .unwrap_or_else(current_date);
        let prompt = PromptBuilder::build(&PromptInputs {
            project_name: &self.project_name,
            executors: &self.prompt_sources.executors,
            branch: self.prompt_sources.branch.as_deref(),
            mode: self.session.mode,
            tool_specs: &tool_specs,
            global_instructions: &instructions.global,
            project_instructions: &instructions.project,
            memory_block: snapshot.map_or("", |value| value.memory_block.as_str()),
            vault_block: snapshot.map_or("", |value| value.vault_block.as_str()),
            skills_index: self.prompt_sources.skills_index.as_deref(),
            today: &today,
        });
        if should_auto_compact(&self.store, &self.session, &prompt, &tool_specs, &messages)? {
            // Compaction is best effort: when it fails, the turn runs on the full history.
            drop(compact_session(self, None).await);
            messages = self
                .store
                .context_messages(&id)
                .map_err(super::store_error)?;
        }
        let request = ChatRequest {
            model_id: self.session.model_id.clone(),
            system: prompt.text,
            messages,
            tools: tool_specs,
            max_output_tokens: super::MAX_OUTPUT_TOKENS,
            reasoning: None,
        };
        let provider = (self.provider_for_turn)()?;
        let cancel = CancellationToken::new();
        if let Ok(mut active) = self.active.lock() {
            *active = Some(cancel.clone());
        }
        self.emit_status(SessionStatus::Requesting);
        if self.tools.is_none() {
            let target = crate::budget::CheckTarget {
                session: &self.session,
                sink: self.sink.as_ref(),
                cancel: &cancel,
            };
            self.budget
                .check(&mut self.store, target, super::now_ms())
                .await?;
        }
        let result = if self.tools.is_some() {
            self.run_tools(provider, request, cancel.clone()).await
        } else {
            self.stream(provider, request, cancel).await
        };
        if let Ok(mut active) = self.active.lock() {
            *active = None;
        }
        let (usage, price_known, result) = result;
        if self.tools.is_none() {
            self.store
                .record_response_usage(&tether_store::ResponseUsageRecord {
                    session_id: id.clone(),
                    provider: self.session.provider,
                    model_id: self.session.model_id.clone(),
                    project_id: self.session.project_id.as_str().into(),
                    day: crate::usage::utc_day_from_ms(super::now_ms()),
                    usage,
                    price_known,
                    now_ms: super::now_ms(),
                })
                .map_err(super::store_error)?;
        }
        let totals = self.store.session_usage(&id).map_err(super::store_error)?;
        self.sink.on_event(CoreEvent::UsageUpdated {
            session_id: id.clone(),
            usage: totals,
        });
        result?;
        self.store
            .set_session_status(&id, SessionStatus::Idle, super::now_ms())
            .map_err(super::store_error)?;
        self.emit_status(SessionStatus::Idle);
        self.sink
            .on_event(CoreEvent::TurnFinished { session_id: id });
        Ok(())
    }

    fn prompt_tool_specs(&self) -> Result<Vec<ToolSpec>, CoreError> {
        let Some(tools) = self.tools.as_ref() else {
            return Ok(Vec::new());
        };
        ToolLoop::with_handlers(tools.guard.clone(), tools.handlers.clone())
            .map(|runner| runner.specs())
            .map_err(|error| CoreError::Internal {
                message: error.to_string(),
            })
    }

    async fn run_tools(
        &mut self,
        provider: Arc<dyn tether_providers::ChatProvider>,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> (UsageSnapshot, bool, Result<(), CoreError>) {
        let Some(tools) = self.tools.as_ref() else {
            return (UsageSnapshot::default(), true, Ok(()));
        };
        let loop_runner = match ToolLoop::with_handlers(tools.guard.clone(), tools.handlers.clone())
        {
            Ok(runner) => runner,
            Err(error) => {
                return (
                    UsageSnapshot::default(),
                    true,
                    Err(CoreError::Internal {
                        message: error.to_string(),
                    }),
                );
            }
        };
        let mode_handle = Arc::clone(&tools.mode);
        let saved_mode = self.session.mode;
        let mode = move || mode_handle.lock().map_or(saved_mode, |mode| *mode);
        let mut context = ToolLoopContext {
            session_id: &self.session.id,
            store: &mut self.store,
            blobs: &tools.blobs,
            cancel,
            approver: tools.approver.as_ref(),
            mode: &mode,
            sink: Some(self.sink.as_ref()),
        };
        let result = loop_runner
            .run_with_budget(provider, request, &mut context, &self.budget)
            .await;
        (UsageSnapshot::default(), true, result.map(|_| ()))
    }
}

fn should_auto_compact(
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
    Ok(u64::from(estimate).saturating_mul(100)
        >= u64::from(window).saturating_mul(u64::from(threshold)))
}

fn append_mention_context(messages: &mut [tether_types::Message], mentions: &MentionResolution) {
    if mentions.attached.is_empty() {
        return;
    }
    let context = context_block(&mentions.attached);
    if let Some(message) = messages
        .iter_mut()
        .rev()
        .find(|message| message.role == Role::User)
        && let Some(MessagePart::Text { text }) = message
            .parts
            .iter_mut()
            .find(|part| matches!(part, MessagePart::Text { .. }))
    {
        text.push_str("\n\n");
        text.push_str(&context);
    }
}
