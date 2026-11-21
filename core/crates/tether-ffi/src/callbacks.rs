//! Callback interfaces the platform layer implements.

use std::sync::Arc;

use tether_types::CoreEvent;

use crate::views::CoreEventView;

/// Receives core events on the platform side (Kotlin today, Swift in v2).
#[uniffi::export(with_foreign)]
pub trait EventSink: Send + Sync {
    /// Called once per event, on the thread that produced it.
    fn on_event(&self, event: CoreEventView);
}

/// Receives background approval and turn notifications.
#[uniffi::export(with_foreign)]
pub trait NotifyPlatform: Send + Sync {
    /// Shows an approval action.
    fn show_approval(
        &self,
        session_id: String,
        tool_call_id: String,
        tool: String,
        target: String,
        summary: String,
    );
    /// Shows a question action.
    fn show_question(
        &self,
        session_id: String,
        tool_call_id: String,
        question: String,
        options: Vec<String>,
    );
    /// Shows a finished turn.
    fn show_turn_finished(&self, session_id: String, title: String, ok: bool, message: String);
    /// Updates the current tool step.
    fn update_run_progress(&self, session_id: String, step: String);
    /// Clears notifications for one session.
    fn clear(&self, session_id: String);
}

/// Starts and stops the platform foreground run service.
#[uniffi::export(with_foreign)]
pub trait RunServicePlatform: Send + Sync {
    /// Starts the service.
    fn start_run_service(&self);
    /// Stops the service.
    fn stop_run_service(&self);
}

/// Reports connectivity to the core.
#[uniffi::export(with_foreign)]
pub trait NetworkPlatform: Send + Sync {
    /// Returns the current connectivity state.
    fn is_online(&self) -> bool;
}

/// Adapts the platform sink to the engine's own sink trait.
pub(crate) struct SinkAdapter(pub(crate) Arc<dyn EventSink>);

impl tether_engine::EventSink for SinkAdapter {
    fn on_event(&self, event: CoreEvent) {
        self.0.on_event(CoreEventView::from(event));
    }
}
