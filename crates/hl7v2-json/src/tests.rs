//! Unit tests for hl7v2-json crate
//!
//! Tests cover:
//! - HL7 to JSON conversion
//! - JSON to HL7 conversion
//! - JSON schema validation

//! - Field structure tests

//! - Segment structure tests

//! - Custom delimiters tests

use super::*;
use serde_json::json;

use hl7v2_model::*;

// ============================================================================
// to_json Tests
// ============================================================================

#[cfg(test)]
mod to_json_tests {
    use super::*;
    
    #[test]
    fn test_empty_message() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        
        assert!(json.is_object());
        assert!(json.get("meta").is_some());
        assert!(json.get("segments").is_some());
        
        let segments = json.get("segments").unwrap().as_array().unwrap();
        assert!(segments.is_empty());
    }
    
    #[test]
    fn test_single_msh_segment() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![
                    Field::from_text("^~\\&"),
                    Field::from_text("SendingApp"),
                    Field::from_text("SendingFac"),
                ],
            }],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        
        assert!(json.is_object());
        
        let meta = json.get("meta").unwrap();
        assert!(meta.get("delims").is_some());
        
        let segments = json.get("segments").unwrap().as_array().unwrap();
        assert_eq!(segments.len(), 1);
        
        let msh = &segments[0];
        assert_eq!(msh.get("id").unwrap(), "MSH");
    }
    
    #[test]
    fn test_multiple_segments() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![
                Segment {
                    id: *b"MSH",
                    fields: vec![Field::from_text("^~\\&")],
                },
                Segment {
                    id: *b"PID",
                    fields: vec![Field::from_text("1")],
                },
                Segment {
                    id: *b"PV1",
                    fields: vec![Field::from_text("1")],
                },
            ],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        
        let segments = json.get("segments").unwrap().as_array().unwrap();
        assert_eq!(segments.len(), 3);
        
        assert_eq!(segments[0].get("id").unwrap(), "MSH");
        assert_eq!(segments[1].get("id").unwrap(), "PID");
        assert_eq!(segments[2].get("id").unwrap(), "PV1");
    }
    
    #[test]
    fn test_delimiters_in_json() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        
        let delims = json.get("meta").unwrap().get("delims").unwrap();
        assert_eq!(delims.get("field").unwrap(), "|");
        assert_eq!(delims.get("comp").unwrap(), "^");
        assert_eq!(delims.get("rep").unwrap(), "~");
        assert_eq!(delims.get("esc").unwrap(), "\\");
        assert_eq!(delims.get("sub").unwrap(), "&");
    }
    
    #[test]
    fn test_charsets_in_json() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![],
            charsets: vec!["ASCII".to_string(), "UNICODE".to_string()],
        };
        
        let json = to_json(&message);
        
        let charsets = json.get("meta").unwrap().get("charsets").unwrap().as_array().unwrap();
        assert_eq!(charsets.len(), 2);
        assert_eq!(charsets[0], "ASCII");
        assert_eq!(charsets[1], "UNICODE");
    }
    
    #[test]
    fn test_fields_in_json() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field::from_text("12345"),
                    Field::from_text("Doe^John"),
                ],
            }],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        
        let segments = json.get("segments").unwrap().as_array().unwrap();
        let pid = &segments[0];
        let fields = pid.get("fields").unwrap();
        
        // Fields are keyed by their position
        assert!(fields.get("1").is_some());
        assert!(fields.get("2").is_some());
        assert!(fields.get("3").is_some());
    }
    
    #[test]
    fn test_repetitions_in_json() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field {
                        reps: vec![
                            Rep {
                                comps: vec![Comp::from_text("Doe"), Comp::from_text("John")],
                            },
                            Rep {
                                comps: vec![Comp::from_text("Smith"), Comp::from_text("Jane")],
                            },
                        ],
                    },
                ],
            }],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        assert!(json.is_object());
    }
    
    #[test]
    fn test_components_in_json() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field {
                        reps: vec![Rep {
                            comps: vec![
                                Comp::from_text("Doe"),
                                Comp::from_text("John"),
                                Comp::from_text("A"),
                            ],
                        }],
                    },
                ],
            }],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        assert!(json.is_object());
    }
    
    #[test]
    fn test_null_atom_in_json() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![
                    Field::from_text("1"),
                    Field {
                        reps: vec![Rep {
                            comps: vec![Comp {
                                subs: vec![Atom::Null],
                            }],
                        }],
                    },
                ],
            }],
            charsets: vec![],
        };
        
        let json_str = to_json_string(&message);
        assert!(json_str.contains("__NULL__"));
    }
}

// ============================================================================
// to_json_string Tests
// ============================================================================

#[cfg(test)]
mod to_json_string_tests {
    use super::*;
    
    #[test]
    fn test_returns_valid_json() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&")],
            }],
            charsets: vec![],
        };
        
        let json_str = to_json_string(&message);
        
        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed.is_object());
    }
    
    #[test]
    fn test_starts_with_brace() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![],
            charsets: vec![],
        };
        
        let json_str = to_json_string(&message);
        assert!(json_str.starts_with('{'));
    }
    
    #[test]
    fn test_ends_with_brace() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![],
            charsets: vec![],
        };
        
        let json_str = to_json_string(&message);
        assert!(json_str.ends_with('}'));
    }
    
    #[test]
    fn test_compact_format() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&")],
            }],
            charsets: vec![],
        };
        
        let json_str = to_json_string(&message);
        // Compact format should not have newlines
        assert!(!json_str.contains('\n'));
    }
}

// ============================================================================
// to_json_string_pretty Tests
// ============================================================================

#[cfg(test)]
mod to_json_string_pretty_tests {
    use super::*;
    
    #[test]
    fn test_pretty_format() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&")],
            }],
            charsets: vec![],
        };
        
        let json_str = to_json_string_pretty(&message);
        // Pretty format should have newlines
        assert!(json_str.contains('\n'));
    }
    
    #[test]
    fn test_starts_with_brace() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![],
            charsets: vec![],
        };
        
        let json_str = to_json_string_pretty(&message);
        assert!(json_str.starts_with('{'));
    }
    
    #[test]
    fn test_valid_json() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text("^~\\&")],
            }],
            charsets: vec![],
        };
        
        let json_str = to_json_string_pretty(&message);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed.is_object());
    }
}

// ============================================================================
// Custom Delimiters Tests
// ============================================================================

#[cfg(test)]
mod custom_delims_tests {
    use super::*;
    
    #[test]
    fn test_custom_delimiters() {
        let message = Message {
            delims: Delims {
                field: '*',
                comp: ':',
                rep: '+',
                esc: '\\',
                sub: '#',
            },
            segments: vec![Segment {
                id: *b"MSH",
                fields: vec![Field::from_text(":~\\&")],
            }],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        
        let delims = json.get("meta").unwrap().get("delims").unwrap();
        assert_eq!(delims.get("field").unwrap(), "*");
        assert_eq!(delims.get("comp").unwrap(), ":");
        assert_eq!(delims.get("rep").unwrap(), "+");
        assert_eq!(delims.get("esc").unwrap(), "\\");
        assert_eq!(delims.get("sub").unwrap(), "#");
    }
}

// ============================================================================
// Segment ID Tests
// ============================================================================

#[cfg(test)]
mod segment_id_tests {
    use super::*;
    
    #[test]
    fn test_segment_id_encoding() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![
                Segment { id: *b"MSH", fields: vec![] },
                Segment { id: *b"PID", fields: vec![] },
                Segment { id: *b"PV1", fields: vec![] },
                Segment { id: *b"OBX", fields: vec![] },
            ],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        let segments = json.get("segments").unwrap().as_array().unwrap();
        
        assert_eq!(segments[0].get("id").unwrap(), "MSH");
        assert_eq!(segments[1].get("id").unwrap(), "PID");
        assert_eq!(segments[2].get("id").unwrap(), "PV1");
        assert_eq!(segments[3].get("id").unwrap(), "OBX");
    }
}

// ============================================================================
// Field Structure Tests
// ============================================================================

#[cfg(test)]
mod field_structure_tests {
    use super::*;
    
    #[test]
    fn test_simple_field() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field::from_text("SimpleValue")],
            }],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        assert!(json.is_object());
    }
    
    #[test]
    fn test_field_with_components() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field {
                    reps: vec![Rep {
                        comps: vec![
                            Comp::from_text("Family"),
                            Comp::from_text("Given"),
                        ],
                    }],
                }],
            }],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        assert!(json.is_object());
    }
    
    #[test]
    fn test_field_with_subcomponents() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field {
                    reps: vec![Rep {
                        comps: vec![Comp {
                            subs: vec![
                                Atom::text("Sub1"),
                                Atom::text("Sub2"),
                            ],
                        }],
                    }],
                }],
            }],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        assert!(json.is_object());
    }
    
    #[test]
    fn test_empty_field() {
        let message = Message {
            delims: Delims::default(),
            segments: vec![Segment {
                id: *b"PID",
                fields: vec![Field::new()],
            }],
            charsets: vec![],
        };
        
        let json = to_json(&message);
        assert!(json.is_object());
    }
}
