//! Unit tests for hl7v2-path crate

use super::*;

mod path_error_tests {
    use super::*;

    #[test]
    fn test_error_invalid_format() {
        let err = PathError::InvalidFormat("test error".to_string());
        assert_eq!(err.to_string(), "Invalid path format: test error");
    }

    #[test]
    fn test_error_invalid_segment_id() {
        let err = PathError::InvalidSegmentId("XX".to_string());
        assert_eq!(err.to_string(), "Invalid segment ID: XX");
    }

    #[test]
    fn test_error_invalid_field_number() {
        let err = PathError::InvalidFieldNumber("abc".to_string());
        assert_eq!(err.to_string(), "Invalid field number: abc");
    }

    #[test]
    fn test_error_invalid_component_number() {
        let err = PathError::InvalidComponentNumber("0".to_string());
        assert_eq!(err.to_string(), "Invalid component number: 0");
    }

    #[test]
    fn test_error_invalid_repetition_index() {
        let err = PathError::InvalidRepetitionIndex("-1".to_string());
        assert_eq!(err.to_string(), "Invalid repetition index: -1");
    }

    #[test]
    fn test_error_clone() {
        let err = PathError::InvalidFormat("test".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_error_partial_eq() {
        let err1 = PathError::InvalidFormat("test".to_string());
        let err2 = PathError::InvalidFormat("test".to_string());
        let err3 = PathError::InvalidSegmentId("test".to_string());

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }
}

mod path_struct_tests {
    use super::*;

    #[test]
    fn test_new_path() {
        let path = Path::new("PID", 5);
        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 5);
        assert_eq!(path.repetition, None);
        assert_eq!(path.component, None);
        assert_eq!(path.subcomponent, None);
    }

    #[test]
    fn test_new_path_lowercase_converted() {
        let path = Path::new("pid", 3);
        assert_eq!(path.segment, "PID");
    }

    #[test]
    fn test_with_repetition() {
        let path = Path::new("PID", 5).with_repetition(2);
        assert_eq!(path.repetition, Some(2));
    }

    #[test]
    fn test_with_component() {
        let path = Path::new("PID", 5).with_component(1);
        assert_eq!(path.component, Some(1));
    }

    #[test]
    fn test_with_subcomponent() {
        let path = Path::new("PID", 5).with_subcomponent(2);
        assert_eq!(path.subcomponent, Some(2));
    }

    #[test]
    fn test_builder_chain() {
        let path = Path::new("PID", 5)
            .with_repetition(2)
            .with_component(1)
            .with_subcomponent(3);

        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 5);
        assert_eq!(path.repetition, Some(2));
        assert_eq!(path.component, Some(1));
        assert_eq!(path.subcomponent, Some(3));
    }

    #[test]
    fn test_is_msh_true() {
        let path = Path::new("MSH", 9);
        assert!(path.is_msh());
    }

    #[test]
    fn test_is_msh_false() {
        let path = Path::new("PID", 5);
        assert!(!path.is_msh());
    }

    #[test]
    fn test_is_msh_lowercase() {
        let path = Path::new("msh", 9);
        assert!(path.is_msh());
    }

    #[test]
    fn test_msh_adjusted_field_msh_1() {
        let path = Path::new("MSH", 1);
        assert_eq!(path.msh_adjusted_field(), 0);
    }

    #[test]
    fn test_msh_adjusted_field_msh_2() {
        let path = Path::new("MSH", 2);
        assert_eq!(path.msh_adjusted_field(), 1);
    }

    #[test]
    fn test_msh_adjusted_field_msh_3() {
        let path = Path::new("MSH", 3);
        assert_eq!(path.msh_adjusted_field(), 1);
    }

    #[test]
    fn test_msh_adjusted_field_msh_4() {
        let path = Path::new("MSH", 4);
        assert_eq!(path.msh_adjusted_field(), 2);
    }

    #[test]
    fn test_msh_adjusted_field_msh_9() {
        let path = Path::new("MSH", 9);
        assert_eq!(path.msh_adjusted_field(), 7);
    }

    #[test]
    fn test_msh_adjusted_field_msh_12() {
        let path = Path::new("MSH", 12);
        assert_eq!(path.msh_adjusted_field(), 10);
    }

    #[test]
    fn test_to_path_string_simple() {
        let path = Path::new("PID", 5);
        assert_eq!(path.to_path_string(), "PID.5");
    }

    #[test]
    fn test_to_path_string_with_repetition() {
        let path = Path::new("PID", 5).with_repetition(2);
        assert_eq!(path.to_path_string(), "PID.5[2]");
    }

    #[test]
    fn test_to_path_string_with_component() {
        let path = Path::new("PID", 5).with_component(1);
        assert_eq!(path.to_path_string(), "PID.5.1");
    }

    #[test]
    fn test_to_path_string_with_repetition_and_component() {
        let path = Path::new("PID", 5).with_repetition(2).with_component(1);
        assert_eq!(path.to_path_string(), "PID.5[2].1");
    }

    #[test]
    fn test_to_path_string_full() {
        let path = Path::new("PID", 5)
            .with_repetition(2)
            .with_component(1)
            .with_subcomponent(3);
        assert_eq!(path.to_path_string(), "PID.5[2].1.3");
    }

    #[test]
    fn test_to_path_string_with_subcomponent_only() {
        let path = Path::new("PID", 5).with_subcomponent(2);
        // When there's no component, subcomponent is rendered as component
        // because to_path_string only adds component.subcomponent if both exist
        assert_eq!(path.to_path_string(), "PID.5.2");
    }

    #[test]
    fn test_display_trait() {
        let path = Path::new("PID", 5).with_component(1);
        assert_eq!(format!("{}", path), "PID.5.1");
    }

    #[test]
    fn test_path_clone() {
        let path = Path::new("PID", 5).with_repetition(2);
        let cloned = path.clone();
        assert_eq!(path, cloned);
    }

    #[test]
    fn test_path_partial_eq() {
        let path1 = Path::new("PID", 5).with_repetition(2);
        let path2 = Path::new("PID", 5).with_repetition(2);
        let path3 = Path::new("PID", 5).with_repetition(3);

        assert_eq!(path1, path2);
        assert_ne!(path1, path3);
    }
}

mod parse_path_tests {
    use super::*;

    #[test]
    fn test_parse_simple_field() {
        let path = parse_path("PID.5").unwrap();
        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 5);
        assert_eq!(path.repetition, None);
        assert_eq!(path.component, None);
        assert_eq!(path.subcomponent, None);
    }

    #[test]
    fn test_parse_with_component() {
        let path = parse_path("PID.5.1").unwrap();
        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 5);
        assert_eq!(path.component, Some(1));
        assert_eq!(path.subcomponent, None);
    }

    #[test]
    fn test_parse_with_subcomponent() {
        let path = parse_path("PID.5.1.2").unwrap();
        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 5);
        assert_eq!(path.component, Some(1));
        assert_eq!(path.subcomponent, Some(2));
    }

    #[test]
    fn test_parse_with_repetition() {
        let path = parse_path("PID.5[2]").unwrap();
        assert_eq!(path.field, 5);
        assert_eq!(path.repetition, Some(2));
        assert_eq!(path.component, None);
    }

    #[test]
    fn test_parse_with_repetition_and_component() {
        let path = parse_path("PID.5[2].1").unwrap();
        assert_eq!(path.field, 5);
        assert_eq!(path.repetition, Some(2));
        assert_eq!(path.component, Some(1));
    }

    #[test]
    fn test_parse_full_path() {
        let path = parse_path("PID.5[2].1.3").unwrap();
        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 5);
        assert_eq!(path.repetition, Some(2));
        assert_eq!(path.component, Some(1));
        assert_eq!(path.subcomponent, Some(3));
    }

    #[test]
    fn test_parse_lowercase_segment() {
        let path = parse_path("pid.5").unwrap();
        assert_eq!(path.segment, "PID");
    }

    #[test]
    fn test_parse_mixed_case_segment() {
        let path = parse_path("PiD.5").unwrap();
        assert_eq!(path.segment, "PID");
    }

    #[test]
    fn test_parse_numeric_segment() {
        // Some HL7 segments have digits like NK1, DG1, etc.
        let path = parse_path("NK1.2").unwrap();
        assert_eq!(path.segment, "NK1");
    }

    #[test]
    fn test_parse_msh_segment() {
        let path = parse_path("MSH.9.1").unwrap();
        assert!(path.is_msh());
        assert_eq!(path.field, 9);
        assert_eq!(path.component, Some(1));
    }

    #[test]
    fn test_parse_evn_segment() {
        let path = parse_path("EVN.1").unwrap();
        assert_eq!(path.segment, "EVN");
        assert_eq!(path.field, 1);
    }

    #[test]
    fn test_parse_pv1_segment() {
        let path = parse_path("PV1.2").unwrap();
        assert_eq!(path.segment, "PV1");
        assert_eq!(path.field, 2);
    }

    #[test]
    fn test_parse_obx_segment() {
        let path = parse_path("OBX.5[1]").unwrap();
        assert_eq!(path.segment, "OBX");
        assert_eq!(path.field, 5);
        assert_eq!(path.repetition, Some(1));
    }

    #[test]
    fn test_parse_with_whitespace() {
        let path = parse_path("  PID.5.1  ").unwrap();
        assert_eq!(path.segment, "PID");
        assert_eq!(path.field, 5);
        assert_eq!(path.component, Some(1));
    }

    #[test]
    fn test_parse_large_field_number() {
        let path = parse_path("PID.99").unwrap();
        assert_eq!(path.field, 99);
    }

    #[test]
    fn test_parse_large_component_number() {
        let path = parse_path("PID.5.999").unwrap();
        assert_eq!(path.component, Some(999));
    }

    #[test]
    fn test_parse_large_repetition_index() {
        let path = parse_path("PID.5[100]").unwrap();
        assert_eq!(path.repetition, Some(100));
    }
}

mod parse_path_error_tests {
    use super::*;

    #[test]
    fn test_parse_empty_string() {
        let result = parse_path("");
        assert!(matches!(result, Err(PathError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = parse_path("   ");
        assert!(matches!(result, Err(PathError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_segment_only() {
        let result = parse_path("PID");
        assert!(matches!(result, Err(PathError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_segment_too_short() {
        let result = parse_path("PI.5");
        assert!(matches!(result, Err(PathError::InvalidSegmentId(_))));
    }

    #[test]
    fn test_parse_segment_too_long() {
        let result = parse_path("PIDX.5");
        assert!(matches!(result, Err(PathError::InvalidSegmentId(_))));
    }

    #[test]
    fn test_parse_segment_with_special_char() {
        let result = parse_path("P-D.5");
        assert!(matches!(result, Err(PathError::InvalidSegmentId(_))));
    }

    #[test]
    fn test_parse_field_zero() {
        let result = parse_path("PID.0");
        assert!(matches!(result, Err(PathError::InvalidFieldNumber(_))));
    }

    #[test]
    fn test_parse_field_non_numeric() {
        let result = parse_path("PID.abc");
        assert!(matches!(result, Err(PathError::InvalidFieldNumber(_))));
    }

    #[test]
    fn test_parse_field_negative() {
        let result = parse_path("PID.-1");
        assert!(matches!(result, Err(PathError::InvalidFieldNumber(_))));
    }

    #[test]
    fn test_parse_repetition_zero() {
        let result = parse_path("PID.5[0]");
        assert!(matches!(result, Err(PathError::InvalidRepetitionIndex(_))));
    }

    #[test]
    fn test_parse_repetition_non_numeric() {
        let result = parse_path("PID.5[abc]");
        assert!(matches!(result, Err(PathError::InvalidRepetitionIndex(_))));
    }

    #[test]
    fn test_parse_repetition_missing_bracket() {
        let result = parse_path("PID.5[2");
        assert!(matches!(result, Err(PathError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_repetition_empty() {
        let result = parse_path("PID.5[]");
        assert!(matches!(result, Err(PathError::InvalidRepetitionIndex(_))));
    }

    #[test]
    fn test_parse_component_zero() {
        let result = parse_path("PID.5.0");
        assert!(matches!(result, Err(PathError::InvalidComponentNumber(_))));
    }

    #[test]
    fn test_parse_component_non_numeric() {
        let result = parse_path("PID.5.abc");
        assert!(matches!(result, Err(PathError::InvalidComponentNumber(_))));
    }

    #[test]
    fn test_parse_subcomponent_zero() {
        let result = parse_path("PID.5.1.0");
        assert!(matches!(result, Err(PathError::InvalidComponentNumber(_))));
    }

    #[test]
    fn test_parse_subcomponent_non_numeric() {
        let result = parse_path("PID.5.1.abc");
        assert!(matches!(result, Err(PathError::InvalidComponentNumber(_))));
    }
}

mod parse_field_part_tests {
    use super::*;

    #[test]
    fn test_parse_simple_field_part() {
        let (field, rep) = parse_field_part("5").unwrap();
        assert_eq!(field, 5);
        assert_eq!(rep, None);
    }

    #[test]
    fn test_parse_field_part_with_repetition() {
        let (field, rep) = parse_field_part("5[2]").unwrap();
        assert_eq!(field, 5);
        assert_eq!(rep, Some(2));
    }

    #[test]
    fn test_parse_field_part_repetition_one() {
        let (field, rep) = parse_field_part("10[1]").unwrap();
        assert_eq!(field, 10);
        assert_eq!(rep, Some(1));
    }

    #[test]
    fn test_parse_field_part_large_numbers() {
        let (field, rep) = parse_field_part("999[500]").unwrap();
        assert_eq!(field, 999);
        assert_eq!(rep, Some(500));
    }

    #[test]
    fn test_parse_field_part_zero_field() {
        let result = parse_field_part("0");
        assert!(matches!(result, Err(PathError::InvalidFieldNumber(_))));
    }

    #[test]
    fn test_parse_field_part_zero_repetition() {
        let result = parse_field_part("5[0]");
        assert!(matches!(result, Err(PathError::InvalidRepetitionIndex(_))));
    }

    #[test]
    fn test_parse_field_part_non_numeric_field() {
        let result = parse_field_part("abc");
        assert!(matches!(result, Err(PathError::InvalidFieldNumber(_))));
    }

    #[test]
    fn test_parse_field_part_non_numeric_repetition() {
        let result = parse_field_part("5[xyz]");
        assert!(matches!(result, Err(PathError::InvalidRepetitionIndex(_))));
    }

    #[test]
    fn test_parse_field_part_missing_close_bracket() {
        let result = parse_field_part("5[2");
        assert!(matches!(result, Err(PathError::InvalidFormat(_))));
    }
}

mod roundtrip_tests {
    use super::*;

    #[test]
    fn test_roundtrip_simple() {
        let original = "PID.5";
        let path = parse_path(original).unwrap();
        assert_eq!(path.to_path_string(), original);
    }

    #[test]
    fn test_roundtrip_with_component() {
        let original = "PID.5.1";
        let path = parse_path(original).unwrap();
        assert_eq!(path.to_path_string(), original);
    }

    #[test]
    fn test_roundtrip_with_repetition() {
        let original = "PID.5[2]";
        let path = parse_path(original).unwrap();
        assert_eq!(path.to_path_string(), original);
    }

    #[test]
    fn test_roundtrip_full() {
        let original = "PID.5[2].1.3";
        let path = parse_path(original).unwrap();
        assert_eq!(path.to_path_string(), original);
    }

    #[test]
    fn test_roundtrip_msh() {
        let original = "MSH.9.1";
        let path = parse_path(original).unwrap();
        assert_eq!(path.to_path_string(), original);
    }

    #[test]
    fn test_roundtrip_lowercase_normalized() {
        let path = parse_path("pid.5").unwrap();
        // Lowercase is normalized to uppercase
        assert_eq!(path.to_path_string(), "PID.5");
    }
}
