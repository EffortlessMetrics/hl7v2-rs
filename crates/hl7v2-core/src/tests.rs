#[cfg(test)]
mod tests {
    use crate::{parse, write, Atom, unescape_text, Delims, get};

    #[test]
    fn test_basic_segment_id() {
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        // Check segment IDs
        assert_eq!(&message.segments[0].id, b"MSH");
        assert_eq!(&message.segments[1].id, b"PID");
        
        // Test writing just the segment IDs
        let mut result = Vec::new();
        result.extend_from_slice(&message.segments[0].id);
        let output_str = String::from_utf8(result).unwrap();
        println!("First segment ID: '{}'", output_str);
        assert_eq!(output_str, "MSH");
    }

    #[test]
    fn test_msh_segment_details() {
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        // Check MSH segment details
        let msh_segment = &message.segments[0];
        assert_eq!(&msh_segment.id, b"MSH");
        println!("MSH segment fields: {}", msh_segment.fields.len());
        
        for (i, field) in msh_segment.fields.iter().enumerate() {
            println!("  Field {}: reps={}", i, field.reps.len());
            for (j, rep) in field.reps.iter().enumerate() {
                println!("    Rep {}: comps={}", j, rep.comps.len());
            }
        }
    }

    #[test]
    fn test_parse_simple_message() {
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        assert_eq!(message.delims.field, '|');
        assert_eq!(message.delims.comp, '^');
        assert_eq!(message.delims.rep, '~');
        assert_eq!(message.delims.esc, '\\');
        assert_eq!(message.delims.sub, '&');
        
        assert_eq!(message.segments.len(), 2);
        
        // Check MSH segment
        assert_eq!(&message.segments[0].id, b"MSH");
        assert_eq!(message.segments[0].fields.len(), 11); // MSH has 11 fields (not counting the field separator)
        
        // Check PID segment
        assert_eq!(&message.segments[1].id, b"PID");
        assert_eq!(message.segments[1].fields.len(), 5); // PID has 5 fields
    }

    #[test]
    fn test_null_values() {
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||\"\"||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        // Check that NULL values are properly parsed
        let pid_segment = &message.segments[1];
        let null_field = &pid_segment.fields[2]; // PID-3
        let null_rep = &null_field.reps[0];
        let null_comp = &null_rep.comps[0];
        
        match &null_comp.subs[0] {
            Atom::Null => {}, // Correct
            _ => panic!("Expected NULL atom"),
        }
    }

    #[test]
    fn test_escape_sequences() {
        // Test all HL7 v2 escape sequences
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||Test\\F\\Field\\S\\Component\\R\\Repeat\\E\\Escape\\T\\Subcomponent||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        // Check that escape sequences are properly parsed
        let pid_segment = &message.segments[1];
        let name_field = &pid_segment.fields[2]; // PID-3
        let name_rep = &name_field.reps[0];
        let name_comp = &name_rep.comps[0];
        
        match &name_comp.subs[0] {
            Atom::Text(text) => {
                assert_eq!(text, "Test|Field^Component~Repeat\\Escape&Subcomponent");
            },
            _ => panic!("Expected Text atom"),
        }
    }

    #[test]
    fn test_direct_unescape() {
        let delims = Delims::default();
        
        // Test simple escape
        let result = unescape_text("Test\\E\\Escape", &delims).unwrap();
        assert_eq!(result, "Test\\Escape");
        
        // Test the written text
        let written = "Test\\E\\Escape";
        let result2 = unescape_text(written, &delims).unwrap();
        assert_eq!(result2, "Test\\Escape");
    }

    #[test]
    fn test_simple_write_debug() {
        let original = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||Test\\E\\Escape||Doe^John\r";
        let message = parse(original.as_bytes()).unwrap();
        
        // Verify segment IDs are correct
        assert_eq!(&message.segments[0].id, b"MSH");
        assert_eq!(&message.segments[1].id, b"PID");
        
        // Check MSH segment details
        let msh_segment = &message.segments[0];
        println!("MSH segment fields: {}", msh_segment.fields.len());
        
        // Check that parsing worked
        let pid_segment = &message.segments[1];
        let name_field = &pid_segment.fields[2]; // PID-3
        let name_rep = &name_field.reps[0];
        let name_comp = &name_rep.comps[0];
        
        match &name_comp.subs[0] {
            Atom::Text(text) => {
                assert_eq!(text, "Test\\Escape");
            },
            _ => panic!("Expected Text atom"),
        }
        
        // Test writing
        let output = write(&message);
        let output_str = String::from_utf8(output).unwrap();
        println!("Written output: '{}'", output_str);
        
        // Check each segment
        for (i, segment) in message.segments.iter().enumerate() {
            let segment_id = String::from_utf8_lossy(&segment.id);
            println!("Segment {}: ID='{}', fields={}", i, segment_id, segment.fields.len());
        }
    }

    #[test]
    fn test_simple_escape_round_trip() {
        // Test a simpler case with just a backslash
        let original = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||Test\\E\\Escape||Doe^John\r";
        let message = parse(original.as_bytes()).unwrap();
        
        // Debug: Check what was parsed
        let pid_segment = &message.segments[1];
        let name_field = &pid_segment.fields[2]; // PID-3
        let name_rep = &name_field.reps[0];
        let name_comp = &name_rep.comps[0];
        
        match &name_comp.subs[0] {
            Atom::Text(text) => {
                println!("Parsed text: '{}'", text);
                assert_eq!(text, "Test\\Escape");
            },
            _ => panic!("Expected Text atom"),
        }
        
        let output = write(&message);
        let output_str = String::from_utf8(output).unwrap();
        println!("Output: '{}'", output_str);
        
        // Parse again to verify round-trip
        let message2 = parse(output_str.as_bytes()).unwrap();
        
        // Check that the text is preserved correctly
        let pid_segment2 = &message2.segments[1];
        let name_field2 = &pid_segment2.fields[2]; // PID-3
        let name_rep2 = &name_field2.reps[0];
        let name_comp2 = &name_rep2.comps[0];
        
        match &name_comp2.subs[0] {
            Atom::Text(text) => {
                println!("Round-trip text: '{}'", text);
                assert_eq!(text, "Test\\Escape");
            },
            _ => panic!("Expected Text atom"),
        }
    }

    #[test]
    fn test_round_trip_escape_sequences() {
        let original = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||Test\\F\\Field\\S\\Component\\R\\Repeat\\E\\Escape\\T\\Subcomponent||Doe^John\r";
        let message = parse(original.as_bytes()).unwrap();
        
        // Debug: Check what was parsed
        let pid_segment = &message.segments[1];
        let name_field = &pid_segment.fields[2]; // PID-3
        let name_rep = &name_field.reps[0];
        let name_comp = &name_rep.comps[0];
        
        match &name_comp.subs[0] {
            Atom::Text(text) => {
                println!("Parsed text: '{}'", text);
                assert_eq!(text, "Test|Field^Component~Repeat\\Escape&Subcomponent");
            },
            _ => panic!("Expected Text atom"),
        }
        
        let output = write(&message);
        let output_str = String::from_utf8(output).unwrap();
        println!("Output: '{}'", output_str);
        
        // Parse again to verify round-trip
        let message2 = parse(output_str.as_bytes()).unwrap();
        
        // Check that the text is preserved correctly
        let pid_segment2 = &message2.segments[1];
        let name_field2 = &pid_segment2.fields[2]; // PID-3
        let name_rep2 = &name_field2.reps[0];
        let name_comp2 = &name_rep2.comps[0];
        
        match &name_comp2.subs[0] {
            Atom::Text(text) => {
                println!("Round-trip text: '{}'", text);
                assert_eq!(text, "Test|Field^Component~Repeat\\Escape&Subcomponent");
            },
            _ => panic!("Expected Text atom"),
        }
    }

    #[test]
    fn test_get_function() {
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        // Debug: Print MSH segment details
        let msh_segment = &message.segments[0];
        println!("MSH segment fields: {}", msh_segment.fields.len());
        for (i, field) in msh_segment.fields.iter().enumerate() {
            println!("  Field {}: reps={}", i, field.reps.len());
            for (j, rep) in field.reps.iter().enumerate() {
                println!("    Rep {}: comps={}", j, rep.comps.len());
                for (k, comp) in rep.comps.iter().enumerate() {
                    println!("      Comp {}: subs={}", k, comp.subs.len());
                    for (l, atom) in comp.subs.iter().enumerate() {
                        match atom {
                            Atom::Text(text) => println!("        Atom {}: Text({})", l, text),
                            Atom::Null => println!("        Atom {}: Null", l),
                        }
                    }
                }
            }
        }
        
        // Test basic field retrieval
        assert_eq!(get(&message, "MSH.3"), Some("SendingApp"));
        assert_eq!(get(&message, "MSH.4"), Some("SendingFac"));
        assert_eq!(get(&message, "PID.1"), Some("1"));
        assert_eq!(get(&message, "PID.3"), Some("123456"));
        
        // Test component retrieval
        assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
        assert_eq!(get(&message, "PID.5.2"), Some("John"));
        
        // Test field that doesn't exist
        assert_eq!(get(&message, "PID.10"), None);
        
        // Test component that doesn't exist
        assert_eq!(get(&message, "PID.1.5"), None);
        
        // Test segment that doesn't exist
        assert_eq!(get(&message, "PV1.1"), None);
        
        // Test MSH segment special fields
        // MSH.1 is the field separator, which we don't support retrieving
        assert_eq!(get(&message, "MSH.2"), Some("^~\\&"));
        assert_eq!(get(&message, "MSH.9.1"), Some("ADT"));
        assert_eq!(get(&message, "MSH.9.2"), Some("A01"));
        assert_eq!(get(&message, "MSH.9.3"), Some("ADT_A01"));
    }

    #[test]
    fn test_get_with_repetitions() {
        // Create a message with field repetitions
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John~Smith^Jane\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        // Test first repetition (default)
        assert_eq!(get(&message, "PID.5.1"), Some("Doe"));
        assert_eq!(get(&message, "PID.5.2"), Some("John"));
        
        // Test second repetition
        assert_eq!(get(&message, "PID.5[2].1"), Some("Smith"));
        assert_eq!(get(&message, "PID.5[2].2"), Some("Jane"));
        
        // Test repetition that doesn't exist
        assert_eq!(get(&message, "PID.5[3].1"), None);
    }
}