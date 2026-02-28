//! Unit tests for hl7v2-model crate
//!
//! Tests cover:
//! - Message model structures
//! - Segment model structures
//! - Field model structures
//! - Error types

use super::*;

// ============================================================================
// Error Tests
// ============================================================================

#[cfg(test)]
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

        let err = Error::UnbalancedEscape;
        assert!(err.to_string().contains("Unbalanced escape"));

        let err = Error::InvalidEscapeToken;
        assert!(err.to_string().contains("Invalid escape token"));

        let err = Error::InvalidProcessingId;
        assert!(err.to_string().contains("Invalid processing ID"));

        let err = Error::InvalidFieldFormat {
            details: "test details".to_string(),
        };
        assert!(err.to_string().contains("Invalid field format"));
        assert!(err.to_string().contains("test details"));
    }

    #[test]
    fn test_error_clone() {
        let err = Error::InvalidSegmentId;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_error_partial_eq() {
        let err1 = Error::InvalidSegmentId;
        let err2 = Error::InvalidSegmentId;
        let err3 = Error::BadDelimLength;

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }
}

// ============================================================================
// Delims Tests
// ============================================================================

#[cfg(test)]
mod delims_tests {
    use super::*;

    #[test]
    fn test_default_delims() {
        let delims = Delims::default();

        assert_eq!(delims.field, '|');
        assert_eq!(delims.comp, '^');
        assert_eq!(delims.rep, '~');
        assert_eq!(delims.esc, '\\');
        assert_eq!(delims.sub, '&');
    }

    #[test]
    fn test_new_delims() {
        let delims = Delims::new();

        assert_eq!(delims.field, '|');
        assert_eq!(delims.comp, '^');
        assert_eq!(delims.rep, '~');
        assert_eq!(delims.esc, '\\');
        assert_eq!(delims.sub, '&');
    }

    #[test]
    fn test_custom_delims() {
        let delims = Delims {
            field: '*',
            comp: ':',
            rep: '+',
            esc: '\\',
            sub: '#',
        };

        assert_eq!(delims.field, '*');
        assert_eq!(delims.comp, ':');
        assert_eq!(delims.rep, '+');
        assert_eq!(delims.esc, '\\');
        assert_eq!(delims.sub, '#');
    }

    #[test]
    fn test_parse_from_msh() {
        // MSH|^~\&|...
        let msh = "MSH|^~\\&|SendingApp|ReceivingApp|...";
        let delims = Delims::parse_from_msh(msh).unwrap();

        assert_eq!(delims.field, '|');
        assert_eq!(delims.comp, '^');
        assert_eq!(delims.rep, '~');
        assert_eq!(delims.esc, '\\');
        assert_eq!(delims.sub, '&');
    }

    #[test]
    fn test_parse_from_msh_custom() {
        // MSH*:+\&|...
        let msh = "MSH*:+\\&|SendingApp|ReceivingApp|...";
        let delims = Delims::parse_from_msh(msh).unwrap();

        assert_eq!(delims.field, '*');
        assert_eq!(delims.comp, ':');
        assert_eq!(delims.rep, '+');
        assert_eq!(delims.esc, '\\');
        assert_eq!(delims.sub, '&');
    }

    #[test]
    fn test_parse_from_msh_too_short() {
        // Too short
        let msh = "MSH|";
        let result = Delims::parse_from_msh(msh);
        assert!(result.is_err());
    }
}

// ============================================================================
// Message Tests
// ============================================================================

#[cfg(test)]
mod message_tests {
    use super::*;

    #[test]
    fn test_new_message() {
        let message = Message::new();

        assert_eq!(message.delims, Delims::default());
        assert!(message.segments.is_empty());
    }

    #[test]
    fn test_default_message() {
        let message = Message::default();

        assert_eq!(message.delims, Delims::default());
        assert!(message.segments.is_empty());
    }

    #[test]
    fn test_message_with_segments() {
        let message = Message::with_segments(vec![Segment {
            id: *b"MSH",
            fields: vec![Field::from_text("test")],
        }]);

        assert_eq!(message.segments.len(), 1);
    }
}

// ============================================================================
// Segment Tests
// ============================================================================

#[cfg(test)]
mod segment_tests {
    use super::*;

    #[test]
    fn test_new_segment() {
        let segment = Segment::new(b"MSH");

        assert_eq!(segment.id, *b"MSH");
        assert!(segment.fields.is_empty());
    }

    #[test]
    fn test_segment_id_str() {
        let segment = Segment::new(b"MSH");
        assert_eq!(segment.id_str(), "MSH");

        let segment = Segment::new(b"PID");
        assert_eq!(segment.id_str(), "PID");
    }

    #[test]
    fn test_add_field() {
        let mut segment = Segment::new(b"PID");
        segment.add_field(Field::from_text("1"));
        segment.add_field(Field::from_text("12345"));

        assert_eq!(segment.fields.len(), 2);
    }
}

// ============================================================================
// Field Tests
// ============================================================================

#[cfg(test)]
mod field_tests {
    use super::*;

    #[test]
    fn test_new_field() {
        let field = Field::new();

        assert!(field.reps.is_empty());
    }

    #[test]
    fn test_default_field() {
        let field = Field::default();

        assert!(field.reps.is_empty());
    }

    #[test]
    fn test_from_text() {
        let field = Field::from_text("test value");

        assert_eq!(field.reps.len(), 1);
    }

    #[test]
    fn test_first_text_empty() {
        let field = Field::new();
        assert!(field.first_text().is_none());
    }

    #[test]
    fn test_first_text_with_value() {
        let field = Field::from_text("test value");
        assert_eq!(field.first_text(), Some("test value"));
    }

    #[test]
    fn test_add_rep() {
        let mut field = Field::new();
        field.add_rep(Rep::from_text("test"));

        assert_eq!(field.reps.len(), 1);
    }
}

// ============================================================================
// Rep Tests
// ============================================================================

#[cfg(test)]
mod rep_tests {
    use super::*;

    #[test]
    fn test_new_rep() {
        let rep = Rep::new();

        assert!(rep.comps.is_empty());
    }

    #[test]
    fn test_default_rep() {
        let rep = Rep::default();

        assert!(rep.comps.is_empty());
    }

    #[test]
    fn test_from_text() {
        let rep = Rep::from_text("test value");

        assert_eq!(rep.comps.len(), 1);
    }

    #[test]
    fn test_add_comp() {
        let mut rep = Rep::new();
        rep.add_comp(Comp::from_text("component"));

        assert_eq!(rep.comps.len(), 1);
    }
}

// ============================================================================
// Comp Tests
// ============================================================================

#[cfg(test)]
mod comp_tests {
    use super::*;

    #[test]
    fn test_new_comp() {
        let comp = Comp::new();

        assert!(comp.subs.is_empty());
    }

    #[test]
    fn test_default_comp() {
        let comp = Comp::default();

        assert!(comp.subs.is_empty());
    }

    #[test]
    fn test_from_text() {
        let comp = Comp::from_text("test value");

        assert_eq!(comp.subs.len(), 1);
    }

    #[test]
    fn test_add_sub() {
        let mut comp = Comp::new();
        comp.add_sub(Atom::text("subcomponent"));

        assert_eq!(comp.subs.len(), 1);
    }
}

// ============================================================================
// Atom Tests
// ============================================================================

#[cfg(test)]
mod atom_tests {
    use super::*;

    #[test]
    fn test_text_atom() {
        let atom = Atom::text("test");

        assert!(matches!(atom, Atom::Text(_)));
        assert_eq!(atom.as_text(), Some("test"));
    }

    #[test]
    fn test_null_atom() {
        let atom = Atom::null();

        assert!(matches!(atom, Atom::Null));
        assert!(atom.is_null());
    }

    #[test]
    fn test_is_null() {
        let null_atom = Atom::null();
        let text_atom = Atom::text("test");

        assert!(null_atom.is_null());
        assert!(!text_atom.is_null());
    }

    #[test]
    fn test_as_text() {
        let text_atom = Atom::text("test");
        let null_atom = Atom::null();

        assert_eq!(text_atom.as_text(), Some("test"));
        assert!(null_atom.as_text().is_none());
    }
}

// ============================================================================
// Batch Tests
// ============================================================================

#[cfg(test)]
mod batch_tests {
    use super::*;

    #[test]
    fn test_default_batch() {
        let batch = Batch::default();

        assert!(batch.header.is_none());
        assert!(batch.messages.is_empty());
        assert!(batch.trailer.is_none());
    }
}

// ============================================================================
// FileBatch Tests
// ============================================================================

#[cfg(test)]
mod file_batch_tests {
    use super::*;

    #[test]
    fn test_default_file_batch() {
        let file_batch = FileBatch::default();

        assert!(file_batch.header.is_none());
        assert!(file_batch.batches.is_empty());
        assert!(file_batch.trailer.is_none());
    }
}

// ============================================================================
// Presence Tests
// ============================================================================

#[cfg(test)]
mod presence_tests {
    use super::*;

    #[test]
    fn test_missing() {
        let presence = Presence::Missing;
        assert!(matches!(presence, Presence::Missing));
    }

    #[test]
    fn test_empty() {
        let presence = Presence::Empty;
        assert!(matches!(presence, Presence::Empty));
    }

    #[test]
    fn test_null() {
        let presence = Presence::Null;
        assert!(matches!(presence, Presence::Null));
    }

    #[test]
    fn test_value() {
        let presence = Presence::Value("test".to_string());
        assert!(matches!(presence, Presence::Value(_)));
    }
}
