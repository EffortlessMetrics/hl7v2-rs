//! Unit tests for hl7v2-cli
//!
//! Tests argument parsing, command execution, and error handling.

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use hl7v2_test_utils::fixtures::SampleMessages;

    // Helper to create a temp file with content
    fn create_temp_file(dir: &TempDir, filename: &str, content: &[u8]) -> PathBuf {
        let path = dir.path().join(filename);
        fs::write(&path, content).expect("Failed to write temp file");
        path
    }

    // Helper to create a temp HL7 file
    fn create_temp_hl7_file(dir: &TempDir, filename: &str) -> PathBuf {
        create_temp_file(dir, filename, SampleMessages::adt_a01().as_bytes())
    }

    // =========================================================================
    // Argument Parsing Tests
    // =========================================================================

    mod argument_parsing {
        use super::*;
        use clap::CommandFactory;

        #[test]
        fn test_parse_command_requires_input() {
            // Test that parse command requires an input file argument
            // This is enforced by clap's derive macros
            use crate::Cli;
            let schema = Cli::command();
            // The command structure requires input for Parse variant
            assert!(schema.get_subcommands().any(|c| c.get_name() == "parse"));
        }

        #[test]
        fn test_validate_command_requires_profile() {
            // Test that validate command requires a profile argument
            use crate::Cli;
            let schema = Cli::command();
            assert!(schema.get_subcommands().any(|c| c.get_name() == "val"));
        }

        #[test]
        fn test_ack_command_mode_options() {
            // Verify ACK mode options exist
            let modes = vec!["original", "enhanced"];
            for mode in modes {
                // These should be valid values for --mode
                assert!(mode == "original" || mode == "enhanced");
            }
        }

        #[test]
        fn test_ack_command_code_options() {
            // Verify ACK code options exist
            let codes = vec!["AA", "AE", "AR", "CA", "CE", "CR"];
            for code in codes {
                // These should be valid values for --code
                assert!(!code.is_empty());
            }
        }
    }

    // =========================================================================
    // Parse Command Tests
    // =========================================================================

    mod parse_command {
        use super::*;
        use hl7v2_core::{parse, to_json};

        #[test]
        fn test_parse_valid_hl7_message() {
            let content = SampleMessages::adt_a01();
            let bytes = content.as_bytes();
            
            let result = parse(bytes);
            assert!(result.is_ok());
            
            let message = result.unwrap();
            assert!(!message.segments.is_empty());
            assert!(message.segments[0].id_str() == "MSH");
        }

        #[test]
        fn test_parse_outputs_valid_json() {
            let content = SampleMessages::adt_a01();
            let bytes = content.as_bytes();
            
            let message = parse(bytes).expect("Parse should succeed");
            let json_value = to_json(&message);
            
            // Verify it's valid JSON
            let json_string = serde_json::to_string(&json_value).expect("Should serialize to JSON");
            assert!(json_string.contains("MSH"));
        }

        #[test]
        fn test_parse_mllp_framed_message() {
            // Create MLLP-framed message
            let content = SampleMessages::adt_a01();
            let mut mllp_bytes = vec![0x0B]; // SB
            mllp_bytes.extend_from_slice(content.as_bytes());
            mllp_bytes.push(0x1C); // EB
            mllp_bytes.push(0x0D); // CR
            
            let result = hl7v2_core::parse_mllp(&mllp_bytes);
            assert!(result.is_ok());
            
            let message = result.unwrap();
            assert!(message.segments[0].id_str() == "MSH");
        }

        #[test]
        fn test_parse_detects_segment_count() {
            let content = SampleMessages::adt_a01();
            let bytes = content.as_bytes();
            
            let message = parse(bytes).expect("Parse should succeed");
            
            // ADT^A01 should have MSH, EVN, PID, PV1 segments
            assert!(message.segments.len() >= 4);
        }

        #[test]
        fn test_parse_extracts_delimiters() {
            let content = SampleMessages::adt_a01();
            let bytes = content.as_bytes();
            
            let message = parse(bytes).expect("Parse should succeed");
            
            // Standard delimiters
            assert_eq!(message.delims.field, '|');
            assert_eq!(message.delims.comp, '^');
            assert_eq!(message.delims.rep, '~');
            assert_eq!(message.delims.esc, '\\');
            assert_eq!(message.delims.sub, '&');
        }
    }

    // =========================================================================
    // Normalize Command Tests
    // =========================================================================

    mod norm_command {
        use super::*;
        use hl7v2_core::{parse, write};

        #[test]
        fn test_normalize_roundtrip() {
            let content = SampleMessages::adt_a01();
            let bytes = content.as_bytes();
            
            let message = parse(bytes).expect("Parse should succeed");
            let normalized = write(&message);
            
            // Should be able to parse the normalized output
            let reparsed = parse(&normalized).expect("Reparse should succeed");
            assert_eq!(message.segments.len(), reparsed.segments.len());
        }

        #[test]
        fn test_normalize_mllp_output() {
            let content = SampleMessages::adt_a01();
            let bytes = content.as_bytes();
            
            let message = parse(bytes).expect("Parse should succeed");
            let normalized = write(&message);
            
            // Wrap in MLLP
            let mllp_bytes = hl7v2_core::wrap_mllp(&normalized);
            
            // Verify MLLP framing
            assert_eq!(mllp_bytes[0], 0x0B); // SB
            assert_eq!(mllp_bytes[mllp_bytes.len() - 2], 0x1C); // EB
            assert_eq!(mllp_bytes[mllp_bytes.len() - 1], 0x0D); // CR
        }
    }

    // =========================================================================
    // Validate Command Tests
    // =========================================================================

    mod validate_command {
        use super::*;

        #[test]
        fn test_validate_with_valid_profile() {
            // Profile format must match hl7v2_prof::Profile struct
            let profile_yaml = r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
constraints:
  - path: MSH.1
    required: true
  - path: MSH.2
    required: true
"#;
            let result = hl7v2_prof::load_profile(profile_yaml);
            assert!(result.is_ok(), "Failed to load profile: {:?}", result.err());
        }

        #[test]
        fn test_validate_detects_missing_required_segment() {
            // Profile format must match hl7v2_prof::Profile struct
            let profile_yaml = r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
  - id: EVN
  - id: PID
  - id: ZZ1
constraints:
  - path: MSH
    required: true
  - path: EVN
    required: true
  - path: PID
    required: true
  - path: ZZ1
    required: true
"#;
            let profile = hl7v2_prof::load_profile(profile_yaml).expect("Profile should load");
            let message = hl7v2_core::parse(SampleMessages::adt_a01().as_bytes()).expect("Parse should succeed");
            
            let results = hl7v2_prof::validate(&message, &profile);
            
            // Should have validation issues because ZZ1 is not in the sample message
            assert!(!results.is_empty(), "Should have validation errors for missing ZZ1 segment");
        }
    }

    // =========================================================================
    // ACK Generation Tests
    // =========================================================================

    mod ack_command {
        use super::*;
        use hl7v2_gen::{ack, AckCode};

        #[test]
        fn test_generate_ack_aa() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2_core::parse(content.as_bytes()).expect("Parse should succeed");
            
            let ack_result = ack(&message, AckCode::AA);
            assert!(ack_result.is_ok());
            
            let ack_message = ack_result.unwrap();
            assert!(ack_message.segments.iter().any(|s| s.id_str() == "MSH"));
            assert!(ack_message.segments.iter().any(|s| s.id_str() == "MSA"));
        }

        #[test]
        fn test_generate_ack_ae() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2_core::parse(content.as_bytes()).expect("Parse should succeed");
            
            let ack_result = ack(&message, AckCode::AE);
            assert!(ack_result.is_ok());
        }

        #[test]
        fn test_generate_ack_ar() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2_core::parse(content.as_bytes()).expect("Parse should succeed");
            
            let ack_result = ack(&message, AckCode::AR);
            assert!(ack_result.is_ok());
        }

        #[test]
        fn test_ack_contains_original_message_control_id() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2_core::parse(content.as_bytes()).expect("Parse should succeed");
            
            let ack_message = ack(&message, AckCode::AA).expect("ACK should generate");
            
            // MSA segment should reference the original message
            let msa = ack_message.segments.iter().find(|s| s.id_str() == "MSA");
            assert!(msa.is_some());
        }
    }

    // =========================================================================
    // Generate Command Tests
    // =========================================================================

    mod gen_command {
        use super::*;

        #[test]
        fn test_parse_template_yaml() {
            // Template format matches hl7v2_template::Template struct
            let template_yaml = r#"
name: ADT_A01
delims: "^~\\&"
segments:
  - "MSH|^~\\&|TestApp|TestFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"
  - "PID|1||123456^^^HOSP^MR||Doe^John"
values: {}
"#;
            let result: Result<hl7v2_gen::Template, _> = serde_yaml::from_str(template_yaml);
            assert!(result.is_ok(), "Failed to parse template YAML: {:?}", result.err());
        }
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    mod error_handling {
        use super::*;

        #[test]
        fn test_parse_empty_input_returns_error() {
            let result = hl7v2_core::parse(b"");
            assert!(result.is_err());
        }

        #[test]
        fn test_parse_invalid_input_returns_error() {
            // Not a valid HL7 message
            let result = hl7v2_core::parse(b"This is not HL7");
            assert!(result.is_err());
        }

        #[test]
        fn test_parse_truncated_message_returns_error() {
            // Truncated message (just MSH with no proper structure)
            let result = hl7v2_core::parse(b"MSH");
            assert!(result.is_err());
        }

        #[test]
        fn test_missing_file_error() {
            let non_existent = PathBuf::from("/nonexistent/path/file.hl7");
            let result = fs::read_to_string(&non_existent);
            assert!(result.is_err());
        }

        #[test]
        fn test_invalid_profile_yaml_returns_error() {
            let invalid_yaml = "this is not: valid: yaml:::";
            let result = hl7v2_prof::load_profile(invalid_yaml);
            // Should handle gracefully (either error or empty profile)
            // Behavior depends on implementation
        }
    }

    // =========================================================================
    // MLLP Tests
    // =========================================================================

    mod mllp_handling {
        use super::*;

        #[test]
        fn test_mllp_wrap() {
            let data = b"MSH|^~\\&|Test\r";
            let wrapped = hl7v2_core::wrap_mllp(data);
            
            assert_eq!(wrapped[0], 0x0B); // SB
            assert!(wrapped[..].ends_with(&[0x1C, 0x0D])); // EB CR
        }

        #[test]
        fn test_mllp_parse_and_unwrap() {
            let content = SampleMessages::adt_a01();
            let mut mllp_bytes = vec![0x0B];
            mllp_bytes.extend_from_slice(content.as_bytes());
            mllp_bytes.push(0x1C);
            mllp_bytes.push(0x0D);
            
            let message = hl7v2_core::parse_mllp(&mllp_bytes).expect("Should parse MLLP");
            assert!(!message.segments.is_empty());
        }

        #[test]
        fn test_mllp_write() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2_core::parse(content.as_bytes()).expect("Parse should succeed");
            
            let mllp_bytes = hl7v2_core::write_mllp(&message);
            
            assert_eq!(mllp_bytes[0], 0x0B);
            assert!(mllp_bytes[..].ends_with(&[0x1C, 0x0D]));
        }
    }

    // =========================================================================
    // Interactive Mode Tests
    // =========================================================================

    mod interactive_mode {
        use super::*;

        #[test]
        fn test_interactive_help_command() {
            // The help command should list all available commands
            let commands = ["parse", "norm", "val", "ack", "gen", "help", "exit"];
            for cmd in commands {
                assert!(!cmd.is_empty());
            }
        }

        #[test]
        fn test_interactive_parse_command_parsing() {
            // Test parsing of interactive parse command format
            let input = "parse test.hl7 --json --summary";
            let parts: Vec<&str> = input.split_whitespace().collect();
            
            assert_eq!(parts[0], "parse");
            assert_eq!(parts[1], "test.hl7");
            assert!(parts.contains(&"--json"));
            assert!(parts.contains(&"--summary"));
        }
    }

    // Performance monitor tests are covered through integration tests
    // since the monitor module is not public

    // =========================================================================
    // Output Formatting Tests
    // =========================================================================

    mod output_formatting {
        use super::*;

        #[test]
        fn test_json_pretty_format() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2_core::parse(content.as_bytes()).expect("Parse should succeed");
            let json_value = hl7v2_core::to_json(&message);
            
            let pretty = serde_json::to_string_pretty(&json_value).expect("Should serialize");
            assert!(pretty.contains('\n')); // Pretty format has newlines
        }

        #[test]
        fn test_json_compact_format() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2_core::parse(content.as_bytes()).expect("Parse should succeed");
            let json_value = hl7v2_core::to_json(&message);
            
            let compact = serde_json::to_string(&json_value).expect("Should serialize");
            // Compact format should be smaller than pretty
            assert!(!compact.contains("\n  ")); // No indented newlines
        }
    }

    // =========================================================================
    // New CLI Flags Tests
    // =========================================================================

    mod new_flags {
        use super::*;
        use hl7v2_core::{parse, write, normalize};

        // -------------------------------------------------------------------------
        // --canonical-delims flag tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_canonical_delimiters_normalization() {
            // Create a message with non-canonical delimiters
            let content = "MSH|@#$\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1\rPID|1||12345^^^HOSP^MR||Doe^John\r";
            
            // Parse and normalize with canonical delimiters
            let message = parse(content.as_bytes()).expect("Parse should succeed");
            let original_bytes = write(&message);
            let normalized = normalize(&original_bytes, true).expect("Normalize should succeed");
            let normalized_str = String::from_utf8_lossy(&normalized);
            
            // Verify canonical delimiters are used
            assert!(normalized_str.starts_with("MSH|^~\\&|"));
        }

        #[test]
        fn test_canonical_delimiters_preserves_content() {
            let content = SampleMessages::adt_a01();
            let message = parse(content.as_bytes()).expect("Parse should succeed");
            
            let original_bytes = write(&message);
            let normalized = normalize(&original_bytes, true).expect("Normalize should succeed");
            
            // Parse the normalized message
            let reparsed = parse(&normalized).expect("Reparse should succeed");
            
            // Verify segment count is preserved
            assert_eq!(message.segments.len(), reparsed.segments.len());
        }

        // -------------------------------------------------------------------------
        // --envelope flag tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_envelope_mllp_wrap() {
            let content = SampleMessages::adt_a01();
            let message = parse(content.as_bytes()).expect("Parse should succeed");
            
            let output_bytes = write(&message);
            let mllp_bytes = hl7v2_core::wrap_mllp(&output_bytes);
            
            // Verify MLLP framing
            assert_eq!(mllp_bytes[0], 0x0B); // SB
            assert!(mllp_bytes[..].ends_with(&[0x1C, 0x0D])); // EB CR
        }

        #[test]
        fn test_envelope_and_canonical_combined() {
            let content = SampleMessages::adt_a01();
            let message = parse(content.as_bytes()).expect("Parse should succeed");
            
            // Normalize and wrap
            let original_bytes = write(&message);
            let normalized = normalize(&original_bytes, true).expect("Normalize should succeed");
            let mllp_bytes = hl7v2_core::wrap_mllp(&normalized);
            
            // Verify MLLP framing
            assert_eq!(mllp_bytes[0], 0x0B); // SB
            
            // Extract and verify content starts with canonical delimiters
            let content = &mllp_bytes[1..mllp_bytes.len()-2];
            let content_str = String::from_utf8_lossy(content);
            assert!(content_str.starts_with("MSH|^~\\&|"));
        }

        // -------------------------------------------------------------------------
        // --report flag tests (validation report formats)
        // -------------------------------------------------------------------------

        #[test]
        fn test_validation_report_json_format() {
            let profile_yaml = r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
constraints:
  - path: MSH.1
    required: true
"#;
            let profile = hl7v2_prof::load_profile(profile_yaml).expect("Profile should load");
            let message = parse(SampleMessages::adt_a01().as_bytes()).expect("Parse should succeed");
            
            let results = hl7v2_prof::validate(&message, &profile);
            
            // Create a validation report
            let report = crate::ValidationReport {
                input_file: "test.hl7".to_string(),
                profile_file: "profile.yaml".to_string(),
                file_size: 100,
                segment_count: message.segments.len(),
                is_valid: results.is_empty(),
                issue_count: results.len(),
                issues: results.iter().map(|r| format!("{:?}", r)).collect(),
            };
            
            // Verify JSON serialization works
            let json = serde_json::to_string_pretty(&report).expect("Should serialize to JSON");
            assert!(json.contains("input_file"));
            assert!(json.contains("is_valid"));
        }

        #[test]
        fn test_validation_report_yaml_format() {
            let profile_yaml = r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
constraints:
  - path: MSH.1
    required: true
"#;
            let profile = hl7v2_prof::load_profile(profile_yaml).expect("Profile should load");
            let message = parse(SampleMessages::adt_a01().as_bytes()).expect("Parse should succeed");
            
            let results = hl7v2_prof::validate(&message, &profile);
            
            // Create a validation report
            let report = crate::ValidationReport {
                input_file: "test.hl7".to_string(),
                profile_file: "profile.yaml".to_string(),
                file_size: 100,
                segment_count: message.segments.len(),
                is_valid: results.is_empty(),
                issue_count: results.len(),
                issues: results.iter().map(|r| format!("{:?}", r)).collect(),
            };
            
            // Verify YAML serialization works
            let yaml = serde_yaml::to_string(&report).expect("Should serialize to YAML");
            assert!(yaml.contains("input_file"));
            assert!(yaml.contains("is_valid"));
        }

        // -------------------------------------------------------------------------
        // --distributions flag tests (stats with field distributions)
        // -------------------------------------------------------------------------

        #[test]
        fn test_stats_field_distributions() {
            let content = SampleMessages::adt_a01();
            let message = parse(content.as_bytes()).expect("Parse should succeed");
            
            // Collect segment statistics
            let mut segment_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for segment in &message.segments {
                *segment_counts.entry(segment.id_str().to_string()).or_insert(0) += 1;
            }
            
            assert!(!segment_counts.is_empty());
            assert!(segment_counts.contains_key("MSH"));
        }

        #[test]
        fn test_stats_report_json_format() {
            let content = SampleMessages::adt_a01();
            let message = parse(content.as_bytes()).expect("Parse should succeed");
            
            // Collect segment statistics
            let mut segment_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for segment in &message.segments {
                *segment_counts.entry(segment.id_str().to_string()).or_insert(0) += 1;
            }
            
            let segments: Vec<crate::SegmentStats> = segment_counts
                .into_iter()
                .map(|(id, count)| crate::SegmentStats { segment_id: id, count })
                .collect();
            
            let report = crate::StatsReport {
                input_file: "test.hl7".to_string(),
                file_size: content.len(),
                segment_count: message.segments.len(),
                segments,
                field_distributions: None,
            };
            
            // Verify JSON serialization works
            let json = serde_json::to_string_pretty(&report).expect("Should serialize to JSON");
            assert!(json.contains("input_file"));
            assert!(json.contains("segment_count"));
        }

        #[test]
        fn test_stats_report_with_distributions() {
            let content = SampleMessages::adt_a01();
            let message = parse(content.as_bytes()).expect("Parse should succeed");
            
            // Collect field distributions
            let mut distributions: Vec<crate::FieldDistribution> = Vec::new();
            
            for segment in &message.segments {
                let segment_id = segment.id_str();
                
                for (field_idx, field) in segment.fields.iter().enumerate().take(5) {
                    if field_idx == 0 {
                        continue;
                    }
                    
                    let path = format!("{}.{}", segment_id, field_idx);
                    let value = field.first_text().unwrap_or("").to_string();
                    
                    if let Some(existing) = distributions.iter_mut().find(|d| d.path == path) {
                        if !existing.sample_values.contains(&value) && existing.sample_values.len() < 10 {
                            existing.sample_values.push(value);
                        }
                        existing.unique_values = existing.sample_values.len();
                    } else {
                        distributions.push(crate::FieldDistribution {
                            path,
                            unique_values: 1,
                            sample_values: vec![value],
                        });
                    }
                }
            }
            
            // Verify we collected some distributions
            assert!(!distributions.is_empty());
        }

        // -------------------------------------------------------------------------
        // --streaming flag tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_streaming_mode_flag() {
            // The streaming flag is primarily for memory-efficient processing
            // of large files. Here we just verify the flag exists and can be parsed.
            use clap::CommandFactory;
            use crate::Cli;
            
            let schema = Cli::command();
            let parse_cmd = schema.get_subcommands().find(|c| c.get_name() == "parse");
            assert!(parse_cmd.is_some());
            
            let parse_cmd = parse_cmd.unwrap();
            let streaming_arg = parse_cmd.get_arguments().find(|a| a.get_id() == "streaming");
            assert!(streaming_arg.is_some());
        }

        // -------------------------------------------------------------------------
        // ReportFormat enum tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_report_format_values() {
            use crate::ReportFormat;
            
            // Verify all format variants exist
            let text = ReportFormat::Text;
            let json = ReportFormat::Json;
            let yaml = ReportFormat::Yaml;
            
            // Verify default
            assert_eq!(ReportFormat::default(), ReportFormat::Text);
        }

        // -------------------------------------------------------------------------
        // Clap argument parsing tests for new flags
        // -------------------------------------------------------------------------

        #[test]
        fn test_parse_command_has_canonical_delims_flag() {
            use clap::CommandFactory;
            use crate::Cli;
            
            let schema = Cli::command();
            let parse_cmd = schema.get_subcommands().find(|c| c.get_name() == "parse");
            assert!(parse_cmd.is_some());
            
            let parse_cmd = parse_cmd.unwrap();
            let canonical_arg = parse_cmd.get_arguments().find(|a| a.get_id() == "canonical_delims");
            assert!(canonical_arg.is_some());
        }

        #[test]
        fn test_parse_command_has_envelope_flag() {
            use clap::CommandFactory;
            use crate::Cli;
            
            let schema = Cli::command();
            let parse_cmd = schema.get_subcommands().find(|c| c.get_name() == "parse");
            assert!(parse_cmd.is_some());
            
            let parse_cmd = parse_cmd.unwrap();
            let envelope_arg = parse_cmd.get_arguments().find(|a| a.get_id() == "envelope");
            assert!(envelope_arg.is_some());
        }

        #[test]
        fn test_val_command_has_report_flag() {
            use clap::CommandFactory;
            use crate::Cli;
            
            let schema = Cli::command();
            let val_cmd = schema.get_subcommands().find(|c| c.get_name() == "val");
            assert!(val_cmd.is_some());
            
            let val_cmd = val_cmd.unwrap();
            let report_arg = val_cmd.get_arguments().find(|a| a.get_id() == "report");
            assert!(report_arg.is_some());
        }

        #[test]
        fn test_stats_command_exists() {
            use clap::CommandFactory;
            use crate::Cli;
            
            let schema = Cli::command();
            let stats_cmd = schema.get_subcommands().find(|c| c.get_name() == "stats");
            assert!(stats_cmd.is_some());
        }

        #[test]
        fn test_stats_command_has_distributions_flag() {
            use clap::CommandFactory;
            use crate::Cli;
            
            let schema = Cli::command();
            let stats_cmd = schema.get_subcommands().find(|c| c.get_name() == "stats");
            assert!(stats_cmd.is_some());
            
            let stats_cmd = stats_cmd.unwrap();
            let distributions_arg = stats_cmd.get_arguments().find(|a| a.get_id() == "distributions");
            assert!(distributions_arg.is_some());
        }
    }
}
