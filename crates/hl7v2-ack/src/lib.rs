//! HL7 v2 ACK (Acknowledgment) message generation.
//!
//! This crate provides functionality for generating HL7 v2 acknowledgment messages
//! in response to received HL7 messages. ACK messages are used to confirm receipt
//! and processing status of HL7 messages.
//!
//! # Example
//!
//! ```
//! use hl7v2_core::{Message, parse};
//! use hl7v2_ack::{ack, AckCode};
//!
//! let original_message = hl7v2_core::parse(
//!     b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\r"
//! ).unwrap();
//!
//! let ack_message = ack(&original_message, AckCode::AA).unwrap();
//! ```

use hl7v2_core::{Message, Error, Segment, Field, Rep, Comp, Atom};

/// ACK codes for HL7 v2 acknowledgment messages.
///
/// These codes indicate the status of message processing:
/// - **Application Accept (AA)**: The message was accepted and processed successfully
/// - **Application Error (AE)**: The message was accepted but processing failed
/// - **Application Reject (AR)**: The message was rejected (e.g., invalid format)
/// - **Commit Accept (CA)**: Used in enhanced mode for commit-level acknowledgment
/// - **Commit Error (CE)**: Used in enhanced mode for commit-level error
/// - **Commit Reject (CR)**: Used in enhanced mode for commit-level reject
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckCode {
    /// Application Accept - Message accepted and processed successfully
    AA,
    /// Application Error - Message accepted but processing failed
    AE,
    /// Application Reject - Message rejected
    AR,
    /// Commit Accept - Enhanced mode commit acknowledgment
    CA,
    /// Commit Error - Enhanced mode commit error
    CE,
    /// Commit Reject - Enhanced mode commit reject
    CR,
}

impl AckCode {
    /// Returns the string representation of the ACK code.
    pub fn as_str(&self) -> &'static str {
        match self {
            AckCode::AA => "AA",
            AckCode::AE => "AE",
            AckCode::AR => "AR",
            AckCode::CA => "CA",
            AckCode::CE => "CE",
            AckCode::CR => "CR",
        }
    }
}

impl std::fmt::Display for AckCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Generate a single ACK message in response to an original HL7 message.
///
/// This function creates an acknowledgment message with the same delimiters
/// as the original message, containing MSH and MSA segments.
///
/// # Arguments
///
/// * `original` - The original HL7 message to acknowledge
/// * `code` - The acknowledgment code indicating processing status
///
/// # Returns
///
/// A new `Message` containing the ACK response, or an error if the original
/// message is invalid.
///
/// # Example
///
/// ```
/// use hl7v2_core::parse;
/// use hl7v2_ack::{ack, AckCode};
///
/// let original = parse(
///     b"MSH|^~\\&|App1|Fac1|App2|Fac2|20250128120000||ADT^A01|MSG001|P|2.5.1\r"
/// ).unwrap();
///
/// let ack_msg = ack(&original, AckCode::AA).unwrap();
/// assert_eq!(ack_msg.segments.len(), 2);
/// ```
pub fn ack(original: &Message, code: AckCode) -> Result<Message, Error> {
    // Create ACK message with same delimiters as original
    let delims = original.delims.clone();
    
    // Create MSH segment for ACK
    let msh_segment = create_ack_msh_segment(original, code)?;
    
    // Create MSA segment
    let msa_segment = create_msa_segment(original, code)?;
    
    Ok(Message {
        delims,
        segments: vec![msh_segment, msa_segment],
        charsets: vec![]
    })
}

/// Create MSH segment for ACK message.
///
/// The MSH segment for an ACK message mirrors the original MSH segment
/// with swapped sending/receiving applications and facilities.
fn create_ack_msh_segment(original: &Message, _code: AckCode) -> Result<Segment, Error> {
    // Get the original MSH segment
    let original_msh = original.segments.first().ok_or(Error::InvalidSegmentId)?;
    if &original_msh.id != b"MSH" {
        return Err(Error::InvalidSegmentId);
    }
    
    // Extract required fields from original MSH
    // Note: For MSH, field indices are offset by 1 because MSH-1 is the field separator |
    let sending_app = get_field_value(original_msh, 2).unwrap_or_else(|| "HL7V2RS".to_string());
    let sending_fac = get_field_value(original_msh, 3).unwrap_or_else(|| "HL7V2RS".to_string());
    let receiving_app = get_field_value(original_msh, 4).unwrap_or_else(|| "".to_string());
    let receiving_fac = get_field_value(original_msh, 5).unwrap_or_else(|| "".to_string());
    let message_type = get_field_value(original_msh, 8).unwrap_or_else(|| "ACK".to_string());
    let control_id = get_field_value(original_msh, 9).unwrap_or_else(|| "".to_string());
    let processing_id = get_field_value(original_msh, 10).unwrap_or_else(|| "P".to_string());
    let version = get_field_value(original_msh, 11).unwrap_or_else(|| "2.5.1".to_string());
    
    // Create timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    
    // Create fields for MSH segment
    let mut fields = Vec::new();
    
    // MSH-2: Encoding characters
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(format!("{}{}{}{}", 
                    original.delims.comp, original.delims.rep, 
                    original.delims.esc, original.delims.sub))],
            }],
        }],
    });
    
    // MSH-3: Sending Application (swap with original receiving)
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(receiving_app)],
            }],
        }],
    });
    
    // MSH-4: Sending Facility (swap with original receiving)
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(receiving_fac)],
            }],
        }],
    });
    
    // MSH-5: Receiving Application (swap with original sending)
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(sending_app)],
            }],
        }],
    });
    
    // MSH-6: Receiving Facility (swap with original sending)
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(sending_fac)],
            }],
        }],
    });
    
    // MSH-7: Date/Time of Message
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(timestamp)],
            }],
        }],
    });
    
    // MSH-8: Security (optional, leave empty)
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(String::new())],
            }],
        }],
    });
    
    // MSH-9: Message Type - use ACK or the original type
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(message_type)],
            }],
        }],
    });
    
    // MSH-10: Message Control ID
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(control_id)],
            }],
        }],
    });
    
    // MSH-11: Processing ID
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(processing_id)],
            }],
        }],
    });
    
    // MSH-12: Version ID
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(version)],
            }],
        }],
    });
    
    Ok(Segment {
        id: *b"MSH",
        fields,
    })
}

/// Create MSA segment for ACK message.
///
/// The MSA (Message Acknowledgment) segment contains the acknowledgment code
/// and the message control ID of the original message.
fn create_msa_segment(original: &Message, code: AckCode) -> Result<Segment, Error> {
    // Get the original MSH segment for control ID
    let original_msh = original.segments.first().ok_or(Error::InvalidSegmentId)?;
    if &original_msh.id != b"MSH" {
        return Err(Error::InvalidSegmentId);
    }
    
    // Get message control ID from original MSH-10
    let control_id = get_field_value(original_msh, 9).unwrap_or_else(|| "".to_string());
    
    // Create fields for MSA segment
    let mut fields = Vec::new();
    
    // MSA-1: Acknowledgment Code
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(code.as_str().to_string())],
            }],
        }],
    });
    
    // MSA-2: Message Control ID
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(control_id)],
            }],
        }],
    });
    
    Ok(Segment {
        id: *b"MSA",
        fields,
    })
}

/// Get field value from a segment.
///
/// This helper function extracts the text value from the first repetition,
/// first component, first subcomponent of a field at the given 1-based index.
fn get_field_value(segment: &Segment, field_index: usize) -> Option<String> {
    if field_index > segment.fields.len() {
        return None;
    }
    
    let field = &segment.fields[field_index - 1];
    if field.reps.is_empty() {
        return None;
    }
    
    let rep = &field.reps[0];
    if rep.comps.is_empty() {
        return None;
    }
    
    let comp = &rep.comps[0];
    if comp.subs.is_empty() {
        return None;
    }
    
    match &comp.subs[0] {
        Atom::Text(text) => Some(text.clone()),
        Atom::Null => None,
    }
}

/// Generate an ACK message with an optional ERR segment.
///
/// This function creates an acknowledgment message that includes an ERR segment
/// for reporting errors when the acknowledgment code indicates an error or rejection.
///
/// # Arguments
///
/// * `original` - The original HL7 message to acknowledge
/// * `code` - The acknowledgment code (typically AE or AR)
/// * `error_message` - Optional error message to include in ERR segment
///
/// # Returns
///
/// A new `Message` containing the ACK response with optional ERR segment
pub fn ack_with_error(
    original: &Message, 
    code: AckCode, 
    error_message: Option<&str>
) -> Result<Message, Error> {
    let mut ack_msg = ack(original, code)?;
    
    if let Some(msg) = error_message {
        let err_segment = create_err_segment(msg);
        ack_msg.segments.push(err_segment);
    }
    
    Ok(ack_msg)
}

/// Create an ERR segment with an error message.
///
/// The ERR segment is used to report errors in message processing.
fn create_err_segment(error_message: &str) -> Segment {
    let mut fields = Vec::new();
    
    // ERR-1: Error Code and Location (using segment ID and field)
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(String::new())],
            }],
        }],
    });
    
    // ERR-2: Error Location (HL7 table 0535)
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(String::new())],
            }],
        }],
    });
    
    // ERR-3: HL7 Error Code (HL7 table 0396)
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(error_message.to_string())],
            }],
        }],
    });
    
    Segment {
        id: *b"ERR",
        fields,
    }
}

#[cfg(test)]
mod tests;
