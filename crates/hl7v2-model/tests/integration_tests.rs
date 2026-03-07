//! Integration tests for hl7v2-model crate
//!
//! Tests cover:
//! - Message creation and building
//! - Message serialization/deserialization
//! - Segment manipulation
//! - Field access and field traversal
//! - Message validation

use hl7v2_model::*;

// ============================================================================
// Message Creation and Building Tests
// ============================================================================

mod message_creation_tests {
    use super::*;

    #[test]
    fn test_create_empty_message() {
        let message = Message::new();
        assert!(message.segments.is_empty());
        assert_eq!(message.delims, Delims::default());
        assert!(message.charsets.is_empty());
    }

    #[test]
    fn test_create_message_with_default() {
        let message = Message::default();
        assert!(message.segments.is_empty());
        assert_eq!(message.delims.field, '|');
        assert_eq!(message.delims.comp, '^');
        assert_eq!(message.delims.rep, '~');
        assert_eq!(message.delims.esc, '\\');
        assert_eq!(message.delims.sub, '&');
    }

    #[test]
    fn test_create_message_with_segments() {
        let msh = Segment::new(b"MSH");
        let pid = Segment::new(b"PID");
        let message = Message::with_segments(vec![msh, pid]);

        assert_eq!(message.segments.len(), 2);
        assert_eq!(message.segments[0].id_str(), "MSH");
        assert_eq!(message.segments[1].id_str(), "PID");
    }

    #[test]
    fn test_build_message_step_by_step() {
        let mut message = Message::new();

        // Add MSH segment
        let mut msh = Segment::new(b"MSH");
        msh.add_field(Field::from_text("|")); // Field separator
        msh.add_field(Field::from_text("^~\\&")); // Encoding characters
        msh.add_field(Field::from_text("SENDING_APP")); // Sending application
        msh.add_field(Field::from_text("SENDING_FAC")); // Sending facility
        msh.add_field(Field::from_text("")); // DateTime (empty for test)
        msh.add_field(Field::from_text("")); // Security (empty for test)
        msh.add_field(Field::from_text("ADT^A01")); // Message type
        msh.add_field(Field::from_text("MSG_ID_123")); // Message Control ID
        msh.add_field(Field::from_text("P")); // Processing ID
        msh.add_field(Field::from_text("2.5.1")); // Version
        message.segments.push(msh);

        // Add PID segment
        let mut pid = Segment::new(b"PID");
        pid.add_field(Field::from_text("1")); // Set ID
        pid.add_field(Field::from_text("")); // Patient ID (external)
        pid.add_field(Field::from_text("PATIENT_ID^CHECK_DIGIT")); // Patient ID (internal)
        pid.add_field(Field::from_text("")); // Alternate Patient ID
        pid.add_field(Field::from_text("DOE^JOHN^MIDDLE")); // Patient Name
        message.segments.push(pid);

        assert_eq!(message.segments.len(), 2);
        assert_eq!(message.segments[0].id_str(), "MSH");
        assert_eq!(message.segments[1].id_str(), "PID");
    }

    #[test]
    fn test_message_with_custom_delimiters() {
        let mut message = Message::new();
        message.delims = Delims {
            field: '*',
            comp: ':',
            rep: '+',
            esc: '\\',
            sub: '#',
        };

        assert_eq!(message.delims.field, '*');
        assert_eq!(message.delims.comp, ':');
        assert_eq!(message.delims.rep, '+');
        assert_eq!(message.delims.sub, '#');
    }

    #[test]
    fn test_message_with_charsets() {
        let mut message = Message::new();
        message.charsets.push("ASCII".to_string());
        message.charsets.push("UNICODE".to_string());

        assert_eq!(message.charsets.len(), 2);
        assert_eq!(message.charsets[0], "ASCII");
        assert_eq!(message.charsets[1], "UNICODE");
    }
}

// ============================================================================
// Message Serialization/Deserialization Tests
// ============================================================================

mod message_serialization_tests {
    use super::*;

    #[test]
    fn test_serialize_empty_message() {
        let message = Message::new();
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"segments\":[]"));
        assert!(json.contains("\"delims\":"));
    }

    #[test]
    fn test_deserialize_empty_message() {
        let json = r#"{"delims":{"field":"|","comp":"^","rep":"~","esc":"\\","sub":"&"},"segments":[],"charsets":[]}"#;
        let message: Message = serde_json::from_str(json).unwrap();

        assert!(message.segments.is_empty());
        assert_eq!(message.delims.field, '|');
        assert_eq!(message.delims.comp, '^');
    }

    #[test]
    fn test_serialize_message_with_segments() {
        let mut msh = Segment::new(b"MSH");
        msh.add_field(Field::from_text("TEST_VALUE"));

        let message = Message::with_segments(vec![msh]);
        let json = serde_json::to_string(&message).unwrap();

        // Segment ID is serialized as byte array [77, 83, 72] for "MSH"
        assert!(json.contains("[77,83,72]"));
        assert!(json.contains("TEST_VALUE"));
    }

    #[test]
    fn test_deserialize_message_with_segments() {
        let json = r#"{
            "delims": {"field": "|", "comp": "^", "rep": "~", "esc": "\\", "sub": "&"},
            "segments": [
                {
                    "id": [77, 83, 72],
                    "fields": [
                        {"reps": [{"comps": [{"subs": [{"Text": "TEST"}]}]}]}
                    ]
                }
            ],
            "charsets": []
        }"#;

        let message: Message = serde_json::from_str(json).unwrap();
        assert_eq!(message.segments.len(), 1);
        assert_eq!(message.segments[0].id_str(), "MSH");
        assert_eq!(message.segments[0].fields[0].first_text(), Some("TEST"));
    }

    #[test]
    fn test_roundtrip_serialization() {
        let mut message = Message::new();

        // Add a segment
        let mut pid = Segment::new(b"PID");
        pid.add_field(Field::from_text("1"));
        pid.add_field(Field::from_text("PATIENT_ID"));
        message.segments.push(pid);

        // Serialize and deserialize
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(message, deserialized);
    }

    #[test]
    fn test_serialize_complex_field() {
        let mut field = Field::new();
        field.add_rep(Rep::from_text("COMP1"));
        field.add_rep(Rep::from_text("COMP2"));

        let segment = Segment {
            id: *b"TST",
            fields: vec![field],
        };

        let message = Message::with_segments(vec![segment]);
        let json = serde_json::to_string(&message).unwrap();

        assert!(json.contains("COMP1"));
        assert!(json.contains("COMP2"));
    }
}

// ============================================================================
// Segment Manipulation Tests
// ============================================================================

mod segment_manipulation_tests {
    use super::*;

    #[test]
    fn test_create_segment() {
        let segment = Segment::new(b"PID");
        assert_eq!(segment.id_str(), "PID");
        assert!(segment.fields.is_empty());
    }

    #[test]
    fn test_segment_id_as_str() {
        let msh = Segment::new(b"MSH");
        assert_eq!(msh.id_str(), "MSH");

        let pid = Segment::new(b"PID");
        assert_eq!(pid.id_str(), "PID");

        let nk1 = Segment::new(b"NK1");
        assert_eq!(nk1.id_str(), "NK1");
    }

    #[test]
    fn test_add_fields_to_segment() {
        let mut segment = Segment::new(b"PID");

        segment.add_field(Field::from_text("1"));
        segment.add_field(Field::from_text("PATIENT_ID"));
        segment.add_field(Field::from_text("DOE^JOHN"));

        assert_eq!(segment.fields.len(), 3);
        assert_eq!(segment.fields[0].first_text(), Some("1"));
        assert_eq!(segment.fields[1].first_text(), Some("PATIENT_ID"));
        assert_eq!(segment.fields[2].first_text(), Some("DOE^JOHN"));
    }

    #[test]
    fn test_segment_with_empty_fields() {
        let mut segment = Segment::new(b"PID");

        segment.add_field(Field::new()); // Empty field
        segment.add_field(Field::from_text("VALUE"));
        segment.add_field(Field::new()); // Another empty field

        assert_eq!(segment.fields.len(), 3);
        assert!(segment.fields[0].first_text().is_none());
        assert_eq!(segment.fields[1].first_text(), Some("VALUE"));
        assert!(segment.fields[2].first_text().is_none());
    }

    #[test]
    fn test_segment_clone() {
        let mut original = Segment::new(b"PID");
        original.add_field(Field::from_text("TEST"));

        let cloned = original.clone();

        assert_eq!(original.id, cloned.id);
        assert_eq!(original.fields.len(), cloned.fields.len());
        assert_eq!(original.fields[0].first_text(), cloned.fields[0].first_text());
    }

    #[test]
    fn test_segment_equality() {
        let mut seg1 = Segment::new(b"PID");
        seg1.add_field(Field::from_text("VALUE"));

        let mut seg2 = Segment::new(b"PID");
        seg2.add_field(Field::from_text("VALUE"));

        let mut seg3 = Segment::new(b"PID");
        seg3.add_field(Field::from_text("DIFFERENT"));

        assert_eq!(seg1, seg2);
        assert_ne!(seg1, seg3);
    }

    #[test]
    fn test_segment_direct_construction() {
        let segment = Segment {
            id: *b"EVN",
            fields: vec![
                Field::from_text("A01"),
                Field::from_text("20230101120000"),
            ],
        };

        assert_eq!(segment.id_str(), "EVN");
        assert_eq!(segment.fields.len(), 2);
    }
}

// ============================================================================
// Field Access and Traversal Tests
// ============================================================================

mod field_access_tests {
    use super::*;

    #[test]
    fn test_create_empty_field() {
        let field = Field::new();
        assert!(field.reps.is_empty());
    }

    #[test]
    fn test_create_field_from_text() {
        let field = Field::from_text("SOME_VALUE");
        assert_eq!(field.reps.len(), 1);
        assert_eq!(field.first_text(), Some("SOME_VALUE"));
    }

    #[test]
    fn test_field_first_text_empty() {
        let field = Field::new();
        assert!(field.first_text().is_none());
    }

    #[test]
    fn test_field_first_text_with_value() {
        let field = Field::from_text("TEST");
        assert_eq!(field.first_text(), Some("TEST"));
    }

    #[test]
    fn test_field_with_multiple_repetitions() {
        let mut field = Field::new();
        field.add_rep(Rep::from_text("REP1"));
        field.add_rep(Rep::from_text("REP2"));
        field.add_rep(Rep::from_text("REP3"));

        assert_eq!(field.reps.len(), 3);
        assert_eq!(field.first_text(), Some("REP1"));
    }

    #[test]
    fn test_field_traversal_deep() {
        // Create a field with nested structure
        let mut field = Field::new();

        // First repetition with multiple components
        let mut rep1 = Rep::new();
        let mut comp1 = Comp::new();
        comp1.add_sub(Atom::text("SUB1"));
        comp1.add_sub(Atom::text("SUB2"));
        rep1.add_comp(comp1);
        field.add_rep(rep1);

        // Verify traversal
        assert_eq!(field.reps[0].comps[0].subs.len(), 2);
        assert_eq!(field.reps[0].comps[0].subs[0].as_text(), Some("SUB1"));
        assert_eq!(field.reps[0].comps[0].subs[1].as_text(), Some("SUB2"));
    }

    #[test]
    fn test_rep_creation() {
        let rep = Rep::from_text("VALUE");
        assert_eq!(rep.comps.len(), 1);
    }

    #[test]
    fn test_rep_with_multiple_components() {
        let mut rep = Rep::new();
        rep.add_comp(Comp::from_text("COMP1"));
        rep.add_comp(Comp::from_text("COMP2"));
        rep.add_comp(Comp::from_text("COMP3"));

        assert_eq!(rep.comps.len(), 3);
    }

    #[test]
    fn test_comp_creation() {
        let comp = Comp::from_text("VALUE");
        assert_eq!(comp.subs.len(), 1);
        assert_eq!(comp.subs[0].as_text(), Some("VALUE"));
    }

    #[test]
    fn test_comp_with_multiple_subcomponents() {
        let mut comp = Comp::new();
        comp.add_sub(Atom::text("SUB1"));
        comp.add_sub(Atom::text("SUB2"));
        comp.add_sub(Atom::text("SUB3"));

        assert_eq!(comp.subs.len(), 3);
    }

    #[test]
    fn test_atom_text() {
        let atom = Atom::text("TEXT_VALUE");
        assert!(matches!(atom, Atom::Text(_)));
        assert_eq!(atom.as_text(), Some("TEXT_VALUE"));
        assert!(!atom.is_null());
    }

    #[test]
    fn test_atom_null() {
        let atom = Atom::null();
        assert!(matches!(atom, Atom::Null));
        assert!(atom.as_text().is_none());
        assert!(atom.is_null());
    }

    #[test]
    fn test_field_with_null_atom() {
        let mut comp = Comp::new();
        comp.add_sub(Atom::null());

        let mut rep = Rep::new();
        rep.add_comp(comp);

        let mut field = Field::new();
        field.add_rep(rep);

        // First text should be None because the atom is null
        assert!(field.first_text().is_none());
    }
}

// ============================================================================
// Message Validation Tests
// ============================================================================

mod message_validation_tests {
    use super::*;

    #[test]
    fn test_valid_message_structure() {
        let mut message = Message::new();

        // Add MSH segment with required fields
        let mut msh = Segment::new(b"MSH");
        msh.add_field(Field::from_text("|"));
        msh.add_field(Field::from_text("^~\\&"));
        msh.add_field(Field::from_text("SENDING_APP^SENDING_FAC"));
        msh.add_field(Field::from_text("RECV_APP^RECV_FAC"));
        msh.add_field(Field::from_text("20230101120000"));
        msh.add_field(Field::from_text(""));
        msh.add_field(Field::from_text("ADT^A01"));
        msh.add_field(Field::from_text("MSG123"));
        msh.add_field(Field::from_text("P"));
        msh.add_field(Field::from_text("2.5.1"));
        message.segments.push(msh);

        // Validate message has required MSH segment
        assert!(!message.segments.is_empty());
        assert_eq!(message.segments[0].id_str(), "MSH");
    }

    #[test]
    fn test_delimiter_validation() {
        // Default delimiters should be valid
        let delims = Delims::default();
        assert_ne!(delims.field, delims.comp);
        assert_ne!(delims.field, delims.rep);
        assert_ne!(delims.field, delims.esc);
        assert_ne!(delims.field, delims.sub);
        assert_ne!(delims.comp, delims.rep);
        assert_ne!(delims.comp, delims.esc);
        assert_ne!(delims.comp, delims.sub);
        assert_ne!(delims.rep, delims.esc);
        assert_ne!(delims.rep, delims.sub);
        assert_ne!(delims.esc, delims.sub);
    }

    #[test]
    fn test_parse_delimiters_from_valid_msh() {
        let msh = "MSH|^~\\&|SENDING|RECV|20230101||ADT^A01|MSG123|P|2.5.1";
        let delims = Delims::parse_from_msh(msh).unwrap();

        assert_eq!(delims.field, '|');
        assert_eq!(delims.comp, '^');
        assert_eq!(delims.rep, '~');
        assert_eq!(delims.esc, '\\');
        assert_eq!(delims.sub, '&');
    }

    #[test]
    fn test_parse_delimiters_too_short() {
        let msh = "MSH|";
        let result = Delims::parse_from_msh(msh);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_segment_order() {
        let mut message = Message::new();

        // Add segments in order
        message.segments.push(Segment::new(b"MSH"));
        message.segments.push(Segment::new(b"EVN"));
        message.segments.push(Segment::new(b"PID"));
        message.segments.push(Segment::new(b"NK1"));
        message.segments.push(Segment::new(b"PV1"));

        // Verify order
        assert_eq!(message.segments[0].id_str(), "MSH");
        assert_eq!(message.segments[1].id_str(), "EVN");
        assert_eq!(message.segments[2].id_str(), "PID");
        assert_eq!(message.segments[3].id_str(), "NK1");
        assert_eq!(message.segments[4].id_str(), "PV1");
    }

    #[test]
    fn test_empty_message_is_valid() {
        let message = Message::new();
        // An empty message is structurally valid (no segments)
        assert!(message.segments.is_empty());
    }
}

// ============================================================================
// Presence Semantics Tests
// ============================================================================

mod presence_tests {
    use super::*;

    #[test]
    fn test_presence_missing() {
        let presence = Presence::Missing;
        assert!(presence.is_missing());
        assert!(!presence.is_present());
        assert!(!presence.has_value());
        assert!(presence.value().is_none());
    }

    #[test]
    fn test_presence_empty() {
        let presence = Presence::Empty;
        assert!(!presence.is_missing());
        assert!(presence.is_present());
        assert!(!presence.has_value());
        assert!(presence.value().is_none());
    }

    #[test]
    fn test_presence_null() {
        let presence = Presence::Null;
        assert!(!presence.is_missing());
        assert!(presence.is_present());
        assert!(!presence.has_value());
        assert!(presence.value().is_none());
    }

    #[test]
    fn test_presence_value() {
        let presence = Presence::Value("TEST_VALUE".to_string());
        assert!(!presence.is_missing());
        assert!(presence.is_present());
        assert!(presence.has_value());
        assert_eq!(presence.value(), Some("TEST_VALUE"));
    }

    #[test]
    fn test_presence_clone() {
        let original = Presence::Value("ORIGINAL".to_string());
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_presence_equality() {
        let p1 = Presence::Value("SAME".to_string());
        let p2 = Presence::Value("SAME".to_string());
        let p3 = Presence::Value("DIFFERENT".to_string());

        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
        assert_ne!(Presence::Missing, Presence::Empty);
        assert_ne!(Presence::Empty, Presence::Null);
    }
}

// ============================================================================
// Batch Tests
// ============================================================================

mod batch_tests {
    use super::*;

    #[test]
    fn test_empty_batch() {
        let batch = Batch::default();
        assert!(batch.header.is_none());
        assert!(batch.messages.is_empty());
        assert!(batch.trailer.is_none());
    }

    #[test]
    fn test_batch_with_messages() {
        let mut batch = Batch::default();

        // Add messages
        batch.messages.push(Message::new());
        batch.messages.push(Message::new());

        assert_eq!(batch.messages.len(), 2);
    }

    #[test]
    fn test_batch_with_header_and_trailer() {
        let mut batch = Batch::default();

        let header = Segment::new(b"BHS");
        let trailer = Segment::new(b"BTS");

        batch.header = Some(header);
        batch.trailer = Some(trailer);

        assert!(batch.header.is_some());
        assert!(batch.trailer.is_some());
        assert_eq!(batch.header.as_ref().unwrap().id_str(), "BHS");
        assert_eq!(batch.trailer.as_ref().unwrap().id_str(), "BTS");
    }

    #[test]
    fn test_file_batch() {
        let file_batch = FileBatch::default();
        assert!(file_batch.header.is_none());
        assert!(file_batch.batches.is_empty());
        assert!(file_batch.trailer.is_none());
    }

    #[test]
    fn test_file_batch_with_batches() {
        let mut file_batch = FileBatch::default();

        file_batch.batches.push(Batch::default());
        file_batch.batches.push(Batch::default());

        assert_eq!(file_batch.batches.len(), 2);
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod error_tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::InvalidSegmentId;
        assert!(err.to_string().contains("Invalid segment ID"));

        let err = Error::BadDelimLength;
        assert!(err.to_string().contains("Bad delimiter length"));

        let err = Error::DuplicateDelims;
        assert!(err.to_string().contains("Duplicate delimiters"));
    }

    #[test]
    fn test_error_clone() {
        let err = Error::InvalidSegmentId;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_error_equality() {
        assert_eq!(Error::InvalidSegmentId, Error::InvalidSegmentId);
        assert_ne!(Error::InvalidSegmentId, Error::BadDelimLength);
    }

    #[test]
    fn test_error_with_details() {
        let err = Error::InvalidFieldFormat {
            details: "Field 5 is malformed".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Invalid field format"));
        assert!(msg.contains("Field 5 is malformed"));
    }

    #[test]
    fn test_parse_error_with_source() {
        let inner = Error::InvalidFieldFormat {
            details: "bad format".to_string(),
        };
        let err = Error::ParseError {
            segment_id: "PID".to_string(),
            field_index: 3,
            source: Box::new(inner),
        };

        let msg = err.to_string();
        assert!(msg.contains("PID"));
        assert!(msg.contains("3"));
    }
}

// ============================================================================
// Real-world Message Building Tests
// ============================================================================

mod real_world_tests {
    use super::*;

    #[test]
    fn test_build_adt_a01_message() {
        let mut message = Message::new();

        // MSH Segment
        let mut msh = Segment::new(b"MSH");
        msh.add_field(Field::from_text("|"));
        msh.add_field(Field::from_text("^~\\&"));
        msh.add_field(Field::from_text("HIS"));
        msh.add_field(Field::from_text("HOSPITAL"));
        msh.add_field(Field::from_text("LAB"));
        msh.add_field(Field::from_text("HOSPITAL"));
        msh.add_field(Field::from_text("20230101120000"));
        msh.add_field(Field::from_text(""));
        msh.add_field(Field::from_text("ADT^A01^ADT_A01"));
        msh.add_field(Field::from_text("MSG00001"));
        msh.add_field(Field::from_text("P"));
        msh.add_field(Field::from_text("2.5.1"));
        message.segments.push(msh);

        // EVN Segment
        let mut evn = Segment::new(b"EVN");
        evn.add_field(Field::from_text("A01"));
        evn.add_field(Field::from_text("20230101120000"));
        message.segments.push(evn);

        // PID Segment
        let mut pid = Segment::new(b"PID");
        pid.add_field(Field::from_text("1"));
        pid.add_field(Field::from_text(""));
        pid.add_field(Field::from_text("PATIENT123^^^HOSPITAL^MR"));
        pid.add_field(Field::from_text(""));
        pid.add_field(Field::from_text("DOE^JOHN^MIDDLE^JR^SR"));
        pid.add_field(Field::from_text(""));
        pid.add_field(Field::from_text("19800101"));
        pid.add_field(Field::from_text("M"));
        message.segments.push(pid);

        // PV1 Segment
        let mut pv1 = Segment::new(b"PV1");
        pv1.add_field(Field::from_text("1"));
        pv1.add_field(Field::from_text("I"));
        pv1.add_field(Field::from_text("WARD001^ROOM100^BED01"));
        message.segments.push(pv1);

        // Verify structure
        assert_eq!(message.segments.len(), 4);
        assert_eq!(message.segments[0].id_str(), "MSH");
        assert_eq!(message.segments[1].id_str(), "EVN");
        assert_eq!(message.segments[2].id_str(), "PID");
        assert_eq!(message.segments[3].id_str(), "PV1");

        // Verify MSH fields (0-indexed: MSG00001 is at index 9, P at 10, 2.5.1 at 11)
        assert_eq!(message.segments[0].fields[9].first_text(), Some("MSG00001"));
        assert_eq!(message.segments[0].fields[10].first_text(), Some("P"));
        assert_eq!(message.segments[0].fields[11].first_text(), Some("2.5.1"));
    }

    #[test]
    fn test_build_message_with_nested_components() {
        let mut message = Message::new();

        // MSH with simple fields
        let mut msh = Segment::new(b"MSH");
        msh.add_field(Field::from_text("|"));
        msh.add_field(Field::from_text("^~\\&"));
        msh.add_field(Field::from_text("APP"));
        message.segments.push(msh);

        // PID with complex name field (component)
        let mut pid = Segment::new(b"PID");
        pid.add_field(Field::from_text("1"));

        // Create a field with components for patient name
        let mut name_field = Field::new();
        let mut name_rep = Rep::new();

        // Family name component
        name_rep.add_comp(Comp::from_text("DOE"));
        // Given name component
        name_rep.add_comp(Comp::from_text("JOHN"));
        // Middle name component
        name_rep.add_comp(Comp::from_text("M"));
        // Suffix component
        name_rep.add_comp(Comp::from_text("JR"));

        name_field.add_rep(name_rep);
        pid.add_field(name_field);

        message.segments.push(pid);

        // Verify nested structure
        assert_eq!(message.segments[1].fields[1].reps[0].comps.len(), 4);
        assert_eq!(
            message.segments[1].fields[1].reps[0].comps[0].subs[0].as_text(),
            Some("DOE")
        );
        assert_eq!(
            message.segments[1].fields[1].reps[0].comps[1].subs[0].as_text(),
            Some("JOHN")
        );
    }

    #[test]
    fn test_message_with_repeating_fields() {
        let mut message = Message::new();

        // Simple MSH
        let mut msh = Segment::new(b"MSH");
        msh.add_field(Field::from_text("|"));
        msh.add_field(Field::from_text("^~\\&"));
        message.segments.push(msh);

        // NK1 with multiple repetitions
        let mut nk1 = Segment::new(b"NK1");
        nk1.add_field(Field::from_text("1"));

        // Create field with multiple repetitions for phone numbers
        let mut phones = Field::new();
        phones.add_rep(Rep::from_text("555-1234^H"));
        phones.add_rep(Rep::from_text("555-5678^W"));
        phones.add_rep(Rep::from_text("555-9999^C"));
        nk1.add_field(phones);

        message.segments.push(nk1);

        // Verify repetitions
        assert_eq!(message.segments[1].fields[1].reps.len(), 3);
    }
}
