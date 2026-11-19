//! The JSON shapes of Anthropic's streaming events and error bodies.
//!
//! Only the fields the adapter reads are listed. Unknown event, block, and delta types fall
//! into an `Other` variant so a new Anthropic feature does not break the stream.

use serde::Deserialize;

/// One streaming event, chosen by the `type` field of its data.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum WireEvent {
    MessageStart {
        message: StartedMessage,
    },
    ContentBlockStart {
        index: u64,
        content_block: BlockStart,
    },
    ContentBlockDelta {
        index: u64,
        delta: Delta,
    },
    ContentBlockStop {
        index: u64,
    },
    MessageDelta {
        #[serde(default)]
        delta: MessageDeltaBody,
        usage: Option<WireUsage>,
    },
    MessageStop,
    Error {
        error: WireError,
    },
    #[serde(other)]
    Other,
}

/// The message object of a `message_start` event.
#[derive(Debug, Deserialize)]
pub(super) struct StartedMessage {
    #[serde(default)]
    pub(super) usage: WireUsage,
}

/// The block a `content_block_start` event opens.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum BlockStart {
    ToolUse {
        id: String,
        name: String,
    },
    #[serde(other)]
    Other,
}

/// The change a `content_block_delta` event carries.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum Delta {
    #[serde(rename = "text_delta")]
    Text { text: String },
    #[serde(rename = "thinking_delta")]
    Thinking { thinking: String },
    #[serde(rename = "input_json_delta")]
    InputJson { partial_json: String },
    #[serde(other)]
    Other,
}

/// The `delta` object of a `message_delta` event.
#[derive(Debug, Default, Deserialize)]
pub(super) struct MessageDeltaBody {
    pub(super) stop_reason: Option<String>,
}

/// Token counts. Every field may be missing, so a later object only replaces what it names.
#[derive(Debug, Default, Deserialize)]
pub(super) struct WireUsage {
    #[serde(rename = "input_tokens")]
    pub(super) input: Option<u64>,
    #[serde(rename = "output_tokens")]
    pub(super) output: Option<u64>,
    #[serde(rename = "cache_creation_input_tokens")]
    pub(super) cache_write: Option<u64>,
    #[serde(rename = "cache_read_input_tokens")]
    pub(super) cache_read: Option<u64>,
}

/// An error object, from an `error` event or from an HTTP error body.
#[derive(Debug, Deserialize)]
pub(super) struct WireError {
    #[serde(rename = "type", default)]
    pub(super) kind: String,
    #[serde(default)]
    pub(super) message: String,
}

/// The JSON body of an HTTP error response.
#[derive(Debug, Deserialize)]
pub(super) struct ErrorBody {
    pub(super) error: WireError,
}
