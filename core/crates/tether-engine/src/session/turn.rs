use std::collections::HashSet;
use std::sync::Arc;

use tether_providers::{ChatRequest, ToolSpec};
use tether_skills::{DiscoverInput, SkillsCatalog};
use tether_types::{CoreError, CoreEvent, MessagePart, Role, SessionStatus, UsageSnapshot};
use tokio_util::sync::CancellationToken;

use super::compact::{compact_session, should_auto_compact};
use super::{SessionActor, ToolLoop, ToolLoopContext};
use crate::commands::{MentionResolution, context_block};
use crate::prompt::{PromptBuilder, PromptInputs, current_date, load_instructions};

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
        self.append_new_skill_note(&mut messages)?;
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
            skills_index: snapshot.map(|value| value.skills_block.as_str()),
            today: &today,
        });
        if should_auto_compact(&self.store, &self.session, &prompt, &tool_specs, &messages)? {
            self.auto_compact().await?;
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

    async fn auto_compact(&mut self) -> Result<(), CoreError> {
        let result = compact_session(self, None).await;
        if let Err(error) = result {
            self.record_compaction_failure(&error)?;
        }
        Ok(())
    }

    fn record_compaction_failure(&mut self, error: &CoreError) -> Result<(), CoreError> {
        self.store
            .append_message(
                &self.session.id,
                Role::SystemNote,
                &[MessagePart::Text {
                    text: format!("Context compaction failed: {}", error.message()),
                }],
                super::now_ms(),
            )
            .map_err(super::store_error)?;
        Ok(())
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

impl SessionActor {
    fn append_new_skill_note(
        &mut self,
        messages: &mut [tether_types::Message],
    ) -> Result<(), CoreError> {
        let Some(catalog) = self.live_skill_catalog()? else {
            return Ok(());
        };
        let snapshot_names = self.snapshot_skill_names();
        let Some(skill) = catalog.skills.iter().find(|skill| {
            !snapshot_names.contains(&skill.name)
                && self.announced_skills.insert(skill.name.clone())
        }) else {
            return Ok(());
        };
        let Some(message) = messages
            .iter_mut()
            .rev()
            .find(|message| message.role == Role::User)
        else {
            return Ok(());
        };
        message.parts.push(MessagePart::Text {
            text: format!(
                "\n\nNew skill available: {}: {}. Use skill_view to load it.",
                skill.name, skill.description
            ),
        });
        Ok(())
    }

    fn live_skill_catalog(&self) -> Result<Option<SkillsCatalog>, CoreError> {
        let user_dir = self
            .prompt_sources
            .memory_root
            .as_deref()
            .and_then(std::path::Path::parent)
            .map(|path| path.join("skills"))
            .or_else(|| {
                self.prompt_sources
                    .project_root
                    .as_deref()
                    .map(|path| path.join("skills"))
            });
        let Some(user_dir) = user_dir else {
            return Ok(None);
        };
        let trusted = self
            .store
            .is_project_trusted(&self.session.project_id)
            .map_err(super::store_error)?;
        Ok(Some(SkillsCatalog::discover(DiscoverInput {
            project_root: self.prompt_sources.project_root.as_deref(),
            project_trusted: trusted,
            user_dir: &user_dir,
            bundled: &[],
        })))
    }

    fn snapshot_skill_names(&self) -> HashSet<String> {
        self.prompt_snapshot
            .as_ref()
            .map_or_else(HashSet::new, |snapshot| {
                snapshot
                    .skills_block
                    .lines()
                    .filter_map(skill_name)
                    .collect::<HashSet<_>>()
            })
    }
}

fn skill_name(line: &str) -> Option<String> {
    let text = line.strip_prefix("- ")?;
    let name = text.split_once(':').map_or(text, |(name, _)| name).trim();
    (!name.is_empty()).then(|| name.to_owned())
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
