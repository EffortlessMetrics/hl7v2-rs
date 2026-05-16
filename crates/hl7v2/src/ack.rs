//! HL7 v2 ACK (Acknowledgment) message generation.
//!
//! This module provides functionality for generating HL7 v2 acknowledgment messages
//! in response to received HL7 messages. ACK messages are used to confirm receipt
//! and processing status of HL7 messages.
//!
//! # Example
//!
//! ```
//! use hl7v2::{AckCode, Message, ack, parse};
//!
//! let original_message = parse(
//!     b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\r"
//! ).unwrap();
//!
//! let ack_message = ack(&original_message, AckCode::AA).unwrap();
//! ```

use crate::model::{Atom, Comp, Error, Field, Message, Rep, Segment};

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
/// # Errors
///
/// Returns [`Error::InvalidSegmentId`] when the original message is empty or
/// does not start with an `MSH` segment.
///
/// # Example
///
/// ```
/// use hl7v2::{AckCode, ack, parse};
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
    let msh_segment = create_ack_msh_segment(original)?;

    // Create MSA segment
    let msa_segment = create_msa_segment(original, code)?;

    Ok(Message {
        delims,
        segments: vec![msh_segment, msa_segment],
        charsets: vec![],
    })
}

/// Create MSH segment for ACK message.
///
/// The MSH segment for an ACK message mirrors the original MSH segment
/// with swapped sending/receiving applications and facilities.
fn create_ack_msh_segment(original: &Message) -> Result<Segment, Error> {
    // Get the original MSH segment
    let original_msh = original.segments.first().ok_or(Error::InvalidSegmentId)?;
    if &original_msh.id != b"MSH" {
        return Err(Error::InvalidSegmentId);
    }

    // Extract required fields from original MSH
    // Note: For MSH, field indices are offset by 1 because MSH-1 is the field separator |
    let sending_app = get_field_value(original_msh, 2).unwrap_or_else(|| "HL7V2RS".to_string());
    let sending_fac = get_field_value(original_msh, 3).unwrap_or_else(|| "HL7V2RS".to_string());
    let receiving_app = get_field_value(original_msh, 4).unwrap_or_default();
    let receiving_fac = get_field_value(original_msh, 5).unwrap_or_default();
    let message_type = get_field_value(original_msh, 8).unwrap_or_else(|| "ACK".to_string());
    let control_id = get_field_value(original_msh, 9).unwrap_or_default();
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
                subs: vec![Atom::Text(format!(
                    "{}{}{}{}",
                    original.delims.comp,
                    original.delims.rep,
                    original.delims.esc,
                    original.delims.sub
                ))],
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

    // MSH-9: Message Type - should be "ACK^MessageType^TriggerEvent"
    // Format: ACK^MessageType^TriggerEvent (2 components)
    // The original message_type may contain ^ separators (e.g., "ADT^A01")
    // We preserve it as a single component to maintain the original format
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![
                Comp {
                    subs: vec![Atom::Text("ACK".to_string())],
                },
                Comp {
                    subs: vec![Atom::Text(message_type.clone())],
                },
            ],
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
    let control_id = get_field_value(original_msh, 9).unwrap_or_default();

    // Create fields for MSA segment
    let fields = vec![
        // MSA-1: Acknowledgment Code
        Field {
            reps: vec![Rep {
                comps: vec![Comp {
                    subs: vec![Atom::Text(code.as_str().to_string())],
                }],
            }],
        },
        // MSA-2: Message Control ID
        Field {
            reps: vec![Rep {
                comps: vec![Comp {
                    subs: vec![Atom::Text(control_id)],
                }],
            }],
        },
    ];

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
    let field = segment.fields.get(field_index.checked_sub(1)?)?;
    let rep = field.reps.first()?;
    let comp = rep.comps.first()?;

    match comp.subs.first()? {
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
///
/// # Errors
///
/// Returns [`Error::InvalidSegmentId`] when the original message is empty or
/// does not start with an `MSH` segment.
pub fn ack_with_error(
    original: &Message,
    code: AckCode,
    error_message: Option<&str>,
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
    let fields = vec![
        // ERR-1: Error Code and Location (using segment ID and field)
        Field {
            reps: vec![Rep {
                comps: vec![Comp {
                    subs: vec![Atom::Text(String::new())],
                }],
            }],
        },
        // ERR-2: Error Location (HL7 table 0535)
        Field {
            reps: vec![Rep {
                comps: vec![Comp {
                    subs: vec![Atom::Text(String::new())],
                }],
            }],
        },
        // ERR-3: HL7 Error Code (HL7 table 0396)
        Field {
            reps: vec![Rep {
                comps: vec![Comp {
                    subs: vec![Atom::Text(error_message.to_string())],
                }],
            }],
        },
    ];

    Segment {
        id: *b"ERR",
        fields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn ensure(condition: bool, message: &'static str) -> TestResult {
        if condition {
            Ok(())
        } else {
            Err(std::io::Error::other(message).into())
        }
    }

    fn parse_sample() -> Result<Message, crate::Error> {
        crate::parse(
            b"MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P|2.5\r",
        )
    }

    fn ack_round_trip(msg: &Message) -> Result<Message, crate::Error> {
        crate::parse(&crate::write(msg))
    }

    #[test]
    fn ack_code_as_str_covers_all_variants() -> TestResult {
        ensure(AckCode::AA.as_str() == "AA", "AA")?;
        ensure(AckCode::AE.as_str() == "AE", "AE")?;
        ensure(AckCode::AR.as_str() == "AR", "AR")?;
        ensure(AckCode::CA.as_str() == "CA", "CA")?;
        ensure(AckCode::CE.as_str() == "CE", "CE")?;
        ensure(AckCode::CR.as_str() == "CR", "CR")
    }

    #[test]
    fn ack_code_display_matches_as_str() -> TestResult {
        for code in [
            AckCode::AA,
            AckCode::AE,
            AckCode::AR,
            AckCode::CA,
            AckCode::CE,
            AckCode::CR,
        ] {
            let displayed = format!("{code}");
            ensure(displayed == code.as_str(), "Display mismatches as_str")?;
        }
        Ok(())
    }

    #[test]
    fn ack_rejects_empty_message() -> TestResult {
        let empty = Message::new();
        let result = ack(&empty, AckCode::AA);
        ensure(
            matches!(result, Err(Error::InvalidSegmentId)),
            "empty message should yield InvalidSegmentId",
        )
    }

    #[test]
    fn ack_rejects_non_msh_first_segment() -> TestResult {
        let invalid = Message::with_segments(vec![Segment {
            id: *b"PID",
            fields: Vec::new(),
        }]);
        let result = ack(&invalid, AckCode::AA);
        ensure(
            matches!(result, Err(Error::InvalidSegmentId)),
            "non-MSH first segment should yield InvalidSegmentId",
        )
    }

    #[test]
    fn ack_preserves_default_encoding_characters() -> TestResult {
        let original = parse_sample()?;
        let ack_msg = ack(&original, AckCode::AA)?;
        let reparsed = ack_round_trip(&ack_msg)?;

        ensure(reparsed.delims.comp == '^', "comp delim")?;
        ensure(reparsed.delims.rep == '~', "rep delim")?;
        ensure(reparsed.delims.esc == '\\', "esc delim")?;
        ensure(reparsed.delims.sub == '&', "sub delim")
    }

    #[test]
    fn ack_propagates_non_default_encoding_characters() -> TestResult {
        let original = crate::parse(
            b"MSH|*&!?|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT*A01|CTRL999|P|2.5\r",
        )?;
        let ack_msg = ack(&original, AckCode::AA)?;
        let reparsed = ack_round_trip(&ack_msg)?;

        ensure(reparsed.delims.comp == '*', "comp delim propagated")?;
        ensure(reparsed.delims.rep == '&', "rep delim propagated")?;
        ensure(reparsed.delims.esc == '!', "esc delim propagated")?;
        ensure(reparsed.delims.sub == '?', "sub delim propagated")
    }

    #[test]
    fn ack_msh9_first_component_is_ack_with_original_message_type() -> TestResult {
        let original = parse_sample()?;
        let ack_msg = ack(&original, AckCode::AA)?;
        let reparsed = ack_round_trip(&ack_msg)?;

        ensure(
            crate::get(&reparsed, "MSH.9.1") == Some("ACK"),
            "MSH.9.1 should be ACK",
        )?;
        ensure(
            crate::get(&reparsed, "MSH.9.2") == Some("ADT"),
            "MSH.9.2 should carry original message type code",
        )
    }

    #[test]
    fn ack_defaults_version_when_source_missing() -> TestResult {
        let original = crate::parse(
            b"MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123|P\r",
        )?;
        let ack_msg = ack(&original, AckCode::AA)?;
        let reparsed = ack_round_trip(&ack_msg)?;

        ensure(
            crate::get(&reparsed, "MSH.12") == Some("2.5.1"),
            "MSH.12 should default to 2.5.1",
        )
    }

    #[test]
    fn ack_defaults_processing_id_when_source_missing() -> TestResult {
        let original = crate::parse(
            b"MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|202605030101||ADT^A01|CTRL123\r",
        )?;
        let ack_msg = ack(&original, AckCode::AA)?;
        let reparsed = ack_round_trip(&ack_msg)?;

        ensure(
            crate::get(&reparsed, "MSH.11") == Some("P"),
            "MSH.11 should default to P",
        )
    }

    #[test]
    fn ack_with_error_without_message_has_no_err_segment() -> TestResult {
        let original = parse_sample()?;
        let ack_msg = ack_with_error(&original, AckCode::AA, None)?;

        ensure(
            ack_msg.segments.len() == 2,
            "no error message should yield only MSH+MSA",
        )?;
        ensure(
            ack_msg.segments.first().map(|s| &s.id) == Some(b"MSH"),
            "first segment MSH",
        )?;
        ensure(
            ack_msg.segments.get(1).map(|s| &s.id) == Some(b"MSA"),
            "second segment MSA",
        )
    }

    #[test]
    fn ack_with_error_appends_err_segment_with_text() -> TestResult {
        let original = parse_sample()?;
        let ack_msg = ack_with_error(&original, AckCode::AE, Some("boom"))?;
        let reparsed = ack_round_trip(&ack_msg)?;

        ensure(reparsed.segments.len() == 3, "MSH + MSA + ERR")?;
        ensure(
            crate::get(&reparsed, "ERR.3") == Some("boom"),
            "ERR.3 should carry error text",
        )
    }

    #[test]
    fn ack_with_error_preserves_ae_and_ar_codes() -> TestResult {
        let original = parse_sample()?;

        for code in [AckCode::AE, AckCode::AR] {
            let ack_msg = ack_with_error(&original, code, Some("err"))?;
            let reparsed = ack_round_trip(&ack_msg)?;
            ensure(
                crate::get(&reparsed, "MSA.1") == Some(code.as_str()),
                "MSA.1 should match code",
            )?;
        }
        Ok(())
    }

    #[test]
    fn ack_carries_original_control_id_in_msa_2() -> TestResult {
        let original = parse_sample()?;
        let ack_msg = ack(&original, AckCode::AA)?;
        let reparsed = ack_round_trip(&ack_msg)?;

        ensure(
            crate::get(&reparsed, "MSA.2") == Some("CTRL123"),
            "MSA.2 should mirror original MSH-10",
        )
    }
}
