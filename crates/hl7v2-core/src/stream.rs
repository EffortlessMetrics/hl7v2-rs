use std::collections::VecDeque;
use std::io::BufRead;

use crate::{Delims, Error, Event};

/// Streaming parser for HL7 v2 messages
pub struct StreamParser<D> {
    /// Reader for input data
    reader: D,
    /// Current delimiters (starts with default, switches per message)
    delims: Delims,
    /// Buffer for accumulating data
    buffer: Vec<u8>,
    /// Current position in buffer
    pos: usize,
    /// Whether we're in pre-MSH mode
    pre_msh: bool,
    /// Whether we've started parsing a message
    in_message: bool,
    /// Queue of events to be returned
    event_queue: VecDeque<Event>,
}

impl<D: BufRead> StreamParser<D> {
    /// Create a new streaming parser
    pub fn new(reader: D) -> Self {
        Self {
            reader,
            delims: Delims::default(),
            buffer: Vec::new(),
            pos: 0,
            pre_msh: true,
            in_message: false,
            event_queue: VecDeque::new(),
        }
    }

    /// Get the next event from the stream
    pub fn next_event(&mut self) -> Result<Option<Event>, Error> {
        // First check if we have any queued events
        if let Some(event) = self.event_queue.pop_front() {
            return Ok(Some(event));
        }

        loop {
            // If we're at the end of our buffer, try to read more data
            if self.pos >= self.buffer.len() {
                let mut temp_buf = vec![0u8; 1024];
                match self.reader.read(&mut temp_buf) {
                    Ok(0) => {
                        // End of input
                        if self.in_message {
                            self.in_message = false;
                            self.pre_msh = true;
                            return Ok(Some(Event::EndMessage));
                        }
                        return Ok(None);
                    }
                    Ok(n) => {
                        // Add the new data to our buffer
                        self.buffer.extend_from_slice(&temp_buf[..n]);
                    }
                    Err(_) => return Err(Error::InvalidCharset),
                }
            }

            // Look for a complete segment (ending with \r)
            if let Some(cr_pos) = self.buffer[self.pos..].iter().position(|&b| b == b'\r') {
                let segment_end = self.pos + cr_pos;
                let segment_data = self.buffer[self.pos..segment_end].to_vec();
                self.pos = segment_end + 1; // Skip the \r

                // Check if this is an MSH segment
                if segment_data.starts_with(b"MSH") {
                    // If we were already in a message, end it
                    if self.in_message {
                        self.event_queue.push_back(Event::EndMessage);
                    }

                    // Parse delimiters from MSH segment
                    if segment_data.len() >= 8 {
                        let field_sep = segment_data[3] as char;
                        let comp_char = segment_data[4] as char;
                        let rep_char = segment_data[5] as char;
                        let esc_char = segment_data[6] as char;
                        let sub_char = segment_data[7] as char;

                        self.delims = Delims {
                            field: field_sep,
                            comp: comp_char,
                            rep: rep_char,
                            esc: esc_char,
                            sub: sub_char,
                        };
                    }

                    // Start new message
                    self.in_message = true;
                    self.pre_msh = false;
                    self.event_queue
                        .push_back(Event::StartMessage { delims: self.delims.clone() });
                } else if self.pre_msh {
                    // Skip anything before the first MSH
                    continue;
                }

                // Parse segment
                if segment_data.len() >= 3 {
                    let segment_id = segment_data[..3].to_vec();
                    self.event_queue.push_back(Event::Segment { id: segment_id });

                    // Parse fields if this isn't an MSH segment's field separator
                    if segment_data.len() > 4 {
                        let fields_data = &segment_data[4..]; // Skip segment ID and field separator
                        let fields: Vec<&[u8]> = fields_data
                            .split(|&b| b == self.delims.field as u8)
                            .collect();

                        for (index, field) in fields.iter().enumerate() {
                            let field_num = (index + 1) as u16;
                            self.event_queue
                                .push_back(Event::Field { num: field_num, raw: field.to_vec() });
                        }
                    }
                }
                return Ok(self.event_queue.pop_front());
            }
        }
    }
}
