//! Engine: sessions, turn loop, permissions, executor router, checkpoints.

//! P0 holds only the version and the event hand-off used by the FFI layer.

mod provider_connection;
mod checkpoint_capture;
mod session;
mod session_provider;
mod tool_loop;
#[cfg(test)]
mod session_stream_tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod session_usage_tests;
pub mod updates;

pub use provider_connection::{ProviderCheck, check_provider, provider_test_targets};
pub use checkpoint_capture::capture_checkpoint;
pub use session::{ProviderFactory, SessionActor, SessionActorError, SessionHandle};
pub use session_provider::provider_for;
pub use tool_loop::{ToolLoop, ToolLoopResult};

use std::sync::Arc;

use tether_types::CoreEvent;

/// Core version. Equals the app version; the device test `NativeLibraryTest` checks that they match.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Receives events from the engine. The FFI layer implements this for the platform.
pub trait EventSink: Send + Sync {
    /// Called once per event, on the thread that produced it.
    fn on_event(&self, event: CoreEvent);
}

/// The engine entry point. It will own sessions, the store, and config.
pub struct Core {
    sink: Arc<dyn EventSink>,
}

impl Core {
    /// Creates the core and announces it with a `Ready` event.
    pub fn new(sink: Arc<dyn EventSink>) -> Self {
        sink.on_event(CoreEvent::Ready {
            version: VERSION.to_owned(),
        });
        Self { sink }
    }

    /// Emits a `Test` event. Used by P0 to prove the event path end to end.
    pub fn emit_test_event(&self, message: String) {
        self.sink.on_event(CoreEvent::Test { message });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<CoreEvent>>);

    impl EventSink for Recorder {
        fn on_event(&self, event: CoreEvent) {
            self.0.lock().expect("lock").push(event);
        }
    }

    #[test]
    fn new_core_announces_ready_then_test_events_flow() {
        let recorder = Arc::new(Recorder::default());
        let core = Core::new(recorder.clone());
        core.emit_test_event("ping".into());

        let events = recorder.0.lock().expect("lock").clone();
        assert_eq!(
            events,
            vec![
                CoreEvent::Ready {
                    version: VERSION.to_owned()
                },
                CoreEvent::Test {
                    message: "ping".into()
                },
            ]
        );
    }
}
