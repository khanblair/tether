//! Server-sent events parser. Every provider streams through it; nothing else parses SSE.

/// One complete event from an SSE stream.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SseEvent {
    /// The `event:` field, when the stream sent one.
    pub event: Option<String>,
    /// The `data:` lines joined with a newline.
    pub data: String,
    /// The `id:` field, when the stream sent one.
    pub id: Option<String>,
}

/// Turns arbitrary byte chunks into events. Chunk boundaries may fall anywhere.
#[derive(Debug, Default)]
pub struct SseParser {}

impl SseParser {
    /// Creates a parser with no buffered input.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds the next chunk and returns every event it completed.
    pub fn push(&mut self, _chunk: &[u8]) -> Vec<SseEvent> {
        Vec::new()
    }
}
