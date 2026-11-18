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

/// Adapts the platform sink to the engine's own sink trait.
pub(crate) struct SinkAdapter(pub(crate) Arc<dyn EventSink>);

impl tether_engine::EventSink for SinkAdapter {
    fn on_event(&self, event: CoreEvent) {
        self.0.on_event(CoreEventView::from(event));
    }
}
