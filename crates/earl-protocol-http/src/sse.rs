/// A parsed Server-Sent Event.
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event_type: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

/// Stateful Server-Sent Events parser.
///
/// Buffers incomplete events across calls to [`feed`] so that events split
/// across HTTP chunk boundaries are handled correctly.
pub struct SseParser {
    /// Leftover text that hasn't yet formed a complete event block.
    buffer: String,
}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SseParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Feed a chunk of text into the parser, returning any complete events.
    ///
    /// Incomplete events are buffered internally and will be emitted on
    /// subsequent `feed()` calls once the closing blank line arrives.
    pub fn feed(&mut self, input: &str) -> Vec<SseEvent> {
        self.buffer.push_str(input);
        let mut events = Vec::new();

        // The SSE spec defines event boundaries as blank lines.
        // We support both \n\n and \r\n\r\n.
        loop {
            // Find the next event boundary (blank line).
            let boundary = self
                .buffer
                .find("\n\n")
                .map(|pos| (pos, 2))
                .or_else(|| self.buffer.find("\r\n\r\n").map(|pos| (pos, 4)));

            let Some((pos, sep_len)) = boundary else {
                break;
            };

            let block = &self.buffer[..pos];
            if let Some(event) = Self::parse_block(block) {
                events.push(event);
            }
            // Drain the consumed block + separator.
            self.buffer.drain(..pos + sep_len);
        }

        events
    }

    /// Flush any remaining buffered data as a final event.
    ///
    /// Call this when the stream ends to emit any trailing event that
    /// wasn't followed by a blank line.
    pub fn flush(&mut self) -> Option<SseEvent> {
        let block = std::mem::take(&mut self.buffer);
        let block = block.trim();
        if block.is_empty() {
            return None;
        }
        Self::parse_block(block)
    }

    fn parse_block(block: &str) -> Option<SseEvent> {
        let mut data_lines: Vec<&str> = Vec::new();
        let mut event_type: Option<String> = None;
        let mut id: Option<String> = None;

        for line in block.lines() {
            if line.starts_with(':') {
                // Comment — skip.
                continue;
            }

            if let Some(rest) = line.strip_prefix("data:") {
                let value = rest.strip_prefix(' ').unwrap_or(rest);
                data_lines.push(value);
            } else if let Some(rest) = line.strip_prefix("event:") {
                let value = rest.strip_prefix(' ').unwrap_or(rest);
                event_type = Some(value.to_string());
            } else if let Some(rest) = line.strip_prefix("id:") {
                let value = rest.strip_prefix(' ').unwrap_or(rest);
                id = Some(value.to_string());
            }
            // Unknown fields are ignored per the SSE spec.
        }

        // Only emit an event if there was at least one data line.
        if data_lines.is_empty() {
            return None;
        }

        Some(SseEvent {
            event_type,
            data: data_lines.join("\n"),
            id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_data_line_returned_as_event_data() {
        let input = "data: hello world\n\n";
        let events = SseParser::new().feed(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello world");
    }

    #[test]
    fn multiple_data_lines_joined_with_newline() {
        let input = "data: line1\ndata: line2\n\n";
        let events = SseParser::new().feed(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn event_field_sets_event_type() {
        let input = "event: update\ndata: {\"key\":\"value\"}\n\n";
        let events = SseParser::new().feed(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type.as_deref(), Some("update"));
    }

    #[test]
    fn comment_lines_excluded_from_event_data() {
        let input = ": this is a comment\ndata: actual data\n\n";
        let events = SseParser::new().feed(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "actual data");
    }

    #[test]
    fn multiple_complete_events_all_returned() {
        let input = "data: event1\n\ndata: event2\n\n";
        let events = SseParser::new().feed(input);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn id_field_sets_event_id() {
        let input = "id: 42\ndata: payload\n\n";
        let events = SseParser::new().feed(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id.as_deref(), Some("42"));
    }

    #[test]
    fn no_space_after_colon_data_is_parsed() {
        let input = "data:no-space\n\n";
        let events = SseParser::new().feed(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "no-space");
    }

    #[test]
    fn block_without_data_field_produces_no_event() {
        let input = "event: ping\n\n";
        let events = SseParser::new().feed(input);
        assert!(events.is_empty());
    }

    #[test]
    fn empty_input_returns_no_events() {
        let events = SseParser::new().feed("");
        assert!(events.is_empty());
    }

    #[test]
    fn event_split_across_chunks_buffered_until_complete() {
        let mut parser = SseParser::new();

        // First chunk contains the beginning of the event but no blank-line terminator.
        let events = parser.feed("data: hel");
        assert!(events.is_empty(), "no complete event yet");

        // Second chunk completes the event with the rest of the data + blank line.
        let events = parser.feed("lo world\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello world");
    }

    #[test]
    fn crlf_line_endings_parse_event_type() {
        let input = "event: update\r\ndata: payload\r\n\r\n";
        let events = SseParser::new().feed(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type.as_deref(), Some("update"));
    }

    #[test]
    fn crlf_line_endings_parse_data() {
        let input = "event: update\r\ndata: payload\r\n\r\n";
        let events = SseParser::new().feed(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "payload");
    }

    #[test]
    fn trailing_data_without_terminator_emitted_on_flush() {
        let mut parser = SseParser::new();

        // Feed an event that is NOT terminated by a blank line.
        let events = parser.feed("data: trailing");
        assert!(events.is_empty(), "no blank line yet, so nothing emitted");

        // Flush should emit the trailing event.
        let event = parser.flush().expect("should flush trailing event");
        assert_eq!(event.data, "trailing");
    }

    #[test]
    fn complete_event_before_partial_in_same_feed_emitted_immediately() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: first\n\ndata: sec");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "first");
    }

    #[test]
    fn subsequent_feed_completes_partial_and_returns_additional_events() {
        let mut parser = SseParser::new();
        // Setup: buffer a partial event.
        parser.feed("data: first\n\ndata: sec");
        // Second feed completes the partial and delivers a further event.
        let events = parser.feed("ond\n\ndata: third\n\n");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "second");
        assert_eq!(events[1].data, "third");
    }
}
