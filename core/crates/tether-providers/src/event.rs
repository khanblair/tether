//! What a provider streams back.

use tether_types::ToolCallId;

/// Why the model stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// The model finished its turn.
    EndTurn,
    /// The model wants one or more tool calls run.
    ToolUse,
    /// The output hit `max_output_tokens`.
    MaxTokens,
    /// The provider's safety system stopped the reply.
    ContentFilter,
    /// A stop reason this crate does not know, kept as the provider wrote it.
    Other(String),
}

/// Running token totals for one response. A later `Usage` event replaces an earlier one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResponseUsage {
    /// Input tokens, not counting cache reads and writes.
    pub input_tokens: u64,
    /// Output tokens so far.
    pub output_tokens: u64,
    /// Input tokens served from the prompt cache.
    pub cache_read_tokens: u64,
    /// Input tokens written to the prompt cache.
    pub cache_write_tokens: u64,
}

/// One event of a streamed response, in the order the model produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    /// A piece of reply text.
    TextDelta {
        /// The new text.
        text: String,
    },
    /// A piece of model reasoning.
    ThinkingDelta {
        /// The new text.
        text: String,
    },
    /// The model started a tool call.
    ToolCallStarted {
        /// The provider's id for the call.
        id: ToolCallId,
        /// Tool name.
        name: String,
    },
    /// A fragment of the call's JSON arguments. Join the fragments in order.
    ToolCallArgsDelta {
        /// The call the fragment belongs to.
        id: ToolCallId,
        /// The next piece of the arguments text.
        json_fragment: String,
    },
    /// The call's arguments are complete.
    ToolCallFinished {
        /// The finished call.
        id: ToolCallId,
    },
    /// Token totals so far.
    Usage(ResponseUsage),
    /// The response is complete. It is always the last event of a successful stream.
    Finished {
        /// Why the model stopped.
        stop_reason: StopReason,
    },
}
