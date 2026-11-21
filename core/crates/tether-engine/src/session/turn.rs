use std::sync::Arc;

use tether_providers::ChatRequest;
use tether_types::{CoreError, CoreEvent, SessionStatus, UsageSnapshot};
use tokio_util::sync::CancellationToken;

use super::{SessionActor, ToolLoop, ToolLoopContext};

impl SessionActor {
    #[expect(
        clippy::too_many_lines,
        reason = "one serialized turn owns persistence, tool execution, usage, and status"
    )]
    pub(super) async fn turn(&mut self, text: String) -> Result<(), CoreError> {
        let id = self.session.id.clone();
        self.store
            .begin_turn(&id, &text, super::now_ms())
            .map_err(super::store_error)?;
        let messages = self
            .store
            .messages(&id)
            .map_err(super::store_error)?
            .into_iter()
            .filter(|message| !message.rewound)
            .collect();
        let request = ChatRequest {
            model_id: self.session.model_id.clone(),
            system: String::new(),
            messages,
            tools: Vec::new(),
            max_output_tokens: super::MAX_OUTPUT_TOKENS,
            reasoning: None,
        };
        let provider = (self.provider_for_turn)()?;
        let cancel = CancellationToken::new();
        if let Ok(mut active) = self.active.lock() {
            *active = Some(cancel.clone());
        }
        self.emit_status(SessionStatus::Requesting);
        let result = if self.tools.is_some() {
            self.run_tools(provider, request, cancel.clone()).await
        } else {
            self.stream(provider, request, cancel).await
        };
        if let Ok(mut active) = self.active.lock() {
            *active = None;
        }
        let (usage, result) = result;
        self.store
            .add_session_usage(&id, usage, super::now_ms())
            .map_err(super::store_error)?;
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

    async fn run_tools(
        &mut self,
        provider: Arc<dyn tether_providers::ChatProvider>,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> (UsageSnapshot, Result<(), CoreError>) {
        let Some(tools) = self.tools.as_ref() else {
            return (UsageSnapshot::default(), Ok(()));
        };
        let loop_runner =
            match ToolLoop::with_handlers(tools.guard.clone(), tools.handlers.clone()) {
                Ok(runner) => runner,
                Err(error) => {
                    return (
                        UsageSnapshot::default(),
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
        let result = loop_runner.run(provider, request, &mut context).await;
        (UsageSnapshot::default(), result.map(|_| ()))
    }
}
