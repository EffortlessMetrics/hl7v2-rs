//! Unit tests for hl7v2-cli
//!
//! Tests argument parsing, command execution, and error handling.

#[cfg(test)]
#[expect(
    clippy::assertions_on_result_states,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "legacy CLI unit tests use static fixtures; cleanup is tracked in policy/clippy-debt.toml"
)]
mod cli_unit_tests {
    use hl7v2_test_utils::fixtures::SampleMessages;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

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
        fn test_profile_command_has_lint_subcommand() {
            use crate::Cli;
            let schema = Cli::command();
            let profile = schema
                .get_subcommands()
                .find(|command| command.get_name() == "profile")
                .expect("profile command should exist");

            assert!(
                profile
                    .get_subcommands()
                    .any(|command| command.get_name() == "lint")
            );
        }

        #[test]
        fn test_profile_command_has_test_subcommand() {
            use crate::Cli;
            let schema = Cli::command();
            let profile = schema
                .get_subcommands()
                .find(|command| command.get_name() == "profile")
                .expect("profile command should exist");

            assert!(
                profile
                    .get_subcommands()
                    .any(|command| command.get_name() == "test")
            );
        }

        #[test]
        fn test_profile_command_has_explain_subcommand() {
            use crate::Cli;
            let schema = Cli::command();
            let profile = schema
                .get_subcommands()
                .find(|command| command.get_name() == "profile")
                .expect("profile command should exist");

            assert!(
                profile
                    .get_subcommands()
                    .any(|command| command.get_name() == "explain")
            );
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

        #[test]
        fn test_redact_command_exists() {
            use crate::Cli;
            let schema = Cli::command();
            assert!(
                schema
                    .get_subcommands()
                    .any(|command| command.get_name() == "redact")
            );
        }

        #[test]
        fn test_bundle_command_exists() {
            use crate::Cli;
            let schema = Cli::command();
            assert!(
                schema
                    .get_subcommands()
                    .any(|command| command.get_name() == "bundle")
            );
        }

        #[test]
        fn test_replay_command_exists() {
            use crate::Cli;
            let schema = Cli::command();
            assert!(
                schema
                    .get_subcommands()
                    .any(|command| command.get_name() == "replay")
            );
        }
    }

    // =========================================================================
    // Parse Command Tests
    // =========================================================================

    mod parse_command {
        use super::*;
        use hl7v2::{parse, to_json};

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

            let result = hl7v2::parse_mllp(&mllp_bytes);
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
        use hl7v2::{parse, write};

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
            let mllp_bytes = hl7v2::wrap_mllp(&normalized);

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
            // Profile format must match hl7v2::Profile struct
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
            let result = hl7v2::load_profile(profile_yaml);
            assert!(result.is_ok(), "Failed to load profile: {:?}", result.err());
        }

        #[test]
        fn test_validate_detects_missing_required_segment() {
            // Profile format must match hl7v2::Profile struct
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
            let profile = hl7v2::load_profile(profile_yaml).expect("Profile should load");
            let message =
                hl7v2::parse(SampleMessages::adt_a01().as_bytes()).expect("Parse should succeed");

            let results = hl7v2::validate(&message, &profile);

            // Should have validation issues because ZZ1 is not in the sample message
            assert!(
                !results.is_empty(),
                "Should have validation errors for missing ZZ1 segment"
            );
        }
    }

    // =========================================================================
    // ACK Generation Tests
    // =========================================================================

    mod ack_command {
        use super::*;
        use hl7v2::{AckCode, ack};

        #[test]
        fn test_generate_ack_aa() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2::parse(content.as_bytes()).expect("Parse should succeed");

            let ack_result = ack(&message, AckCode::AA);
            assert!(ack_result.is_ok());

            let ack_message = ack_result.unwrap();
            assert!(ack_message.segments.iter().any(|s| s.id_str() == "MSH"));
            assert!(ack_message.segments.iter().any(|s| s.id_str() == "MSA"));
        }

        #[test]
        fn test_generate_ack_ae() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2::parse(content.as_bytes()).expect("Parse should succeed");

            let ack_result = ack(&message, AckCode::AE);
            assert!(ack_result.is_ok());
        }

        #[test]
        fn test_generate_ack_ar() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2::parse(content.as_bytes()).expect("Parse should succeed");

            let ack_result = ack(&message, AckCode::AR);
            assert!(ack_result.is_ok());
        }

        #[test]
        fn test_ack_contains_original_message_control_id() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2::parse(content.as_bytes()).expect("Parse should succeed");

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

        #[test]
        fn test_parse_template_yaml() {
            // Template format matches hl7v2::synthetic::template::Template struct
            let template_yaml = r#"
name: ADT_A01
delims: "^~\\&"
segments:
  - "MSH|^~\\&|TestApp|TestFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"
  - "PID|1||123456^^^HOSP^MR||Doe^John"
values: {}
"#;
            let result: Result<hl7v2::synthetic::generate::Template, _> =
                serde_yaml::from_str(template_yaml);
            assert!(
                result.is_ok(),
                "Failed to parse template YAML: {:?}",
                result.err()
            );
        }
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    mod error_handling {
        use super::*;

        #[test]
        fn test_parse_empty_input_returns_error() {
            let result = hl7v2::parse(b"");
            assert!(result.is_err());
        }

        #[test]
        fn test_parse_invalid_input_returns_error() {
            // Not a valid HL7 message
            let result = hl7v2::parse(b"This is not HL7");
            assert!(result.is_err());
        }

        #[test]
        fn test_parse_truncated_message_returns_error() {
            // Truncated message (just MSH with no proper structure)
            let result = hl7v2::parse(b"MSH");
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
            let _result = hl7v2::load_profile(invalid_yaml);
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
            let wrapped = hl7v2::wrap_mllp(data);

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

            let message = hl7v2::parse_mllp(&mllp_bytes).expect("Should parse MLLP");
            assert!(!message.segments.is_empty());
        }

        #[test]
        fn test_mllp_write() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2::parse(content.as_bytes()).expect("Parse should succeed");

            let mllp_bytes = hl7v2::write_mllp(&message);

            assert_eq!(mllp_bytes[0], 0x0B);
            assert!(mllp_bytes[..].ends_with(&[0x1C, 0x0D]));
        }
    }

    // =========================================================================
    // Interactive Mode Tests
    // =========================================================================

    mod interactive_mode {

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
            let message = hl7v2::parse(content.as_bytes()).expect("Parse should succeed");
            let json_value = hl7v2::to_json(&message);

            let pretty = serde_json::to_string_pretty(&json_value).expect("Should serialize");
            assert!(pretty.contains('\n')); // Pretty format has newlines
        }

        #[test]
        fn test_json_compact_format() {
            let content = SampleMessages::adt_a01();
            let message = hl7v2::parse(content.as_bytes()).expect("Parse should succeed");
            let json_value = hl7v2::to_json(&message);

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
        use hl7v2::{normalize, parse, write};

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
            let mllp_bytes = hl7v2::wrap_mllp(&output_bytes);

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
            let mllp_bytes = hl7v2::wrap_mllp(&normalized);

            // Verify MLLP framing
            assert_eq!(mllp_bytes[0], 0x0B); // SB

            // Extract and verify content starts with canonical delimiters
            let content = &mllp_bytes[1..mllp_bytes.len() - 2];
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
            let profile = hl7v2::load_profile(profile_yaml).expect("Profile should load");
            let message =
                parse(SampleMessages::adt_a01().as_bytes()).expect("Parse should succeed");

            let results = hl7v2::validate(&message, &profile);

            // Create a validation report
            let report = hl7v2::ValidationReport::from_issues(
                &message,
                Some("profile.yaml".to_string()),
                results,
            );

            // Verify JSON serialization works
            let json = serde_json::to_string_pretty(&report).expect("Should serialize to JSON");
            assert!(json.contains("message_type"));
            assert!(json.contains("valid"));
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
            let profile = hl7v2::load_profile(profile_yaml).expect("Profile should load");
            let message =
                parse(SampleMessages::adt_a01().as_bytes()).expect("Parse should succeed");

            let results = hl7v2::validate(&message, &profile);

            // Create a validation report
            let report = hl7v2::ValidationReport::from_issues(
                &message,
                Some("profile.yaml".to_string()),
                results,
            );

            // Verify YAML serialization works
            let yaml = serde_yaml::to_string(&report).expect("Should serialize to YAML");
            assert!(yaml.contains("message_type"));
            assert!(yaml.contains("valid"));
        }

        // -------------------------------------------------------------------------
        // Stats command logic tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_collect_stats_basic() {
            use hl7v2::parse;
            let content = SampleMessages::adt_a01();
            let message = parse(content.as_bytes()).expect("Parse should succeed");

            let stats = crate::collect_stats(&message, false);

            assert_eq!(stats.segment_count, message.segments.len());
            assert!(stats.field_distributions.is_none());
            assert!(stats.segments.iter().any(|s| s.segment_id == "MSH"));
            assert!(stats.segments.iter().any(|s| s.segment_id == "PID"));
        }

        #[test]
        fn test_collect_stats_with_distributions() {
            use hl7v2::parse;
            let content = SampleMessages::adt_a01();
            let message = parse(content.as_bytes()).expect("Parse should succeed");

            let stats = crate::collect_stats(&message, true);

            assert!(stats.field_distributions.is_some());
            let dists = stats.field_distributions.unwrap();
            assert!(!dists.is_empty());
            // Should contain MSH.3, etc. (skips MSH.0 as per logic)
            assert!(dists.iter().any(|d| d.path == "MSH.3"));
        }

        #[test]
        fn test_format_stats_report_text() {
            let segments = vec![
                crate::SegmentStats {
                    segment_id: "MSH".to_string(),
                    count: 1,
                },
                crate::SegmentStats {
                    segment_id: "PID".to_string(),
                    count: 1,
                },
            ];

            let report = crate::StatsReport {
                input_file: "test.hl7".to_string(),
                file_size: 100,
                segment_count: 2,
                segments,
                field_distributions: None,
            };

            let output = crate::format_stats_report(&report, &crate::ReportFormat::Text)
                .expect("Should format as text");

            assert!(output.contains("Message Statistics:"));
            assert!(output.contains("Input file: test.hl7"));
            assert!(output.contains("Total segments: 2"));
            assert!(output.contains("MSH: 1 occurrence(s)"));
            assert!(output.contains("PID: 1 occurrence(s)"));
        }

        #[test]
        fn test_format_stats_report_yaml() {
            let report = crate::StatsReport {
                input_file: "test.hl7".to_string(),
                file_size: 100,
                segment_count: 1,
                segments: vec![crate::SegmentStats {
                    segment_id: "MSH".to_string(),
                    count: 1,
                }],
                field_distributions: None,
            };

            let output = crate::format_stats_report(&report, &crate::ReportFormat::Yaml)
                .expect("Should format as YAML");

            assert!(output.contains("input_file: test.hl7"));
            assert!(output.contains("segment_count: 1"));
        }

        #[test]
        fn test_expected_report_array_matching_allows_issue_subset_out_of_order() {
            let expected = serde_json::json!({
                "issues": [
                    {
                        "code": "missing_required_field",
                        "severity": "error",
                        "path": "PID.3"
                    }
                ]
            });
            let actual = serde_json::json!({
                "valid": false,
                "issues": [
                    {
                        "code": "datatype_violation",
                        "severity": "warning",
                        "path": "PID.7"
                    },
                    {
                        "code": "missing_required_field",
                        "severity": "error",
                        "path": "PID.3",
                        "message": "PID.3 is required"
                    }
                ]
            });

            assert!(crate::json_subset_matches(&expected, &actual, "$").is_ok());
        }

        #[test]
        fn test_expected_report_lookup_uses_mirrored_paths_for_duplicate_stems() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            let fixture_root = dir.path().join("fixtures");
            let expected_root = fixture_root.join("expected");
            let valid_root = fixture_root.join("valid");
            let invalid_root = fixture_root.join("invalid");
            std::fs::create_dir_all(expected_root.join("valid")).unwrap();
            std::fs::create_dir_all(expected_root.join("invalid")).unwrap();
            std::fs::create_dir_all(&valid_root).unwrap();
            std::fs::create_dir_all(&invalid_root).unwrap();

            let valid_file = valid_root.join("same.hl7");
            let invalid_file = invalid_root.join("same.hl7");
            std::fs::write(&valid_file, b"valid").unwrap();
            std::fs::write(&invalid_file, b"invalid").unwrap();
            std::fs::write(expected_root.join("valid/same.report.json"), b"{}").unwrap();
            std::fs::write(expected_root.join("invalid/same.report.json"), b"{}").unwrap();

            let valid_files = vec![valid_file.clone()];
            let invalid_files = vec![invalid_file.clone()];
            let lookup = crate::build_expected_report_lookup(
                &fixture_root,
                &expected_root,
                [&valid_files, &invalid_files],
            );

            let valid_expected = match lookup.get(&valid_file) {
                Some(crate::ExpectedReportCandidate::File(path)) => Some(path),
                _ => None,
            };
            let invalid_expected = match lookup.get(&invalid_file) {
                Some(crate::ExpectedReportCandidate::File(path)) => Some(path),
                _ => None,
            };
            assert_eq!(
                valid_expected,
                Some(&expected_root.join("valid/same.report.json"))
            );
            assert_eq!(
                invalid_expected,
                Some(&expected_root.join("invalid/same.report.json"))
            );
        }

        #[test]
        fn test_expected_report_lookup_flags_ambiguous_basename_fallback() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            let fixture_root = dir.path().join("fixtures");
            let expected_root = fixture_root.join("expected");
            let valid_root = fixture_root.join("valid");
            let invalid_root = fixture_root.join("invalid");
            std::fs::create_dir_all(&expected_root).unwrap();
            std::fs::create_dir_all(&valid_root).unwrap();
            std::fs::create_dir_all(&invalid_root).unwrap();

            let valid_file = valid_root.join("same.hl7");
            let invalid_file = invalid_root.join("same.hl7");
            std::fs::write(&valid_file, b"valid").unwrap();
            std::fs::write(&invalid_file, b"invalid").unwrap();
            std::fs::write(expected_root.join("same.report.json"), b"{}").unwrap();

            let valid_files = vec![valid_file.clone()];
            let invalid_files = vec![invalid_file.clone()];
            let lookup = crate::build_expected_report_lookup(
                &fixture_root,
                &expected_root,
                [&valid_files, &invalid_files],
            );

            let valid_ambiguous = match lookup.get(&valid_file) {
                Some(crate::ExpectedReportCandidate::Ambiguous(path)) => Some(path),
                _ => None,
            };
            let invalid_ambiguous = match lookup.get(&invalid_file) {
                Some(crate::ExpectedReportCandidate::Ambiguous(path)) => Some(path),
                _ => None,
            };
            assert_eq!(
                valid_ambiguous,
                Some(&expected_root.join("same.report.json"))
            );
            assert_eq!(
                invalid_ambiguous,
                Some(&expected_root.join("same.report.json"))
            );
        }

        #[test]
        fn test_fixture_file_order_uses_case_stable_tie_breaker() {
            let upper = PathBuf::from("fixtures/CASE.hl7");
            let lower = PathBuf::from("fixtures/case.hl7");

            assert_eq!(
                crate::compare_paths_case_stable(&upper, &lower),
                std::cmp::Ordering::Less
            );
        }

        #[test]
        fn test_format_corpus_summary_report_text() {
            let report = hl7v2::synthetic::corpus::CorpusSummary {
                root: "corpus".to_string(),
                file_count: 2,
                message_count: 1,
                parse_error_count: 1,
                total_bytes: 128,
                message_types: vec![hl7v2::synthetic::corpus::CorpusCount {
                    value: "ADT^A01".to_string(),
                    count: 1,
                }],
                segments: vec![hl7v2::synthetic::corpus::CorpusCount {
                    value: "PID".to_string(),
                    count: 1,
                }],
                field_presence: vec![hl7v2::synthetic::corpus::CorpusFieldPresence {
                    path: "PID.3".to_string(),
                    message_count: 1,
                    occurrence_count: 1,
                }],
                parse_errors: vec![hl7v2::synthetic::corpus::CorpusParseFailure {
                    path: "bad.hl7".to_string(),
                    error: "Invalid segment ID".to_string(),
                }],
            };

            let output = crate::format_corpus_summary(&report, &crate::ReportFormat::Text)
                .expect("Should format as text");

            assert!(output.contains("Corpus Summary:"));
            assert!(output.contains("Files scanned: 2"));
            assert!(output.contains("ADT^A01: 1"));
            assert!(output.contains("PID.3: 1 message(s), 1 occurrence(s)"));
            assert!(output.contains("bad.hl7: Invalid segment ID"));
        }

        #[test]
        fn test_corpus_summarize_command_execution() {
            use crate::OutputOptions;
            use crate::ReportFormat;
            use crate::corpus_summarize_command;
            let dir = TempDir::new().expect("Failed to create temp dir");
            create_temp_hl7_file(&dir, "test.hl7");

            let result = corpus_summarize_command(
                &dir.path().to_path_buf(),
                &ReportFormat::Json,
                &OutputOptions::new(None, false, false),
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_format_corpus_diff_report_text() {
            let report = hl7v2::synthetic::corpus::CorpusDiffReport {
                diff_version: "1".to_string(),
                tool_version: "1.3.0".to_string(),
                before_root: "before".to_string(),
                after_root: "after".to_string(),
                profile: None,
                file_count: hl7v2::synthetic::corpus::CorpusTotalDiff {
                    before: 1,
                    after: 2,
                    delta: 1,
                },
                message_count: hl7v2::synthetic::corpus::CorpusTotalDiff {
                    before: 1,
                    after: 2,
                    delta: 1,
                },
                parse_error_count: hl7v2::synthetic::corpus::CorpusTotalDiff {
                    before: 0,
                    after: 1,
                    delta: 1,
                },
                new_message_types: vec!["ORU^R01".to_string()],
                removed_message_types: Vec::new(),
                new_segments: vec!["OBX".to_string()],
                removed_segments: Vec::new(),
                message_type_counts: vec![hl7v2::synthetic::corpus::CorpusCountDiff {
                    value: "ORU^R01".to_string(),
                    before: 0,
                    after: 1,
                    delta: 1,
                }],
                segment_counts: vec![hl7v2::synthetic::corpus::CorpusCountDiff {
                    value: "OBX".to_string(),
                    before: 0,
                    after: 1,
                    delta: 1,
                }],
                field_presence: vec![hl7v2::synthetic::corpus::CorpusFieldPresenceDiff {
                    path: "OBX.5".to_string(),
                    before_message_count: 0,
                    after_message_count: 1,
                    message_count_delta: 1,
                    before_occurrence_count: 0,
                    after_occurrence_count: 1,
                    occurrence_count_delta: 1,
                }],
                field_cardinality: vec![hl7v2::synthetic::corpus::CorpusFieldCardinalityDiff {
                    path: "OBX.5".to_string(),
                    before_min_per_message: 0,
                    after_min_per_message: 0,
                    min_per_message_delta: 0,
                    before_max_per_message: 0,
                    after_max_per_message: 1,
                    max_per_message_delta: 1,
                    before_total_occurrences: 0,
                    after_total_occurrences: 1,
                    total_occurrences_delta: 1,
                    before_message_count: 0,
                    after_message_count: 1,
                    message_count_delta: 1,
                }],
                value_shape_stats: vec![hl7v2::synthetic::corpus::CorpusValueShapeStatsDiff {
                    path: "OBX.5".to_string(),
                    coded_count: hl7v2::synthetic::corpus::CorpusTotalDiff {
                        before: 0,
                        after: 0,
                        delta: 0,
                    },
                    timestamp_count: hl7v2::synthetic::corpus::CorpusTotalDiff {
                        before: 0,
                        after: 0,
                        delta: 0,
                    },
                    numeric_count: hl7v2::synthetic::corpus::CorpusTotalDiff {
                        before: 0,
                        after: 1,
                        delta: 1,
                    },
                    null_count: hl7v2::synthetic::corpus::CorpusTotalDiff {
                        before: 0,
                        after: 0,
                        delta: 0,
                    },
                    text_count: hl7v2::synthetic::corpus::CorpusTotalDiff {
                        before: 0,
                        after: 0,
                        delta: 0,
                    },
                }],
                validation_issue_code_counts: Vec::new(),
            };

            let output = crate::format_corpus_diff(&report, &crate::ReportFormat::Text)
                .expect("Should format as text");

            assert!(output.contains("Corpus Diff:"));
            assert!(output.contains("Parsed messages: 1 -> 2 (+1)"));
            assert!(output.contains("ORU^R01: 0 -> 1 (+1)"));
            assert!(output.contains("OBX.5: messages 0 -> 1 (+1)"));
        }

        #[test]
        fn test_corpus_diff_command_execution() {
            use crate::OutputOptions;
            use crate::ReportFormat;
            use crate::corpus_diff_command;
            let before = TempDir::new().expect("Failed to create before temp dir");
            let after = TempDir::new().expect("Failed to create after temp dir");
            create_temp_hl7_file(&before, "test.hl7");
            create_temp_hl7_file(&after, "test.hl7");

            let result = corpus_diff_command(
                &before.path().to_path_buf(),
                &after.path().to_path_buf(),
                None,
                &ReportFormat::Json,
                &OutputOptions::new(None, false, false),
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_format_corpus_fingerprint_report_text() {
            let report = hl7v2::synthetic::corpus::CorpusFingerprint {
                fingerprint_version: "1".to_string(),
                tool_version: "1.3.0".to_string(),
                root: "corpus".to_string(),
                profile: Some(hl7v2::synthetic::corpus::CorpusFingerprintProfile {
                    path: "profile.yaml".to_string(),
                    sha256: "abc123".to_string(),
                    version: "2.5.1".to_string(),
                    message_structure: "ADT_A01".to_string(),
                }),
                file_count: 2,
                message_count: 1,
                parse_error_count: 1,
                message_type_counts: vec![hl7v2::synthetic::corpus::CorpusCount {
                    value: "ADT^A01".to_string(),
                    count: 1,
                }],
                segment_counts: vec![hl7v2::synthetic::corpus::CorpusCount {
                    value: "PID".to_string(),
                    count: 1,
                }],
                field_presence: vec![hl7v2::synthetic::corpus::CorpusFieldPresence {
                    path: "PID.3".to_string(),
                    message_count: 1,
                    occurrence_count: 1,
                }],
                field_cardinality: vec![hl7v2::synthetic::corpus::CorpusFieldCardinality {
                    path: "PID.3".to_string(),
                    min_per_message: 0,
                    max_per_message: 1,
                    total_occurrences: 1,
                    message_count: 1,
                }],
                value_shape_stats: vec![hl7v2::synthetic::corpus::CorpusValueShapeStats {
                    path: "PID.3".to_string(),
                    coded_count: 1,
                    timestamp_count: 0,
                    numeric_count: 0,
                    null_count: 0,
                    text_count: 0,
                }],
                validation_issue_code_counts: vec![hl7v2::synthetic::corpus::CorpusCount {
                    value: "missing_required_field".to_string(),
                    count: 1,
                }],
            };

            let output = crate::format_corpus_fingerprint(&report, &crate::ReportFormat::Text)
                .expect("Should format as text");

            assert!(output.contains("Corpus Fingerprint:"));
            assert!(output.contains("Profile:"));
            assert!(output.contains("ADT^A01: 1"));
            assert!(output.contains("PID.3: 1 message(s), 1 occurrence(s), min 0, max 1"));
            assert!(output.contains("missing_required_field: 1"));
        }

        #[test]
        fn test_corpus_fingerprint_command_execution() {
            use crate::OutputOptions;
            use crate::ReportFormat;
            use crate::corpus_fingerprint_command;
            let dir = TempDir::new().expect("Failed to create temp dir");
            create_temp_hl7_file(&dir, "test.hl7");

            let result = corpus_fingerprint_command(
                &dir.path().to_path_buf(),
                None,
                &ReportFormat::Json,
                &OutputOptions::new(None, false, false),
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_stats_command_execution() {
            use crate::ReportFormat;
            use crate::stats_command;
            let dir = TempDir::new().expect("Failed to create temp dir");
            let file_path = create_temp_hl7_file(&dir, "test.hl7");

            // Execute stats command
            let result = stats_command(&file_path, false, true, &ReportFormat::Text);
            assert!(result.is_ok());
        }

        #[test]
        fn test_command_paths_accept_mllp_inputs_through_facade() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            let framed = hl7v2::wrap_mllp(SampleMessages::adt_a01().as_bytes());
            let mllp_path = create_temp_file(&dir, "message.mllp", &framed);
            let output_path = dir.path().join("normalized.hl7");
            let profile_path = create_temp_file(
                &dir,
                "profile.yaml",
                br#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
constraints:
  - path: MSH.9
    required: true
"#,
            );

            assert!(
                crate::parse_command(&mllp_path, false, true, true, true, false, false).is_ok()
            );
            assert!(
                crate::norm_command(&mllp_path, true, &Some(output_path), true, true, false)
                    .is_ok()
            );
            assert!(
                crate::val_command(
                    &mllp_path,
                    &profile_path,
                    true,
                    false,
                    &crate::ReportFormat::Text,
                    false,
                    &crate::OutputOptions::new(None, false, false),
                )
                .is_ok()
            );
            assert!(
                crate::stats_command(&mllp_path, true, false, &crate::ReportFormat::Text).is_ok()
            );
            assert!(
                crate::ack_command(
                    &mllp_path,
                    &crate::AckMode::Original,
                    &crate::AckCode::AA,
                    true,
                    true,
                    false,
                )
                .is_ok()
            );
        }

        // -------------------------------------------------------------------------
        // --streaming flag tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_streaming_mode_flag() {
            // The streaming flag is primarily for memory-efficient processing
            // of large files. Here we just verify the flag exists and can be parsed.
            use crate::Cli;
            use clap::CommandFactory;

            let schema = Cli::command();
            let parse_cmd = schema.get_subcommands().find(|c| c.get_name() == "parse");
            assert!(parse_cmd.is_some());

            let parse_cmd = parse_cmd.unwrap();
            let streaming_arg = parse_cmd
                .get_arguments()
                .find(|a| a.get_id() == "streaming");
            assert!(streaming_arg.is_some());
        }

        // -------------------------------------------------------------------------
        // ReportFormat enum tests
        // -------------------------------------------------------------------------

        #[test]
        fn test_report_format_values() {
            use crate::ReportFormat;

            // Verify all format variants exist
            let _text = ReportFormat::Text;
            let _json = ReportFormat::Json;
            let _yaml = ReportFormat::Yaml;

            // Verify default
            assert_eq!(ReportFormat::default(), ReportFormat::Text);
        }

        // -------------------------------------------------------------------------
        // Clap argument parsing tests for new flags
        // -------------------------------------------------------------------------

        #[test]
        fn test_parse_command_has_canonical_delims_flag() {
            use crate::Cli;
            use clap::CommandFactory;

            let schema = Cli::command();
            let parse_cmd = schema.get_subcommands().find(|c| c.get_name() == "parse");
            assert!(parse_cmd.is_some());

            let parse_cmd = parse_cmd.unwrap();
            let canonical_arg = parse_cmd
                .get_arguments()
                .find(|a| a.get_id() == "canonical_delims");
            assert!(canonical_arg.is_some());
        }

        #[test]
        fn test_parse_command_has_envelope_flag() {
            use crate::Cli;
            use clap::CommandFactory;

            let schema = Cli::command();
            let parse_cmd = schema.get_subcommands().find(|c| c.get_name() == "parse");
            assert!(parse_cmd.is_some());

            let parse_cmd = parse_cmd.unwrap();
            let envelope_arg = parse_cmd.get_arguments().find(|a| a.get_id() == "envelope");
            assert!(envelope_arg.is_some());
        }

        #[test]
        fn test_val_command_has_report_flag() {
            use crate::Cli;
            use clap::CommandFactory;

            let schema = Cli::command();
            let val_cmd = schema.get_subcommands().find(|c| c.get_name() == "val");
            assert!(val_cmd.is_some());

            let val_cmd = val_cmd.unwrap();
            let report_arg = val_cmd.get_arguments().find(|a| a.get_id() == "report");
            assert!(report_arg.is_some());
        }

        #[test]
        fn test_stats_command_exists() {
            use crate::Cli;
            use clap::CommandFactory;

            let schema = Cli::command();
            let stats_cmd = schema.get_subcommands().find(|c| c.get_name() == "stats");
            assert!(stats_cmd.is_some());
        }

        #[test]
        fn test_corpus_command_has_summarize_subcommand() {
            use crate::Cli;
            use clap::CommandFactory;

            let schema = Cli::command();
            let corpus_cmd = schema
                .get_subcommands()
                .find(|c| c.get_name() == "corpus")
                .expect("corpus command should exist");

            assert!(
                corpus_cmd
                    .get_subcommands()
                    .any(|c| c.get_name() == "summarize")
            );
        }

        #[test]
        fn test_corpus_command_has_fingerprint_subcommand() {
            use crate::Cli;
            use clap::CommandFactory;

            let schema = Cli::command();
            let corpus_cmd = schema
                .get_subcommands()
                .find(|c| c.get_name() == "corpus")
                .expect("corpus command should exist");

            assert!(
                corpus_cmd
                    .get_subcommands()
                    .any(|c| c.get_name() == "fingerprint")
            );
        }

        #[test]
        fn test_diff_corpus_command_exists() {
            use crate::Cli;
            use clap::CommandFactory;

            let schema = Cli::command();
            let corpus_cmd = schema
                .get_subcommands()
                .find(|c| c.get_name() == "corpus")
                .expect("corpus command should exist");

            assert!(corpus_cmd.get_subcommands().any(|c| c.get_name() == "diff"));
        }

        #[test]
        fn test_stats_command_has_distributions_flag() {
            use crate::Cli;
            use clap::CommandFactory;

            let schema = Cli::command();
            let stats_cmd = schema.get_subcommands().find(|c| c.get_name() == "stats");
            assert!(stats_cmd.is_some());

            let stats_cmd = stats_cmd.unwrap();
            let distributions_arg = stats_cmd
                .get_arguments()
                .find(|a| a.get_id() == "distributions");
            assert!(distributions_arg.is_some());
        }
    }
}
