//! The reasoning effort a turn's request carries.

use tether_types::{ReasoningEffort, Support};

use super::SessionActor;

impl SessionActor {
    /// Returns the saved effort for the turn's request. Only a model whose catalog entry
    /// says it reasons gets the setting, because other models reject it. A missing entry
    /// or an unknown capability sends nothing.
    pub(super) fn reasoning_for_request(&self) -> Option<ReasoningEffort> {
        let effort = self.session.reasoning_effort?;
        let model = self
            .store
            .models(self.session.provider)
            .ok()?
            .into_iter()
            .find(|model| model.id == self.session.model_id)?;
        (model.caps.reasoning == Support::Yes).then_some(effort)
    }
}
