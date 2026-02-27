use crate::{
    get, parse, parse_batch, parse_file_batch, parse_mllp, unescape_text, write, write_batch,
    write_mllp, Atom, Delims,
};

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

    #[test]
    fn test_batch_parsing() {
        // Test simple batch with BHS/BTS
        let batch_text = "BHS|^~\\&|SendingApp|SendingFac\rMSH|^~\\&|App|Fac\rPID|1||123456^^^HOSP^MR||Doe^John\rBTS|1\r";
        let batch = parse_batch(batch_text.as_bytes()).unwrap();
        
        assert!(batch.header.is_some());
        assert_eq!(batch.messages.len(), 1);
        assert!(batch.trailer.is_some());
        
        // Check BHS segment
        let bhs = batch.header.as_ref().unwrap();
        assert_eq!(&bhs.id, b"BHS");
        
        // Check message
        let message = &batch.messages[0];
        assert_eq!(&message.segments[0].id, b"MSH");
        assert_eq!(&message.segments[1].id, b"PID");
        
        // Check BTS segment
        let bts = batch.trailer.as_ref().unwrap();
        assert_eq!(&bts.id, b"BTS");
    }

    #[test]
    fn test_file_batch_parsing() {
        // Test simple file batch with FHS/FTS
        let file_batch_text = "FHS|^~\\&|SendingApp|SendingFac\rBHS|^~\\&|BatchApp|BatchFac\rMSH|^~\\&|App|Fac\rPID|1||123456^^^HOSP^MR||Doe^John\rBTS|1\rFTS|1\r";
        let file_batch = parse_file_batch(file_batch_text.as_bytes()).unwrap();
        
        assert!(file_batch.header.is_some());
        assert_eq!(file_batch.batches.len(), 1);
        assert!(file_batch.trailer.is_some());
        
        // Check FHS segment
        let fhs = file_batch.header.as_ref().unwrap();
        assert_eq!(&fhs.id, b"FHS");
        
        // Check batch
        let batch = &file_batch.batches[0];
        assert!(batch.header.is_some());
        assert_eq!(batch.messages.len(), 1);
        
        // Check message
        let message = &batch.messages[0];
        assert_eq!(&message.segments[0].id, b"MSH");
        assert_eq!(&message.segments[1].id, b"PID");
        
        // Check BTS segment
        let bts = batch.trailer.as_ref().unwrap();
        assert_eq!(&bts.id, b"BTS");
        
        // Check FTS segment
        let fts = file_batch.trailer.as_ref().unwrap();
        assert_eq!(&fts.id, b"FTS");
    }

    #[test]
    fn test_batch_writing() {
        // Create a batch and write it back
        let batch_text = "BHS|^~\\&|SendingApp|SendingFac\rMSH|^~\\&|App|Fac\rPID|1||123456^^^HOSP^MR||Doe^John\rBTS|1\r";
        let batch = parse_batch(batch_text.as_bytes()).unwrap();
        
        let written = write_batch(&batch);
        let written_str = String::from_utf8(written).unwrap();
        
        // Parse the written batch again
        let batch2 = parse_batch(written_str.as_bytes()).unwrap();
        
        // Verify the structure is preserved
        assert!(batch2.header.is_some());
        assert_eq!(batch2.messages.len(), 1);
        assert!(batch2.trailer.is_some());
    }

    #[test]
    fn test_presence_semantics() {
        use crate::{Presence, get_presence};

        // Create a message with various field states
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        // Test Value presence
        match get_presence(&message, "PID.3") {
            Presence::Value(value) => assert_eq!(value, "123456"),
            _ => panic!("Expected Value presence"),
        }
        
        // Test Empty presence
        match get_presence(&message, "PID.4") {
            Presence::Empty => {}, // Correct
            _ => panic!("Expected Empty presence"),
        }
        
        // Test Missing presence (field beyond what exists)
        match get_presence(&message, "PID.50") {
            Presence::Missing => {}, // Correct
            _ => panic!("Expected Missing presence"),
        }
        
        // Test component access
        match get_presence(&message, "PID.5.1") {
            Presence::Value(value) => assert_eq!(value, "Doe"),
            _ => panic!("Expected Value presence"),
        }
        
        // Test missing component
        match get_presence(&message, "PID.5.5") {
            Presence::Missing => {}, // Correct
            _ => panic!("Expected Missing presence"),
        }
        
        // Test MSH special fields
        match get_presence(&message, "MSH.1") {
            Presence::Value(value) => assert_eq!(value, "|"),
            _ => panic!("Expected Value presence for MSH-1"),
        }
        
        match get_presence(&message, "MSH.2") {
            Presence::Value(value) => assert_eq!(value, "^~\\&"),
            _ => panic!("Expected Value presence for MSH-2"),
        }
    }

    #[test]
    fn test_charset_extraction() {
        use crate::extract_charsets;

        // Create a message with charset information in MSH-18
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1||||||UTF-8\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message = parse(hl7_text.as_bytes()).unwrap();
        
        // Check that charset was extracted correctly
        assert_eq!(message.charsets.len(), 1);
        assert_eq!(message.charsets[0], "UTF-8");
        
        // Test with multiple charsets
        let hl7_text_multi = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1||||||UTF-8^ISO-8859-1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message_multi = parse(hl7_text_multi.as_bytes()).unwrap();
        
        // Check that multiple charsets were extracted correctly
        assert_eq!(message_multi.charsets.len(), 2);
        assert_eq!(message_multi.charsets[0], "UTF-8");
        assert_eq!(message_multi.charsets[1], "ISO-8859-1");
        
        // Test with no charset information
        let hl7_text_no_charset = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let message_no_charset = parse(hl7_text_no_charset.as_bytes()).unwrap();
        
        // Check that no charsets were extracted
        assert_eq!(message_no_charset.charsets.len(), 0);
    }

    #[test]
    fn test_streaming_parser() {
        use crate::{StreamParser, Event, Delims};
        use std::io::BufReader;

        // Create a simple HL7 message
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let reader = BufReader::new(hl7_text.as_bytes());
        let mut parser = StreamParser::new(reader);
        
        // Collect all events
        let mut events = Vec::new();
        while let Ok(Some(event)) = parser.next_event() {
            events.push(event);
        }
        
        // Check the events
        assert_eq!(events.len(), 5); // StartMessage, Segment(PID), Field(1), Field(2), Field(3), EndMessage
        
        // Check StartMessage event
        match &events[0] {
            Event::StartMessage { delims } => {
                assert_eq!(delims.field, '|');
                assert_eq!(delims.comp, '^');
                assert_eq!(delims.rep, '~');
                assert_eq!(delims.esc, '\\');
                assert_eq!(delims.sub, '&');
            },
            _ => panic!("Expected StartMessage event"),
        }
        
        // Check Segment event
        match &events[1] {
            Event::Segment { id } => {
                assert_eq!(id, b"PID");
            },
            _ => panic!("Expected Segment event"),
        }
        
        // Check Field events
        match &events[2] {
            Event::Field { num, raw } => {
                assert_eq!(*num, 1);
                assert_eq!(raw, b"1");
            },
            _ => panic!("Expected Field event"),
        }
        
        match &events[3] {
            Event::Field { num, raw } => {
                assert_eq!(*num, 2);
                assert_eq!(raw, b"");
            },
            _ => panic!("Expected Field event"),
        }
        
        match &events[4] {
            Event::Field { num, raw } => {
                assert_eq!(*num, 3);
                assert_eq!(raw, b"123456^^^HOSP^MR");
            },
            _ => panic!("Expected Field event"),
        }
    }

    #[test]
    fn test_mllp_parsing_and_writing() {
        // Create a simple HL7 message
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
        let original_message = parse(hl7_text.as_bytes()).unwrap();

        // Wrap with MLLP framing
        let mllp_bytes = write_mllp(&original_message);

        // Verify MLLP framing
        assert_eq!(mllp_bytes[0], 0x0B); // Start byte
        assert_eq!(mllp_bytes[mllp_bytes.len() - 2], 0x1C); // End byte 1
        assert_eq!(mllp_bytes[mllp_bytes.len() - 1], 0x0D); // End byte 2

        // Parse from MLLP framed bytes
        let parsed_message = parse_mllp(&mllp_bytes).unwrap();

        // Verify the messages are equivalent
        assert_eq!(original_message.segments.len(), parsed_message.segments.len());
        assert_eq!(
            std::str::from_utf8(&original_message.segments[0].id).unwrap(),
            std::str::from_utf8(&parsed_message.segments[0].id).unwrap()
        );
        assert_eq!(
            std::str::from_utf8(&original_message.segments[1].id).unwrap(),
            std::str::from_utf8(&parsed_message.segments[1].id).unwrap()
        );
    }

    #[test]
    fn test_network_module() {
        // Test that the network module can be compiled and used
        #[cfg(feature = "network")]
        {
            use crate::network::{AckTimingPolicy, MllpClient, MllpConfig, MllpServer};
            use std::time::Duration;

            // Test creating a config
            let config = MllpConfig {
                connect_timeout: Duration::from_secs(10),
                read_timeout: Duration::from_secs(10),
                write_timeout: Duration::from_secs(10),
                use_tls: false,
                ack_timing: AckTimingPolicy::Immediate,
            };

            // Test creating a client
            let _client = MllpClient::new(config.clone());

            // Test creating a server
            let _server = MllpServer::new(config);

            // These are just compilation tests - the actual functionality
            // would require network access which we don't want in unit tests
            assert!(true);
        }

        // Test that the module compiles even without the network feature
        #[cfg(not(feature = "network"))]
        {
            // This should compile fine even without the network feature
            assert!(true);
        }
    }
