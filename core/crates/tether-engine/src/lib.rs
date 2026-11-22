//! Engine: sessions, turn loop, permissions, executor router, checkpoints.

//! P0 holds only the version and the event hand-off used by the FFI layer.

mod budget;
mod checkpoint_capture;
pub mod commands;
pub mod config;
pub mod context;
pub mod executors;
pub mod network;
pub mod notify;
pub mod permissions;
pub mod prompt;
mod provider_connection;
mod rewind;
mod session;
mod session_provider;
#[cfg(test)]
mod session_stream_tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod session_tool_tests;
#[cfg(test)]
mod session_usage_tests;
mod tool_loop;
pub mod updates;
mod usage;

pub use checkpoint_capture::capture_checkpoint;
pub use provider_connection::{ProviderCheck, check_provider, provider_test_targets};
pub use rewind::{RewindError, RewindScope, rewind_checkpoint, rewind_conversation_only};
pub use session::{
    ProviderFactory, SessionActor, SessionActorError, SessionActorOptions, SessionHandle,
    ToolRuntime,
};
pub use session_provider::provider_for;
pub use tool_loop::git_tools::{
    CommitGate, CommitInput, GateFinding, GateSeverity, GitIdentityReader, GitTools,
    GitToolsConfig, GithubClientFactory, TransportFactory,
};
pub use tool_loop::session_tools::{SESSION_TOOL_SPECS, SessionToolSpec, SessionTools};
pub use tool_loop::{
    ExecuteContext, PrepareContext, PreparedCall, RegistryError, ToolHandler, ToolOutput,
    ToolRegistry,
};
pub use tool_loop::{ToolLoop, ToolLoopContext, ToolLoopResult};

use std::sync::Arc;

pub use tether_providers::BoxFuture;
use tether_types::CoreEvent;
use tether_types::{SessionId, ToolCallId};
use tokio_util::sync::CancellationToken;

/// The prepared action shown to the user before approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequest {
    /// Session that owns the call.
    pub session_id: SessionId,
    /// Provider call identifier.
    pub tool_call_id: ToolCallId,
    /// Tool name.
    pub tool: String,
    /// Guarded display target.
    pub target: String,
}

/// A question the platform may answer while a tool turn waits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionRequest {
    /// Session that owns the question.
    pub session_id: SessionId,
    /// Provider call identifier.
    pub tool_call_id: ToolCallId,
    /// Question text.
    pub question: String,
    /// Tappable answer options.
    pub options: Vec<String>,
    /// Whether typed answers are accepted.
    pub allow_free_text: bool,
}

/// The answer returned by the platform for a question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Text selected or entered by the user.
    Text(String),
    /// The question ended without an answer.
    Cancelled,
}

/// Resolves questions without blocking the tool loop thread.
pub trait Asker: Send + Sync {
    /// Reports whether the platform can display an interactive question.
    fn interactive(&self) -> bool {
        true
    }

    /// Waits for an answer or resolves as cancelled when cancellation fires.
    fn ask(&self, request: QuestionRequest, cancel: CancellationToken) -> BoxFuture<'_, Answer>;
}

/// The user's answer to an approval request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalAnswer {
    /// The call may run, optionally creating a project rule.
    Allow {
        /// Whether the user selected Always allow.
        always: bool,
    },
    /// The call may run after an approval notification action.
    AllowFromNotification {
        /// Whether the user selected Always allow.
        always: bool,
    },
    /// The call must not run.
    Deny {
        /// Optional user-provided explanation.
        reason: Option<String>,
    },
    /// The call was denied from an approval notification.
    DenyFromNotification {
        /// Optional user-provided explanation.
        reason: Option<String>,
    },
}

/// Resolves approval requests without blocking the tool loop thread.
pub trait Approver: Send + Sync {
    /// Reports whether the platform can display an interactive approval.
    fn interactive(&self) -> bool {
        true
    }

    /// Waits for an answer or resolves as denied when cancellation fires.
    fn decide(
        &self,
        request: ApprovalRequest,
        cancel: CancellationToken,
    ) -> BoxFuture<'_, ApprovalAnswer>;
}

impl<F> Approver for F
where
    F: Fn(&str, &str) -> bool + Send + Sync,
{
    fn decide(
        &self,
        request: ApprovalRequest,
        _cancel: CancellationToken,
    ) -> BoxFuture<'_, ApprovalAnswer> {
        let answer = if self(request.tool.as_str(), request.target.as_str()) {
            ApprovalAnswer::Allow { always: false }
        } else {
            ApprovalAnswer::Deny { reason: None }
        };
        Box::pin(async move { answer })
    }
}

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
