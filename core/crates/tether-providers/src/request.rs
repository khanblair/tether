//! What the engine asks a provider to do.

use tether_types::Message;

/// How much reasoning a model should spend. The type lives in `tether-types`.
pub use tether_types::ReasoningEffort;

/// A tool the model may call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    /// Tool name, such as `edit_file`.
    pub name: String,
    /// What the tool does, written for the model.
    pub description: String,
    /// JSON Schema of the arguments, as a JSON object in text form.
    pub input_schema_json: String,
}

/// One request for a model response.
///
/// Order matters for prompt caching: `system` and `tools` come first, so an adapter can
/// mark them as the cached prefix, and `messages` follow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRequest {
    /// The provider's own model identifier.
    pub model_id: String,
    /// System prompt, instructions, and the memory slice, already joined.
    pub system: String,
    /// The conversation so far, oldest first.
    pub messages: Vec<Message>,
    /// Tools the model may call. Empty for a plain chat turn.
    pub tools: Vec<ToolSpec>,
    /// Upper bound on generated tokens.
    pub max_output_tokens: u32,
    /// Reasoning effort, or `None` for the model's default.
    pub reasoning: Option<ReasoningEffort>,
}
