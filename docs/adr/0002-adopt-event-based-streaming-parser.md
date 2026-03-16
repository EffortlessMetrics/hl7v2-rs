# ADR-0002: Adopt Event-Based Streaming Parser

**Status**: Accepted

**Date**: 2025-11-19

**Deciders**: Architecture team

**Technical Story**: Large HL7v2 batch files containing thousands of messages cause excessive memory consumption when parsed entirely into memory. A streaming approach is needed to process messages incrementally with bounded memory usage and backpressure support.

## Context

HL7v2 messages in production healthcare environments vary dramatically in size and volume:

- **Individual messages** are typically 500 bytes to 10 KB (e.g., a single ADT^A01 admission event).
- **Batch files** wrapped in FHS/BHS/BTS/FTS envelopes can contain thousands of messages, reaching tens or hundreds of megabytes (e.g., nightly lab result dumps, insurance claim batches).
- **Streaming feeds** over MLLP connections deliver a continuous flow of messages that may never "end" (e.g., real-time ADT feeds from a hospital HIS).

The existing one-shot parser (`hl7v2-parser`) loads the entire message into memory, constructs a full `Message` tree (segments, fields, repetitions, components, sub-components), and returns it. This approach has several limitations:

1. **Memory pressure** -- Batch files with thousands of messages require the entire file to be resident in memory before processing can begin.
2. **Latency** -- Processing cannot start until the complete message or batch is received and parsed.
3. **Backpressure** -- No mechanism to slow down producers when consumers cannot keep up (e.g., validation is slower than parsing).
4. **Composability** -- Downstream consumers (validators, transformers, routers) must accept a fully-materialized `Message` object even when they only need specific segments or fields.

The system needs a parser that can process HL7v2 data incrementally, emitting structured events as it encounters message boundaries, segments, and fields, without requiring the entire message to be in memory simultaneously.

## Decision

We will implement an **event-based streaming parser** (SAX-like model) in the `hl7v2-stream` crate. The parser emits `Event` values as it encounters different parts of the HL7v2 message structure, allowing consumers to process data incrementally.

The design consists of four components:

1. **`Event` enum** -- Represents the four structural events in an HL7v2 message:
   - `StartMessage { delims }` -- Beginning of a message, carrying the discovered delimiters.
   - `Segment { id }` -- A segment with its 3-character identifier (e.g., `MSH`, `PID`, `OBX`).
   - `Field { num, raw }` -- A field with its 1-based number and raw byte content.
   - `EndMessage` -- End of the current message.

2. **`StreamParser<D: BufRead>`** -- Synchronous streaming parser that reads from any `BufRead` source.

3. **`AsyncStreamParser`** -- Asynchronous version using `tokio::sync::mpsc` bounded channel for backpressure.

4. **`StreamParserBuilder`** -- Builder pattern for configuring buffer size (default: 100 events) and maximum message size (default: 1 MB).

**Rationale:**

1. **Bounded memory** -- Only the current segment is held in memory at any time, not the entire message or batch.
2. **Immediate processing** -- Events are emitted as soon as a segment boundary (`\r`) is encountered; no need to wait for the entire message.
3. **Backpressure** -- The async variant uses a bounded `mpsc` channel; when the consumer falls behind, the parser blocks on `send`, naturally throttling the producer.
4. **Delimiter discovery** -- The parser automatically detects delimiters from each MSH segment and uses them for the duration of that message.
5. **Memory safety** -- The `max_message_size` limit prevents unbounded memory growth from malformed or malicious input.
6. **Composability** -- Consumers can filter, transform, or route events without materializing full `Message` objects.

## Consequences

### Positive

- **Constant memory usage** -- Processing a 100 MB batch file uses the same memory as processing a 1 KB message; only one segment is buffered at a time, plus the event queue (`VecDeque`).
- **Low latency** -- First event is emitted as soon as the MSH segment is read; consumers can begin processing immediately without waiting for the full message.
- **Natural backpressure** -- The bounded `mpsc` channel (default capacity: 100 events) automatically pauses the parser when consumers cannot keep up, preventing unbounded buffering.
- **Memory bounds enforcement** -- The `max_message_size` parameter (default: 1 MB) prevents a single malformed message from consuming all available memory, returning a `MessageTooLarge` error.
- **Flexible consumption** -- Consumers can filter events by segment ID (e.g., only process `PID` and `OBX` segments), skip irrelevant fields, or route events to different handlers, all without materializing the full message tree.
- **Dual sync/async API** -- `StreamParser` works in synchronous contexts (CLI tools, batch scripts), while `AsyncStreamParser` integrates with the Tokio-based server and network stack.

### Negative

- **No random access** -- Unlike the DOM-style `Message` type, the streaming parser cannot go back to previously emitted segments or fields. Consumers needing random access must buffer events themselves.
- **Complex state management** -- Consumers must track parser state (current segment, field context) across multiple event callbacks, which is more complex than working with a fully parsed `Message` object.
- **Two parser APIs** -- The codebase now has two parsing approaches (one-shot in `hl7v2-parser`, streaming in `hl7v2-stream`), which increases the surface area developers must understand.
- **Field-level granularity only** -- Events are emitted at the field level; repetitions, components, and sub-components within a field must still be parsed from the raw bytes by the consumer.

### Neutral

- **Complements rather than replaces** -- The streaming parser does not replace `hl7v2-parser`; both coexist. The one-shot parser remains the right choice for single-message scenarios where random access is needed, while the streaming parser is for large batches and continuous feeds.
- **Event ordering contract** -- Consumers must handle events in a fixed order: `StartMessage` -> (`Segment` -> `Field`*)\* -> `EndMessage`. This is implicit in the API but not enforced by the type system.

## Alternatives Considered

### Alternative 1: DOM-Style Full Parse (Load Entire Message)

**Pros:**
- Simpler API; consumers get a complete `Message` object with random access to any segment or field.
- Already implemented in `hl7v2-parser`; no new code needed.
- Pattern matching and path-based queries (`hl7v2-query`, `hl7v2-path`) work directly on the `Message` type.

**Cons:**
- Memory usage is proportional to message/batch size; 100 MB batch files require 100+ MB of RAM.
- Processing cannot begin until the entire message is parsed.
- No backpressure mechanism; the parser always runs to completion.
- For batch files, all messages must be parsed before any can be processed.

**Why not chosen:**
Unacceptable memory usage for batch processing scenarios. Healthcare batch files routinely contain thousands of messages and can reach hundreds of megabytes. A system processing multiple concurrent batch imports would exhaust memory quickly.

### Alternative 2: Line-by-Line Text Processing

**Pros:**
- Extremely simple implementation; read lines delimited by `\r` and process each.
- Low memory usage (one line at a time).
- No complex parser state to manage.

**Cons:**
- No structural awareness; the processor must manually detect segment boundaries, parse field separators, and track message boundaries.
- Delimiter handling is ad hoc; no automatic delimiter discovery from MSH.
- No event abstraction; every consumer must reimplement segment/field extraction.
- Error handling is difficult without structural context.

**Why not chosen:**
Too low-level. Every consumer would need to reimplement HL7v2 structural parsing. The event-based approach provides the same memory efficiency but with meaningful structural events that abstract away the byte-level details.

### Alternative 3: Custom Binary Protocol Parser (nom/winnow)

**Pros:**
- Parser combinator libraries (`nom`, `winnow`) provide composable, high-performance parsers.
- Can express complex grammars declaratively.
- Zero-copy parsing is possible.

**Cons:**
- HL7v2 is a text protocol with configurable delimiters, not a binary protocol; parser combinators add complexity without proportional benefit.
- Delimiter discovery (reading delimiters from MSH) doesn't fit the parser combinator model cleanly, since the "grammar" changes per message.
- Learning curve for parser combinator libraries.
- Overkill for a protocol that is essentially delimiter-separated text with a simple segment structure.

**Why not chosen:**
HL7v2's grammar is simple enough (delimiter-separated fields within CR-delimited segments) that a hand-written event-based parser is more readable and maintainable than a parser combinator approach. The configurable delimiters per message make a declarative grammar awkward.

### Alternative 4: Iterator-Based Pull Parser

**Pros:**
- Idiomatic Rust; implements `Iterator<Item = Result<Event, Error>>`.
- Consumers use familiar `for` loops, `.filter()`, `.map()`, etc.
- No channel overhead for the synchronous case.

**Cons:**
- Harder to add async support; `async Iterator` (Stream) is not yet stabilized.
- Backpressure in async contexts requires wrapping in a channel anyway.
- Less control over batching and buffering of events.

**Why not chosen:**
The `next_event()` method on `StreamParser` is functionally equivalent to a pull-based iterator. We chose an explicit method over `impl Iterator` to allow the async variant to use `tokio::sync::mpsc` channels for backpressure without impedance mismatch. A future version could add an `Iterator` adapter trivially.

## Implementation Notes

### Event Enum

The `Event` enum models the four structural boundaries in an HL7v2 message:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Start of a new message with discovered delimiters
    StartMessage { delims: Delims },
    /// A segment with its 3-character ID
    Segment { id: Vec<u8> },
    /// A field with its 1-based number and raw content
    Field { num: u16, raw: Vec<u8> },
    /// End of message
    EndMessage,
}
```

### Synchronous StreamParser

The `StreamParser<D: BufRead>` reads from any `BufRead` source. It uses a `VecDeque<Event>` as an internal event queue to batch field events for a segment, then returns them one at a time via `next_event()`:

```rust
pub struct StreamParser<D> {
    reader: D,
    delims: Delims,
    buffer: Vec<u8>,
    pos: usize,
    pre_msh: bool,
    in_message: bool,
    event_queue: VecDeque<Event>,
    max_message_size: usize,
    current_message_size: usize,
}

impl<D: BufRead> StreamParser<D> {
    pub fn new(reader: D) -> Self { /* ... */ }
    pub fn next_event(&mut self) -> Result<Option<Event>, Error> { /* ... */ }
}
```

Key behaviors:
- **Delimiter discovery**: When the parser encounters an MSH segment, it calls `Delims::parse_from_msh()` to extract the field separator and encoding characters, then uses those delimiters for all subsequent segments in that message.
- **Event queuing**: When a segment is parsed, all its field events are pushed onto the `event_queue` (`VecDeque`), then drained one at a time on subsequent `next_event()` calls. This is more efficient than returning a single event per `next_event()` call when the caller is consuming all events.
- **Message boundary detection**: A new MSH segment while `in_message` is `true` triggers an `EndMessage` event for the previous message before starting the new one.
- **Memory bounds**: Each segment's size is accumulated in `current_message_size`; if it exceeds `max_message_size`, the parser returns an error and resets state.

### Async StreamParser with Backpressure

The `AsyncStreamParser` wraps a synchronous `StreamParser` in a Tokio task, communicating events through a bounded `mpsc` channel:

```rust
pub struct AsyncStreamParser {
    receiver: Receiver<Result<Event, StreamError>>,
}

impl AsyncStreamParser {
    pub async fn next(&mut self) -> Option<Result<Event, StreamError>> {
        self.receiver.recv().await
    }
}
```

The producer task runs the synchronous parser in a `tokio::spawn` block. When the channel buffer (default: 100 events) is full, the `tx.send().await` call pauses the parser until the consumer reads from the receiver, providing natural backpressure.

### Builder Pattern

The `StreamParserBuilder` provides configuration for both sync and async parsers:

```rust
let mut parser = StreamParserBuilder::new()
    .buffer_size(100)           // Async channel capacity (events)
    .max_message_size(1024 * 1024) // 1 MB limit per message
    .build_async(data);         // Or .build(reader) for sync
```

### Error Types

The `StreamError` enum covers the three failure modes specific to streaming:

```rust
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum StreamError {
    #[error("Message size {actual} exceeds maximum allowed size {max}")]
    MessageTooLarge { actual: usize, max: usize },

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Channel error: {0}")]
    ChannelError(String),
}
```

### Usage Example

```rust
use hl7v2_stream::{StreamParser, Event};
use std::io::{BufReader, Cursor};

let hl7 = b"MSH|^~\\&|App|Fac|Recv|RFac|20250128||ADT^A01|123|P|2.5.1\rPID|1||456^^^HOSP^MR||Doe^John\r";
let reader = BufReader::new(Cursor::new(&hl7[..]));
let mut parser = StreamParser::new(reader);

while let Ok(Some(event)) = parser.next_event() {
    match event {
        Event::StartMessage { delims } => { /* new message */ }
        Event::Segment { id } => { /* segment boundary */ }
        Event::Field { num, raw } => { /* field data */ }
        Event::EndMessage => { /* message complete */ }
    }
}
```

## References

- [SAX (Simple API for XML)](https://www.saxproject.org/) -- The event-based parsing model that inspired this design.
- [HL7 Version 2 Messaging Standard](https://www.hl7.org/implement/standards/product_brief.cfm?product_id=185)
- [HL7v2 Batch Protocol (FHS/BHS/BTS/FTS)](https://hl7-definition.caristix.com/v2/HL7v2.5.1/Segments)
- [Tokio mpsc Channel](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)
- [Rust BufRead Trait](https://doc.rust-lang.org/std/io/trait.BufRead.html)
- [VecDeque for Efficient Queuing](https://doc.rust-lang.org/std/collections/struct.VecDeque.html)
