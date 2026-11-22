//! Routes each command the session actor receives to its handler.

use crate::commands::MentionResolution;

use super::{Command, SessionActor, SessionActorError};

impl SessionActor {
    /// Runs one command and sends its reply when the caller asked for one.
    pub(super) async fn handle_command(&mut self, command: Command) {
        match command {
            Command::Send { text, outcome } => {
                let resolution = self.resolve_mentions(&text);
                drop(outcome.map(|reply| reply.send(resolution.clone())));
                self.run_turn(text, resolution).await;
            }
            Command::Rewind {
                checkpoint_id,
                scope,
                reply,
            } => {
                let result = self.rewind(&checkpoint_id, scope);
                drop(reply.send(result));
            }
            Command::EnableBypass { reply } => {
                drop(reply.send(self.set_bypass(true)));
            }
            Command::DisableBypass { reply } => {
                drop(reply.send(self.set_bypass(false)));
            }
            Command::RunPlan { mode, reply } => {
                let result = self.set_working_mode(mode);
                let started = result.is_ok();
                drop(reply.send(result));
                self.run_plan(started).await;
            }
            Command::ClearContext { reply } => {
                let result = self.clear_context();
                drop(reply.send(result));
            }
            Command::Resume { reply } => {
                let result = self.resume_turn().await;
                self.end_on_turn_error(&result);
                drop(reply.send(result));
            }
            Command::DiscardInterrupted { reply } => {
                let result = self.discard_interrupted();
                drop(reply.send(result));
            }
            Command::Compact { focus, reply } => {
                let result = self.compact(focus).await;
                drop(reply.send(result));
            }
        }
    }

    /// Runs one turn and ends the session with its error when the turn fails.
    async fn run_turn(&mut self, text: String, mentions: MentionResolution) {
        if let Err(error) = self.turn(text, mentions).await {
            self.end_with_error(error);
        }
    }

    /// Starts the plan turn when the working mode was changed.
    async fn run_plan(&mut self, started: bool) {
        if started {
            self.run_turn("Run the plan.".into(), MentionResolution::default())
                .await;
        }
    }

    /// Ends the session with the turn error, when the result carries one.
    fn end_on_turn_error<T>(&mut self, result: &Result<T, SessionActorError>) {
        if let Err(SessionActorError::Turn(error)) = result {
            self.end_with_error(error.clone());
        }
    }
}
