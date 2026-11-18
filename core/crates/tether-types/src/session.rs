//! Session settings and states: permission mode, executor, provider, model, status.

/// Which tool calls need the user's approval (product-scope F5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionMode {
    /// Investigate read-only and return a plan.
    Plan,
    /// Ask before every change.
    Ask,
    /// File edits run without asking; commands still ask.
    AcceptEdits,
    /// Sandbox runs and edits run without asking.
    Auto,
    /// Everything runs without asking, except a force push.
    Bypass,
}

/// Where a command runs (product-scope F10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Executor {
    /// WebAssembly runtimes inside the app.
    Sandbox,
    /// Alpine Linux through PRoot (`full` flavor only).
    Linux,
    /// GitHub Actions in the project's repo.
    Ci,
}

/// A model provider the user can connect (product-scope F1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    /// Anthropic.
    Anthropic,
    /// OpenAI.
    OpenAi,
    /// Google Gemini.
    Gemini,
    /// DeepSeek.
    DeepSeek,
    /// OpenRouter.
    OpenRouter,
}

/// Names one model at one provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelRef {
    /// The provider that serves the model.
    pub provider: Provider,
    /// The provider's own model identifier.
    pub model_id: String,
}

/// The session state machine from architecture section 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    /// Waiting for the user.
    Idle,
    /// A request is on its way to the provider.
    Requesting,
    /// The model is streaming a reply.
    Streaming,
    /// The permission check for a proposed tool call is running.
    CheckingTool,
    /// A tool call is running.
    RunningTool,
    /// A tool call waits for the user to allow or deny it.
    AwaitingApproval,
    /// The network is down; the turn resumes when it returns.
    Paused,
    /// The turn was stopped or the process died; the user can resume.
    Interrupted,
    /// A fatal error ended the turn; the user can retry.
    Failed,
}

/// The tool call state machine from architecture section 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallStatus {
    /// The model proposed the call.
    Proposed,
    /// The permission check allowed it.
    Allowed,
    /// It waits for the user.
    AwaitingApproval,
    /// It is running.
    Running,
    /// The user denied it.
    Denied,
    /// The rules blocked it without asking.
    Blocked,
    /// It finished and succeeded.
    Succeeded,
    /// It finished and failed.
    Failed,
    /// It hit its time limit.
    TimedOut,
    /// It was cancelled.
    Cancelled,
}

/// State of a provider connection from architecture section 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// No key, or the key changed since the last test.
    Unset,
    /// A test call is running.
    Testing,
    /// The key works.
    Ok,
    /// The provider rejected the key.
    BadKey,
    /// The test call could not reach the provider.
    NetworkError,
    /// The provider refused the request for this region or account.
    Blocked,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_refs_compare_by_provider_and_id() {
        let a = ModelRef {
            provider: Provider::Anthropic,
            model_id: "m1".into(),
        };
        let b = ModelRef {
            provider: Provider::OpenRouter,
            model_id: "m1".into(),
        };
        assert_ne!(a, b);
    }
}
