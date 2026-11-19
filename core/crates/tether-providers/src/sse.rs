//! Server-sent events parser. Every provider streams through it; nothing else parses SSE.
//!
//! The rules follow the WHATWG event stream parsing algorithm. Two safety limits are added so a
//! hostile server cannot grow memory without bound: a line longer than `MAX_LINE_BYTES` is
//! dropped, and an event whose data passes `MAX_EVENT_BYTES` is dropped whole.

use std::borrow::Cow;

/// The longest line the parser keeps. A longer line is dropped up to its line ending.
const MAX_LINE_BYTES: usize = 1_048_576;

/// The most data one event may carry. A larger event is dropped whole.
const MAX_EVENT_BYTES: usize = 8 * 1_048_576;

/// The UTF-8 byte order mark. It is skipped when it starts the stream.
const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// The data lines of the event being read.
#[derive(Debug, Default)]
enum DataBuffer {
    /// No data line has arrived since the last dispatch.
    #[default]
    Empty,
    /// Data lines so far, each followed by LF.
    Text(String),
    /// The event passed `MAX_EVENT_BYTES` and is being dropped.
    TooLarge,
}

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
pub struct SseParser {
    /// Bytes of the current line, without its line ending.
    line: Vec<u8>,
    /// True while the bytes of an over-long line are being dropped.
    discarding: bool,
    /// True when the last byte was a CR. An LF right after it belongs to the same line ending.
    pending_cr: bool,
    /// True once the start of the stream is known, so no more BOM checks are needed.
    bom_done: bool,
    /// Leading bytes that may still be the start of a BOM.
    bom_prefix: Vec<u8>,
    /// Data lines of the current event.
    data: DataBuffer,
    /// The `event` field of the current block.
    event: Option<String>,
    /// The last `id` field of the current block.
    id: Option<String>,
}

impl SseParser {
    /// Creates a parser with no buffered input.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds the next chunk and returns every event it completed.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        let mut events = Vec::new();
        let body = self.skip_bom(chunk);
        for &byte in &*body {
            self.feed_byte(byte, &mut events);
        }
        events
    }

    /// Returns the part of `chunk` that follows a leading BOM.
    ///
    /// The BOM may be split across chunks, so bytes that might still be a BOM are kept in
    /// `bom_prefix` until the check is decided.
    fn skip_bom<'a>(&mut self, chunk: &'a [u8]) -> Cow<'a, [u8]> {
        if self.bom_done {
            return Cow::Borrowed(chunk);
        }
        let mut seen = std::mem::take(&mut self.bom_prefix);
        seen.extend_from_slice(chunk);
        let comparable = seen.len().min(BOM.len());
        if seen[..comparable] != BOM[..comparable] {
            self.bom_done = true;
            return Cow::Owned(seen);
        }
        if seen.len() < BOM.len() {
            self.bom_prefix = seen;
            return Cow::Borrowed(&[]);
        }
        self.bom_done = true;
        Cow::Owned(seen.split_off(BOM.len()))
    }

    /// Handles one byte, ending a line or adding to the current one.
    fn feed_byte(&mut self, byte: u8, events: &mut Vec<SseEvent>) {
        // An LF right after a CR is the second half of the same line ending.
        if std::mem::take(&mut self.pending_cr) && byte == b'\n' {
            return;
        }
        match byte {
            b'\r' => {
                self.end_line(events);
                self.pending_cr = true;
            }
            b'\n' => self.end_line(events),
            _ => self.append(byte),
        }
    }

    /// Adds one byte to the current line. Drops the line once it passes the size limit.
    fn append(&mut self, byte: u8) {
        if self.discarding {
            return;
        }
        if self.line.len() == MAX_LINE_BYTES {
            self.discarding = true;
            self.line.clear();
            return;
        }
        self.line.push(byte);
    }

    /// Ends the current line. An empty line dispatches the pending event.
    fn end_line(&mut self, events: &mut Vec<SseEvent>) {
        let line = std::mem::take(&mut self.line);
        if std::mem::take(&mut self.discarding) {
            return;
        }
        if line.is_empty() {
            self.dispatch(events);
        } else {
            self.process_line(&line);
        }
    }

    /// Applies one non-empty line: a comment, or a `field: value` pair.
    fn process_line(&mut self, line: &[u8]) {
        if line.first() == Some(&b':') {
            return;
        }
        let text = String::from_utf8_lossy(line);
        let (field, value) = match text.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            None => (&text[..], ""),
        };
        match field {
            "event" => self.event = Some(value.to_owned()),
            "data" => self.append_data(value),
            "id" if !value.contains('\0') => self.id = Some(value.to_owned()),
            _ => {}
        }
    }

    /// Adds one data line to the data buffer.
    fn append_data(&mut self, value: &str) {
        match &mut self.data {
            DataBuffer::TooLarge => {}
            DataBuffer::Empty => self.data = DataBuffer::Text(format!("{value}\n")),
            DataBuffer::Text(text) if text.len() + value.len() + 1 > MAX_EVENT_BYTES => {
                self.data = DataBuffer::TooLarge;
            }
            DataBuffer::Text(text) => {
                text.push_str(value);
                text.push('\n');
            }
        }
    }

    /// Emits the pending event, if any data was seen. Resets the event and id fields.
    fn dispatch(&mut self, events: &mut Vec<SseEvent>) {
        let event = self.event.take();
        let id = self.id.take();
        let DataBuffer::Text(mut data) = std::mem::take(&mut self.data) else {
            return;
        };
        data.pop();
        events.push(SseEvent { event, data, id });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The fixtures are hand-written to follow the providers' documented formats. They are not recorded traffic.
    const ANTHROPIC: &str = include_str!("../../../fixtures/provider-streams/anthropic-text.sse");
    const OPENAI: &str = include_str!("../../../fixtures/provider-streams/openai-chat.sse");
    const MULTILINE: &str = include_str!("../../../fixtures/provider-streams/multiline-data.sse");
    const UNICODE: &str = include_str!("../../../fixtures/provider-streams/unicode.sse");

    impl SseParser {
        fn buffered_len(&self) -> usize {
            self.line.len()
        }
    }

    fn ev(event: Option<&str>, data: &str, id: Option<&str>) -> SseEvent {
        SseEvent {
            event: event.map(str::to_owned),
            data: data.to_owned(),
            id: id.map(str::to_owned),
        }
    }

    fn parse_whole(input: &[u8]) -> Vec<SseEvent> {
        SseParser::new().push(input)
    }

    fn parse_chunks(chunks: &[&[u8]]) -> Vec<SseEvent> {
        let mut parser = SseParser::new();
        let mut events = Vec::new();
        for chunk in chunks {
            events.extend(parser.push(chunk));
        }
        events
    }

    /// Checks that every two-chunk split and the one-byte-at-a-time feed give the same events.
    fn assert_split_invariant(input: &[u8]) {
        let whole = parse_whole(input);
        for at in 0..=input.len() {
            let split = parse_chunks(&[&input[..at], &input[at..]]);
            assert_eq!(split, whole, "split at byte {at}");
        }
        let single: Vec<&[u8]> = input.chunks(1).collect();
        assert_eq!(parse_chunks(&single), whole, "one byte at a time");
    }

    #[test]
    fn anthropic_stream_parses() {
        let events = parse_whole(ANTHROPIC.as_bytes());
        assert_eq!(events.len(), 8);
        assert_eq!(events[0].event.as_deref(), Some("message_start"));
        assert_eq!(events[7].event.as_deref(), Some("message_stop"));
        assert_eq!(events[7].data, r#"{"type":"message_stop"}"#);
    }

    #[test]
    fn openai_stream_parses_without_event_names() {
        let events = parse_whole(OPENAI.as_bytes());
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].event, None);
        assert!(events[0].data.starts_with(r#"{"id":"chatcmpl-1""#));
        assert_eq!(events[4].data, "[DONE]");
    }

    #[test]
    fn multiline_fixture_parses_exactly() {
        assert_eq!(
            parse_whole(MULTILINE.as_bytes()),
            vec![
                ev(Some("multi"), "line one\nline two\nline three", Some("7")),
                ev(None, "no-space", None),
                ev(None, "", None),
            ]
        );
    }

    #[test]
    fn unicode_fixture_parses_exactly() {
        assert_eq!(
            parse_whole(UNICODE.as_bytes()),
            vec![
                ev(Some("text"), "你好，世界\nemoji 😀 done", None),
                ev(None, "日本語テキスト 🚀", None),
            ]
        );
    }

    #[test]
    fn crlf_stream_parses_like_lf() {
        let crlf = ANTHROPIC.replace('\n', "\r\n");
        assert_eq!(parse_whole(crlf.as_bytes()).len(), 8);
        assert_eq!(
            parse_whole(crlf.as_bytes()),
            parse_whole(ANTHROPIC.as_bytes())
        );
    }

    #[test]
    fn anthropic_stream_survives_every_split() {
        assert_split_invariant(ANTHROPIC.as_bytes());
    }

    #[test]
    fn openai_stream_survives_every_split() {
        assert_split_invariant(OPENAI.as_bytes());
    }

    #[test]
    fn multiline_fixture_survives_every_split() {
        assert_split_invariant(MULTILINE.as_bytes());
    }

    #[test]
    fn unicode_fixture_survives_every_split() {
        assert_split_invariant(UNICODE.as_bytes());
    }

    #[test]
    fn crlf_stream_survives_every_split() {
        let crlf = ANTHROPIC.replace('\n', "\r\n");
        assert_split_invariant(crlf.as_bytes());
    }

    #[test]
    fn lone_cr_line_endings_work() {
        assert_eq!(
            parse_whole(b"data: a\rdata: b\r\r"),
            vec![ev(None, "a\nb", None)]
        );
    }

    #[test]
    fn lf_after_a_split_cr_adds_no_blank_line() {
        let events = parse_chunks(&[&b"data: a\r"[..], &b"\ndata: b\n\n"[..]]);
        assert_eq!(events, vec![ev(None, "a\nb", None)]);
    }

    #[test]
    fn leading_bom_is_ignored_even_when_split() {
        let input = b"\xEF\xBB\xBFdata: ok\n\n";
        assert_eq!(parse_whole(input), vec![ev(None, "ok", None)]);
        assert_split_invariant(input);
    }

    #[test]
    fn bom_later_in_the_stream_is_kept() {
        assert_eq!(
            parse_whole("data: \u{feff}x\n\n".as_bytes()),
            vec![ev(None, "\u{feff}x", None)]
        );
    }

    #[test]
    fn comment_lines_are_ignored() {
        assert_eq!(
            parse_whole(b": keep-alive\ndata: a\n: ping\n\n"),
            vec![ev(None, "a", None)]
        );
    }

    #[test]
    fn unknown_fields_are_ignored() {
        assert_eq!(
            parse_whole(b"retry: 100\ndata: a\n\n"),
            vec![ev(None, "a", None)]
        );
    }

    #[test]
    fn only_one_leading_space_is_removed() {
        assert_eq!(parse_whole(b"data:  two\n\n"), vec![ev(None, " two", None)]);
    }

    #[test]
    fn id_with_nul_is_ignored() {
        assert_eq!(
            parse_whole(b"id: a\0b\ndata: x\n\n"),
            vec![ev(None, "x", None)]
        );
    }

    #[test]
    fn id_is_reset_after_each_event() {
        assert_eq!(
            parse_whole(b"id: 1\nid: 2\ndata: a\n\ndata: b\n\n"),
            vec![ev(None, "a", Some("2")), ev(None, "b", None)]
        );
    }

    #[test]
    fn event_without_data_is_not_dispatched() {
        assert_eq!(
            parse_whole(b"event: orphan\n\ndata: next\n\n"),
            vec![ev(None, "next", None)]
        );
    }

    #[test]
    fn unterminated_event_at_end_is_not_returned() {
        assert_eq!(
            parse_whole(b"data: a\n\ndata: b\n"),
            vec![ev(None, "a", None)]
        );
        assert_eq!(parse_whole(b"data: c\n"), Vec::new());
    }

    #[test]
    fn invalid_utf8_becomes_replacement_characters() {
        assert_eq!(
            parse_whole(b"data: a\xFFb\n\n"),
            vec![ev(None, "a\u{FFFD}b", None)]
        );
    }

    #[test]
    fn over_long_line_is_dropped_and_parsing_resumes() {
        let mut parser = SseParser::new();
        let long = vec![b'x'; 2 * MAX_LINE_BYTES];
        for part in long.chunks(65_536) {
            assert_eq!(parser.push(part), Vec::new());
        }
        assert!(parser.buffered_len() <= MAX_LINE_BYTES);
        let events = parser.push(b"\n\ndata: ok\n\n");
        assert_eq!(events, vec![ev(None, "ok", None)]);
    }

    #[test]
    fn line_of_exactly_the_limit_is_kept() {
        let value = "x".repeat(MAX_LINE_BYTES - "data: ".len());
        let input = format!("data: {value}\n\n");
        let events = parse_whole(input.as_bytes());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, value);
    }

    #[test]
    fn an_event_with_too_much_data_is_dropped_and_the_next_one_is_kept() {
        let mut parser = SseParser::new();
        let line = format!("data: {}\n", "x".repeat(1023));
        let mut events = Vec::new();
        for _ in 0..(MAX_EVENT_BYTES / 1024 + 2) {
            events.extend(parser.push(line.as_bytes()));
        }
        events.extend(parser.push(b"\ndata: ok\n\n"));
        assert_eq!(
            events,
            vec![SseEvent {
                data: "ok".into(),
                ..SseEvent::default()
            }]
        );
    }
}
