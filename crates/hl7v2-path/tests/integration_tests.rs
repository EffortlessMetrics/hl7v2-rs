//! Integration tests for hl7v2-path crate

use hl7v2_path::{Path, PathError, parse_path};

mod real_world_path_scenarios {
    use super::*;

    /// Test parsing common ADT message paths
    #[test]
    fn test_adt_message_paths() {
        // MSH segment paths
        let msh9 = parse_path("MSH.9").unwrap();
        assert_eq!(msh9.segment, "MSH");
        assert_eq!(msh9.field, 9);

        let msh9_1 = parse_path("MSH.9.1").unwrap();
        assert_eq!(msh9_1.component, Some(1));

        let msh9_2 = parse_path("MSH.9.2").unwrap();
        assert_eq!(msh9_2.component, Some(2));

        let msh9_3 = parse_path("MSH.9.3").unwrap();
        assert_eq!(msh9_3.component, Some(3));

        // MSH-12 version ID
        let msh12 = parse_path("MSH.12").unwrap();
        assert_eq!(msh12.field, 12);
    }

    /// Test parsing PID segment paths (Patient Identification)
    #[test]
    fn test_pid_segment_paths() {
        // PID-3 Patient Identifier List (repeating)
        let pid3 = parse_path("PID.3").unwrap();
        assert_eq!(pid3.field, 3);

        let pid3_1 = parse_path("PID.3[1]").unwrap();
        assert_eq!(pid3_1.repetition, Some(1));

        let pid3_2 = parse_path("PID.3[2]").unwrap();
        assert_eq!(pid3_2.repetition, Some(2));

        // PID-3.1 ID Number
        let pid3_1_1 = parse_path("PID.3[1].1").unwrap();
        assert_eq!(pid3_1_1.repetition, Some(1));
        assert_eq!(pid3_1_1.component, Some(1));

        // PID-3.4 Assigning Authority
        let pid3_1_4 = parse_path("PID.3[1].4").unwrap();
        assert_eq!(pid3_1_4.component, Some(4));

        // PID-3.4.1 Namespace ID
        let pid3_1_4_1 = parse_path("PID.3[1].4.1").unwrap();
        assert_eq!(pid3_1_4_1.subcomponent, Some(1));

        // PID-5 Patient Name
        let pid5 = parse_path("PID.5").unwrap();
        assert_eq!(pid5.field, 5);

        // PID-5.1 Family Name
        let pid5_1 = parse_path("PID.5.1").unwrap();
        assert_eq!(pid5_1.component, Some(1));

        // PID-5.2 Given Name
        let pid5_2 = parse_path("PID.5.2").unwrap();
        assert_eq!(pid5_2.component, Some(2));

        // PID-7 Date/Time of Birth
        let pid7 = parse_path("PID.7").unwrap();
        assert_eq!(pid7.field, 7);

        // PID-8 Administrative Sex
        let pid8 = parse_path("PID.8").unwrap();
        assert_eq!(pid8.field, 8);
    }

    /// Test parsing PV1 segment paths (Patient Visit)
    #[test]
    fn test_pv1_segment_paths() {
        // PV1-2 Patient Class
        let pv1_2 = parse_path("PV1.2").unwrap();
        assert_eq!(pv1_2.field, 2);

        // PV1-3 Assigned Patient Location
        let pv1_3 = parse_path("PV1.3").unwrap();
        assert_eq!(pv1_3.field, 3);

        // PV1-3.1 Point of Care
        let pv1_3_1 = parse_path("PV1.3.1").unwrap();
        assert_eq!(pv1_3_1.component, Some(1));

        // PV1-3.2 Room
        let pv1_3_2 = parse_path("PV1.3.2").unwrap();
        assert_eq!(pv1_3_2.component, Some(2));

        // PV1-3.3 Bed
        let pv1_3_3 = parse_path("PV1.3.3").unwrap();
        assert_eq!(pv1_3_3.component, Some(3));

        // PV1-4 Admission Type
        let pv1_4 = parse_path("PV1.4").unwrap();
        assert_eq!(pv1_4.field, 4);

        // PV1-44 Admit Date/Time
        let pv1_44 = parse_path("PV1.44").unwrap();
        assert_eq!(pv1_44.field, 44);
    }

    /// Test parsing OBX segment paths (Observation Result)
    #[test]
    fn test_obx_segment_paths() {
        // OBX-1 Set ID
        let obx1 = parse_path("OBX.1").unwrap();
        assert_eq!(obx1.field, 1);

        // OBX-2 Value Type
        let obx2 = parse_path("OBX.2").unwrap();
        assert_eq!(obx2.field, 2);

        // OBX-3 Observation Identifier
        let obx3 = parse_path("OBX.3").unwrap();
        assert_eq!(obx3.field, 3);

        // OBX-3.1 Identifier
        let obx3_1 = parse_path("OBX.3.1").unwrap();
        assert_eq!(obx3_1.component, Some(1));

        // OBX-5 Observation Value (can repeat)
        let obx5 = parse_path("OBX.5").unwrap();
        assert_eq!(obx5.field, 5);

        let obx5_1 = parse_path("OBX.5[1]").unwrap();
        assert_eq!(obx5_1.repetition, Some(1));

        let obx5_2 = parse_path("OBX.5[2]").unwrap();
        assert_eq!(obx5_2.repetition, Some(2));

        // OBX-11 Observation Result Status
        let obx11 = parse_path("OBX.11").unwrap();
        assert_eq!(obx11.field, 11);
    }

    /// Test parsing NK1 segment paths (Next of Kin)
    #[test]
    fn test_nk1_segment_paths() {
        // NK1-1 Set ID
        let nk1_1 = parse_path("NK1.1").unwrap();
        assert_eq!(nk1_1.field, 1);

        // NK1-2 Name
        let nk1_2 = parse_path("NK1.2").unwrap();
        assert_eq!(nk1_2.field, 2);

        // NK1-2.1 Family Name
        let nk1_2_1 = parse_path("NK1.2.1").unwrap();
        assert_eq!(nk1_2_1.component, Some(1));

        // NK1-3 Relationship
        let nk1_3 = parse_path("NK1.3").unwrap();
        assert_eq!(nk1_3.field, 3);

        // NK1-4 Address (can repeat)
        let nk1_4 = parse_path("NK1.4[1]").unwrap();
        assert_eq!(nk1_4.repetition, Some(1));

        // NK1-5 Phone Number (can repeat)
        let nk1_5_1 = parse_path("NK1.5[1]").unwrap();
        assert_eq!(nk1_5_1.repetition, Some(1));

        let nk1_5_2 = parse_path("NK1.5[2]").unwrap();
        assert_eq!(nk1_5_2.repetition, Some(2));
    }

    /// Test parsing DG1 segment paths (Diagnosis)
    #[test]
    fn test_dg1_segment_paths() {
        // DG1-1 Set ID
        let dg1_1 = parse_path("DG1.1").unwrap();
        assert_eq!(dg1_1.field, 1);

        // DG1-2 Diagnosis Coding Method
        let dg1_2 = parse_path("DG1.2").unwrap();
        assert_eq!(dg1_2.field, 2);

        // DG1-3 Diagnosis Code
        let dg1_3 = parse_path("DG1.3").unwrap();
        assert_eq!(dg1_3.field, 3);

        // DG1-3.1 Identifier
        let dg1_3_1 = parse_path("DG1.3.1").unwrap();
        assert_eq!(dg1_3_1.component, Some(1));

        // DG1-4 Diagnosis Description
        let dg1_4 = parse_path("DG1.4").unwrap();
        assert_eq!(dg1_4.field, 4);

        // DG1-5 Diagnosis Date/Time
        let dg1_5 = parse_path("DG1.5").unwrap();
        assert_eq!(dg1_5.field, 5);
    }

    /// Test parsing AL1 segment paths (Allergy Information)
    #[test]
    fn test_al1_segment_paths() {
        // AL1-1 Set ID
        let al1_1 = parse_path("AL1.1").unwrap();
        assert_eq!(al1_1.field, 1);

        // AL1-2 Allergen Type Code
        let al1_2 = parse_path("AL1.2").unwrap();
        assert_eq!(al1_2.field, 2);

        // AL1-3 Allergen Code/Mnemonic/Description
        let al1_3 = parse_path("AL1.3").unwrap();
        assert_eq!(al1_3.field, 3);

        // AL1-3.1 Identifier
        let al1_3_1 = parse_path("AL1.3.1").unwrap();
        assert_eq!(al1_3_1.component, Some(1));

        // AL1-4 Allergy Severity Code
        let al1_4 = parse_path("AL1.4").unwrap();
        assert_eq!(al1_4.field, 4);
    }

    /// Test parsing ORC segment paths (Common Order)
    #[test]
    fn test_orc_segment_paths() {
        // ORC-1 Order Control
        let orc1 = parse_path("ORC.1").unwrap();
        assert_eq!(orc1.field, 1);

        // ORC-2 Placer Order Number
        let orc2 = parse_path("ORC.2").unwrap();
        assert_eq!(orc2.field, 2);

        // ORC-2.1 Entity Identifier
        let orc2_1 = parse_path("ORC.2.1").unwrap();
        assert_eq!(orc2_1.component, Some(1));

        // ORC-3 Filler Order Number
        let orc3 = parse_path("ORC.3").unwrap();
        assert_eq!(orc3.field, 3);

        // ORC-9 Date/Time of Transaction
        let orc9 = parse_path("ORC.9").unwrap();
        assert_eq!(orc9.field, 9);
    }

    /// Test parsing OBR segment paths (Observation Request)
    #[test]
    fn test_obr_segment_paths() {
        // OBR-1 Set ID
        let obr1 = parse_path("OBR.1").unwrap();
        assert_eq!(obr1.field, 1);

        // OBR-2 Placer Order Number
        let obr2 = parse_path("OBR.2").unwrap();
        assert_eq!(obr2.field, 2);

        // OBR-4 Universal Service Identifier
        let obr4 = parse_path("OBR.4").unwrap();
        assert_eq!(obr4.field, 4);

        // OBR-4.1 Identifier
        let obr4_1 = parse_path("OBR.4.1").unwrap();
        assert_eq!(obr4_1.component, Some(1));

        // OBR-4.2 Text
        let obr4_2 = parse_path("OBR.4.2").unwrap();
        assert_eq!(obr4_2.component, Some(2));

        // OBR-7 Observation Date/Time
        let obr7 = parse_path("OBR.7").unwrap();
        assert_eq!(obr7.field, 7);

        // OBR-25 Priority
        let obr25 = parse_path("OBR.25").unwrap();
        assert_eq!(obr25.field, 25);
    }
}

mod msh_special_handling {
    use super::*;

    /// Test MSH field adjustment for special MSH-1 and MSH-2 handling
    #[test]
    fn test_msh_field_adjustment() {
        // MSH-1 is the field separator (|)
        let msh1 = parse_path("MSH.1").unwrap();
        assert_eq!(msh1.msh_adjusted_field(), 0);

        // MSH-2 is the encoding characters (^~\&)
        let msh2 = parse_path("MSH.2").unwrap();
        assert_eq!(msh2.msh_adjusted_field(), 1);

        // MSH-3 Sending Application
        let msh3 = parse_path("MSH.3").unwrap();
        assert_eq!(msh3.msh_adjusted_field(), 1);

        // MSH-4 Sending Facility
        let msh4 = parse_path("MSH.4").unwrap();
        assert_eq!(msh4.msh_adjusted_field(), 2);

        // MSH-5 Receiving Application
        let msh5 = parse_path("MSH.5").unwrap();
        assert_eq!(msh5.msh_adjusted_field(), 3);

        // MSH-6 Receiving Facility
        let msh6 = parse_path("MSH.6").unwrap();
        assert_eq!(msh6.msh_adjusted_field(), 4);

        // MSH-7 Date/Time of Message
        let msh7 = parse_path("MSH.7").unwrap();
        assert_eq!(msh7.msh_adjusted_field(), 5);

        // MSH-8 Security
        let msh8 = parse_path("MSH.8").unwrap();
        assert_eq!(msh8.msh_adjusted_field(), 6);

        // MSH-9 Message Type
        let msh9 = parse_path("MSH.9").unwrap();
        assert_eq!(msh9.msh_adjusted_field(), 7);

        // MSH-10 Message Control ID
        let msh10 = parse_path("MSH.10").unwrap();
        assert_eq!(msh10.msh_adjusted_field(), 8);

        // MSH-11 Processing ID
        let msh11 = parse_path("MSH.11").unwrap();
        assert_eq!(msh11.msh_adjusted_field(), 9);

        // MSH-12 Version ID
        let msh12 = parse_path("MSH.12").unwrap();
        assert_eq!(msh12.msh_adjusted_field(), 10);

        // MSH-15 Accept Acknowledgment Type
        let msh15 = parse_path("MSH.15").unwrap();
        assert_eq!(msh15.msh_adjusted_field(), 13);

        // MSH-16 Application Acknowledgment Type
        let msh16 = parse_path("MSH.16").unwrap();
        assert_eq!(msh16.msh_adjusted_field(), 14);
    }

    /// Test that is_msh works correctly
    #[test]
    fn test_is_msh_detection() {
        let msh_path = parse_path("MSH.9.1").unwrap();
        assert!(msh_path.is_msh());

        let pid_path = parse_path("PID.5").unwrap();
        assert!(!pid_path.is_msh());

        let pv1_path = parse_path("PV1.3").unwrap();
        assert!(!pv1_path.is_msh());

        // Lowercase should be normalized
        let lowercase_msh = parse_path("msh.9").unwrap();
        assert!(lowercase_msh.is_msh());
    }
}

mod path_comparison_and_sorting {
    use super::*;

    /// Test path equality
    #[test]
    fn test_path_equality() {
        let path1 = parse_path("PID.5.1").unwrap();
        let path2 = parse_path("PID.5.1").unwrap();
        assert_eq!(path1, path2);

        let path3 = Path::new("PID", 5).with_component(1);
        assert_eq!(path1, path3);
    }

    /// Test path inequality
    #[test]
    fn test_path_inequality() {
        let path1 = parse_path("PID.5.1").unwrap();
        let path2 = parse_path("PID.5.2").unwrap();
        assert_ne!(path1, path2);

        let path3 = parse_path("PID.5[1].1").unwrap();
        assert_ne!(path1, path3); // one has repetition, one doesn't

        let path4 = parse_path("PV1.5.1").unwrap();
        assert_ne!(path1, path4); // different segments
    }

    /// Test that paths can be cloned
    #[test]
    fn test_path_clone() {
        let original = parse_path("PID.5[2].1.3").unwrap();
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}

mod complex_paths {
    use super::*;

    /// Test deeply nested subcomponent paths
    #[test]
    fn test_deeply_nested_paths() {
        // CX data type: ID + check digit + check digit scheme + assigning authority
        // PID-3.4.1 (Assigning Authority -> Namespace ID)
        let path = parse_path("PID.3.4.1").unwrap();
        assert_eq!(path.field, 3);
        assert_eq!(path.component, Some(4));
        assert_eq!(path.subcomponent, Some(1));

        // XPN data type: Family name + given + second + suffix + prefix + degree + name type
        // PID-5.1.1 (Family Name -> Surname)
        let path2 = parse_path("PID.5.1.1").unwrap();
        assert_eq!(path2.field, 5);
        assert_eq!(path2.component, Some(1));
        assert_eq!(path2.subcomponent, Some(1));

        // PID-5.1.2 (Family Name -> Own Surname Prefix)
        let path3 = parse_path("PID.5.1.2").unwrap();
        assert_eq!(path3.component, Some(1));
        assert_eq!(path3.subcomponent, Some(2));
    }

    /// Test paths with large repetition indices
    #[test]
    fn test_large_repetition_indices() {
        let path = parse_path("OBX.5[100]").unwrap();
        assert_eq!(path.repetition, Some(100));

        let path2 = parse_path("PID.3[999].1").unwrap();
        assert_eq!(path2.repetition, Some(999));
        assert_eq!(path2.component, Some(1));
    }

    /// Test paths with large field numbers
    #[test]
    fn test_large_field_numbers() {
        // Some segments have many fields
        let path = parse_path("PID.39").unwrap();
        assert_eq!(path.field, 39);

        let path2 = parse_path("IN1.999").unwrap();
        assert_eq!(path2.field, 999);
    }

    /// Test combining repetition with components and subcomponents
    #[test]
    fn test_combined_repetition_component_subcomponent() {
        // Full path with all parts
        let path = parse_path("PID.3[2].4.1").unwrap();
        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 3);
        assert_eq!(path.repetition, Some(2));
        assert_eq!(path.component, Some(4));
        assert_eq!(path.subcomponent, Some(1));

        // Verify to_path_string round-trips correctly
        assert_eq!(path.to_path_string(), "PID.3[2].4.1");
    }
}

mod error_handling_scenarios {
    use super::*;

    /// Test that invalid segment IDs are rejected
    #[test]
    fn test_invalid_segment_ids() {
        // Too short
        let result = parse_path("MS.5");
        assert!(matches!(result, Err(PathError::InvalidSegmentId(_))));

        // Too long
        let result = parse_path("MSHX.5");
        assert!(matches!(result, Err(PathError::InvalidSegmentId(_))));

        // Special characters
        let result = parse_path("M-H.5");
        assert!(matches!(result, Err(PathError::InvalidSegmentId(_))));

        let result = parse_path("MS*.5");
        assert!(matches!(result, Err(PathError::InvalidSegmentId(_))));
    }

    /// Test that invalid field numbers are rejected
    #[test]
    fn test_invalid_field_numbers() {
        // Zero
        let result = parse_path("PID.0");
        assert!(matches!(result, Err(PathError::InvalidFieldNumber(_))));

        // Negative (parsed as non-numeric due to dash)
        let result = parse_path("PID.-1");
        assert!(matches!(result, Err(PathError::InvalidFieldNumber(_))));

        // Non-numeric
        let result = parse_path("PID.abc");
        assert!(matches!(result, Err(PathError::InvalidFieldNumber(_))));
    }

    /// Test that invalid repetition indices are rejected
    #[test]
    fn test_invalid_repetition_indices() {
        // Zero
        let result = parse_path("PID.5[0]");
        assert!(matches!(result, Err(PathError::InvalidRepetitionIndex(_))));

        // Non-numeric
        let result = parse_path("PID.5[abc]");
        assert!(matches!(result, Err(PathError::InvalidRepetitionIndex(_))));

        // Empty
        let result = parse_path("PID.5[]");
        assert!(matches!(result, Err(PathError::InvalidRepetitionIndex(_))));

        // Missing close bracket
        let result = parse_path("PID.5[2");
        assert!(matches!(result, Err(PathError::InvalidFormat(_))));
    }

    /// Test that invalid component numbers are rejected
    #[test]
    fn test_invalid_component_numbers() {
        // Zero
        let result = parse_path("PID.5.0");
        assert!(matches!(result, Err(PathError::InvalidComponentNumber(_))));

        // Non-numeric
        let result = parse_path("PID.5.abc");
        assert!(matches!(result, Err(PathError::InvalidComponentNumber(_))));
    }

    /// Test that invalid subcomponent numbers are rejected
    #[test]
    fn test_invalid_subcomponent_numbers() {
        // Zero
        let result = parse_path("PID.5.1.0");
        assert!(matches!(result, Err(PathError::InvalidComponentNumber(_))));

        // Non-numeric
        let result = parse_path("PID.5.1.abc");
        assert!(matches!(result, Err(PathError::InvalidComponentNumber(_))));
    }

    /// Test that malformed paths are rejected
    #[test]
    fn test_malformed_paths() {
        // Empty
        let result = parse_path("");
        assert!(matches!(result, Err(PathError::InvalidFormat(_))));

        // Whitespace only
        let result = parse_path("   ");
        assert!(matches!(result, Err(PathError::InvalidFormat(_))));

        // Segment only
        let result = parse_path("PID");
        assert!(matches!(result, Err(PathError::InvalidFormat(_))));

        // Dot only
        let result = parse_path(".");
        assert!(matches!(result, Err(PathError::InvalidSegmentId(_))));
    }
}

mod path_display_and_formatting {
    use super::*;

    /// Test Display trait implementation
    #[test]
    fn test_display_format() {
        let path = parse_path("PID.5.1").unwrap();
        assert_eq!(format!("{}", path), "PID.5.1");

        let path2 = parse_path("MSH.9[1].2").unwrap();
        assert_eq!(format!("{}", path2), "MSH.9[1].2");
    }

    /// Test to_path_string round-trip
    #[test]
    fn test_to_path_string_roundtrip() {
        let paths = vec![
            "MSH.9",
            "MSH.9.1",
            "MSH.9.1.2",
            "PID.5",
            "PID.5.1",
            "PID.5[1]",
            "PID.5[2].1",
            "PID.5[2].1.3",
            "OBX.5[100]",
            "PV1.44",
        ];

        for path_str in paths {
            let path = parse_path(path_str).unwrap();
            assert_eq!(path.to_path_string(), path_str);
        }
    }

    /// Test that lowercase segments are normalized to uppercase
    #[test]
    fn test_lowercase_normalization() {
        let path = parse_path("pid.5").unwrap();
        assert_eq!(path.segment, "PID");
        assert_eq!(path.to_path_string(), "PID.5");

        let path2 = parse_path("msh.9.1").unwrap();
        assert_eq!(path2.segment, "MSH");
        assert_eq!(path2.to_path_string(), "MSH.9.1");
    }

    /// Test that whitespace is trimmed
    #[test]
    fn test_whitespace_trimming() {
        let path = parse_path("  PID.5.1  ").unwrap();
        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 5);
        assert_eq!(path.component, Some(1));
    }
}

mod builder_pattern {
    use super::*;

    /// Test the builder pattern for constructing paths
    #[test]
    fn test_builder_simple() {
        let path = Path::new("PID", 5);
        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 5);
        assert_eq!(path.repetition, None);
        assert_eq!(path.component, None);
        assert_eq!(path.subcomponent, None);
    }

    #[test]
    fn test_builder_with_repetition() {
        let path = Path::new("PID", 5).with_repetition(2);
        assert_eq!(path.repetition, Some(2));
    }

    #[test]
    fn test_builder_with_component() {
        let path = Path::new("PID", 5).with_component(1);
        assert_eq!(path.component, Some(1));
    }

    #[test]
    fn test_builder_with_subcomponent() {
        let path = Path::new("PID", 5).with_subcomponent(2);
        assert_eq!(path.subcomponent, Some(2));
    }

    #[test]
    fn test_builder_full_chain() {
        let path = Path::new("PID", 3)
            .with_repetition(2)
            .with_component(4)
            .with_subcomponent(1);

        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 3);
        assert_eq!(path.repetition, Some(2));
        assert_eq!(path.component, Some(4));
        assert_eq!(path.subcomponent, Some(1));

        assert_eq!(path.to_path_string(), "PID.3[2].4.1");
    }

    /// Test that builder produces equivalent paths to parser
    #[test]
    fn test_builder_matches_parser() {
        let parsed = parse_path("PID.5[2].1.3").unwrap();
        let built = Path::new("PID", 5)
            .with_repetition(2)
            .with_component(1)
            .with_subcomponent(3);

        assert_eq!(parsed, built);
    }
}
