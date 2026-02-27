use hl7v2_core::{Atom, Comp, Error, Field, Message, Rep, Segment};

/// Generate a single ACK message
pub fn ack(original: &Message, code: AckCode) -> Result<Message, Error> {
    // Create ACK message with same delimiters as original
    let delims = original.delims.clone();

    // Create MSH segment for ACK
    let msh_segment = create_ack_msh_segment(original)?;

    // Create MSA segment
    let msa_segment = create_msa_segment(original, &code)?;

    Ok(Message {
        delims,
        segments: vec![msh_segment, msa_segment],
        charsets: vec![],
    })
}

/// Create MSH segment for ACK message
fn create_ack_msh_segment(original: &Message) -> Result<Segment, Error> {
    // Get the original MSH segment
    let original_msh = original.segments.first().ok_or(Error::InvalidSegmentId)?;
    if &original_msh.id != b"MSH" {
        return Err(Error::InvalidSegmentId);
    }

    // Extract required fields from original MSH
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
                subs: vec![Atom::Text(format!(
                    "{}{}{}{}",
                    original.delims.comp, original.delims.rep, original.delims.esc, original.delims.sub
                ))],
            }],
        }],
    });

    // MSH-3: Sending Application
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(sending_app)],
            }],
        }],
    });

    // MSH-4: Sending Facility
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(sending_fac)],
            }],
        }],
    });

    // MSH-5: Receiving Application
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(receiving_app)],
            }],
        }],
    });

    // MSH-6: Receiving Facility
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(receiving_fac)],
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

    // MSH-8: Message Type
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(message_type)],
            }],
        }],
    });

    // MSH-9: Message Control ID
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(control_id)],
            }],
        }],
    });

    // MSH-10: Processing ID
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(processing_id)],
            }],
        }],
    });

    // MSH-11: Version ID
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

/// Create MSA segment for ACK message
fn create_msa_segment(original: &Message, code: &AckCode) -> Result<Segment, Error> {
    // Get the original MSH segment for control ID
    let original_msh = original.segments.first().ok_or(Error::InvalidSegmentId)?;
    if &original_msh.id != b"MSH" {
        return Err(Error::InvalidSegmentId);
    }

    // Get message control ID from original MSH-10
    let control_id = get_field_value(original_msh, 9).unwrap_or_else(|| "".to_string());

    // Convert ACK code to string
    let ack_code_str = match code {
        AckCode::AA => "AA",
        AckCode::AE => "AE",
        AckCode::AR => "AR",
        AckCode::CA => "CA",
        AckCode::CE => "CE",
        AckCode::CR => "CR",
    };

    // Create fields for MSA segment
    let mut fields = Vec::new();

    // MSA-1: Acknowledgment Code
    fields.push(Field {
        reps: vec![Rep {
            comps: vec![Comp {
                subs: vec![Atom::Text(ack_code_str.to_string())],
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

/// Get field value from a segment
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

/// ACK codes
#[derive(Debug, Clone)]
pub enum AckCode {
    AA, // Application Accept
    AE, // Application Error
    AR, // Application Reject
    CA, // Commit Accept
    CE, // Commit Error
    CR, // Commit Reject
}
