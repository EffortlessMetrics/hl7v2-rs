#[cfg(test)]
mod tests {
    use crate::{load_profile, validate};

    #[test]
    fn test_load_simple_profile() {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
message_type: "ADT^A01"
segments:
  - id: "MSH"
  - id: "PID"
  - id: "PV1"
constraints:
  - path: "PID.5.1"
    required: true
  - path: "PID.5.2"
    required: true
"#;
        
        let profile = load_profile(yaml).unwrap();
        assert_eq!(profile.message_structure, "ADT_A01");
        assert_eq!(profile.version, "2.5.1");
        assert_eq!(profile.message_type, Some("ADT^A01".to_string()));
        assert_eq!(profile.segments.len(), 3);
        assert_eq!(profile.segments[0].id, "MSH");
        assert_eq!(profile.segments[1].id, "PID");
        assert_eq!(profile.segments[2].id, "PV1");
        assert_eq!(profile.constraints.len(), 2);
        assert_eq!(profile.constraints[0].path, "PID.5.1");
        assert_eq!(profile.constraints[0].required, true);
        assert_eq!(profile.constraints[1].path, "PID.5.2");
        assert_eq!(profile.constraints[1].required, true);
    }

    #[test]
    fn test_load_profile_with_valueset() {
        let yaml = r#"
message_structure: "ORU_R01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
valuesets:
  - path: "PID.8"
    name: "AdministrativeSex"
    codes:
      - "F"
      - "M"
      - "U"
      - "A"
      - "N"
"#;
        
        let profile = load_profile(yaml).unwrap();
        assert_eq!(profile.message_structure, "ORU_R01");
        assert_eq!(profile.version, "2.5.1");
        assert_eq!(profile.valuesets.len(), 1);
        assert_eq!(profile.valuesets[0].path, "PID.8");
        assert_eq!(profile.valuesets[0].name, "AdministrativeSex");
        assert_eq!(profile.valuesets[0].codes.len(), 5);
        assert!(profile.valuesets[0].codes.contains(&"F".to_string()));
        assert!(profile.valuesets[0].codes.contains(&"M".to_string()));
    }

    #[test]
    fn test_basic_validation() {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.5.1"
    required: true
  - path: "PID.5.2"
    required: true
valuesets:
  - path: "PID.8"
    name: "AdministrativeSex"
    codes:
      - "F"
      - "M"
      - "U"
"#;
        
        let profile = load_profile(yaml).unwrap();
        
        // Test with a valid message
        // PID segment: PID|1||123456^^^HOSP^MR||Doe^John|||M||||
        // This ensures PID.8 has the value "M"
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John|||M||||\r";
        let message = hl7v2_core::parse(hl7_text.as_bytes()).unwrap();
        let issues = validate(&message, &profile);
        assert_eq!(issues.len(), 0);
        
        // Test with a message missing required fields
        // PID segment: PID|1||123456^^^HOSP^MR|
        // This ensures PID.5.1 and PID.5.2 are missing
        let hl7_text_missing = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR|\r";
        let message_missing = hl7v2_core::parse(hl7_text_missing.as_bytes()).unwrap();
        let issues_missing = validate(&message_missing, &profile);
        assert_eq!(issues_missing.len(), 2);
        assert_eq!(issues_missing[0].code, "MISSING_REQUIRED_FIELD");
        assert_eq!(issues_missing[1].code, "MISSING_REQUIRED_FIELD");
        
        // Test with a message having invalid value set
        // PID segment: PID|1||123456^^^HOSP^MR||Doe^John|||X||||
        // This ensures PID.8 has the value "X" which is not in the allowed set
        let hl7_text_invalid = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John|||X||||\r";
        let message_invalid = hl7v2_core::parse(hl7_text_invalid.as_bytes()).unwrap();
        let issues_invalid = validate(&message_invalid, &profile);
        assert_eq!(issues_invalid.len(), 1);
        assert_eq!(issues_invalid[0].code, "VALUE_NOT_IN_SET");
    }

    #[test]
    fn test_debug_valid_message() {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.5.1"
    required: true
  - path: "PID.5.2"
    required: true
valuesets:
  - path: "PID.8"
    name: "AdministrativeSex"
    codes:
      - "F"
      - "M"
      - "U"
"#;
        
        let profile = load_profile(yaml).unwrap();
        
        // Test with a valid message
        // PID segment: PID|1||123456^^^HOSP^MR||Doe^John|||M||||
        // This ensures PID.8 has the value "M"
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John|||M||||\r";
        let message = hl7v2_core::parse(hl7_text.as_bytes()).unwrap();
        
        // Debug what the get function returns for various paths
        println!("PID.5.1: {:?}", hl7v2_core::get(&message, "PID.5.1"));
        println!("PID.5.2: {:?}", hl7v2_core::get(&message, "PID.5.2"));
        println!("PID.8: {:?}", hl7v2_core::get(&message, "PID.8"));
        
        let issues = validate(&message, &profile);
        println!("Valid message issues: {:?}", issues);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_debug_get_function() {
        // Test with a valid message
        // PID segment: PID|1||123456^^^HOSP^MR||Doe^John|||M||||
        // This ensures PID.8 has the value "M"
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John|||M||||\r";
        let message = hl7v2_core::parse(hl7_text.as_bytes()).unwrap();
        
        // Debug what the get function returns for various paths
        println!("PID.5.1: {:?}", hl7v2_core::get(&message, "PID.5.1"));
        println!("PID.5.2: {:?}", hl7v2_core::get(&message, "PID.5.2"));
        println!("PID.8: {:?}", hl7v2_core::get(&message, "PID.8"));
    }

    #[test]
    fn test_debug_hl7_structure() {
        // Test with a valid message
        // PID segment: PID|1||123456^^^HOSP^MR||Doe^John|||M||||
        // This ensures PID.8 has the value "M"
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John|||M||||\r";
        let message = hl7v2_core::parse(hl7_text.as_bytes()).unwrap();
        
        // Debug the structure of the PID segment
        for (i, segment) in message.segments.iter().enumerate() {
            let segment_id = std::str::from_utf8(&segment.id).unwrap();
            println!("Segment {}: {} with {} fields", i, segment_id, segment.fields.len());
            if segment_id == "PID" {
                for (j, field) in segment.fields.iter().enumerate() {
                    println!("  Field {}: {} reps", j, field.reps.len());
                    for (k, rep) in field.reps.iter().enumerate() {
                        println!("    Rep {}: {} comps", k, rep.comps.len());
                        for (l, comp) in rep.comps.iter().enumerate() {
                            println!("      Comp {}: {} subs", l, comp.subs.len());
                            for (m, sub) in comp.subs.iter().enumerate() {
                                match sub {
                                    hl7v2_core::Atom::Text(text) => println!("        Sub {}: Text({})", m, text),
                                    hl7v2_core::Atom::Null => println!("        Sub {}: Null", m),
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_data_type_validation() {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
datatypes:
  - path: "PID.1"
    type: "SI"  # Sequence ID
  - path: "PID.8"
    type: "ID"  # Administrative Sex
  - path: "MSH.7"
    type: "TS"  # Time Stamp
"#;
        
        let profile = load_profile(yaml).unwrap();
        
        // Test with a valid message
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John|||M||||\r";
        let message = hl7v2_core::parse(hl7_text.as_bytes()).unwrap();
        let issues = validate(&message, &profile);
        assert_eq!(issues.len(), 0);
        
        // Test with invalid data types
        let hl7_text_invalid = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|invalid_timestamp||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|invalid_seq_id||123456^^^HOSP^MR||Doe^John|||M||||\r";
        let message_invalid = hl7v2_core::parse(hl7_text_invalid.as_bytes()).unwrap();
        let issues_invalid = validate(&message_invalid, &profile);
        assert_eq!(issues_invalid.len(), 2);
        assert_eq!(issues_invalid[0].code, "INVALID_DATA_TYPE");
        assert_eq!(issues_invalid[1].code, "INVALID_DATA_TYPE");
    }

    #[test]
    fn test_length_validation() {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
lengths:
  - path: "PID.1"
    max: 4  # PID.1 (Sequence Number) max length 4
  - path: "PID.5.1"
    max: 50  # PID.5.1 (Family Name) max length 50
  - path: "PID.5.2"
    max: 50  # PID.5.2 (Given Name) max length 50
"#;
        
        let profile = load_profile(yaml).unwrap();
        
        // Test with a valid message (values within length limits)
        let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John|||M||||\r";
        let message = hl7v2_core::parse(hl7_text.as_bytes()).unwrap();
        let issues = validate(&message, &profile);
        assert_eq!(issues.len(), 0);
        
        // Test with values exceeding length limits
        let long_name = "A".repeat(51); // 51 characters, exceeds max of 50
        let hl7_text_invalid = format!("MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||{}^John|||M||||\r", long_name);
        let message_invalid = hl7v2_core::parse(hl7_text_invalid.as_bytes()).unwrap();
        let issues_invalid = validate(&message_invalid, &profile);
        assert_eq!(issues_invalid.len(), 1);
        assert_eq!(issues_invalid[0].code, "VALUE_TOO_LONG");
    }

    #[test]
    fn test_conditional_validation() {
        let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
constraints:
  - path: "PID.5.1"
    required: true
    when:
      eq:
        - "PID.8"
        - "M"
  - path: "PID.6"
    required: true
    when:
      eq:
        - "PID.8"
        - "F"
"#;
        
        let profile = load_profile(yaml).unwrap();
        
        // Test with male patient (PID.8 = "M") and required PID.5.1 present
        let hl7_text_male = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John|||M||||\r";
        let message_male = hl7v2_core::parse(hl7_text_male.as_bytes()).unwrap();
        let issues_male = validate(&message_male, &profile);
        assert_eq!(issues_male.len(), 0);
        
        // Test with female patient (PID.8 = "F") and required PID.6 present
        let hl7_text_female = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR|||Doe^Jane||F||||\r";
        let message_female = hl7v2_core::parse(hl7_text_female.as_bytes()).unwrap();
        let issues_female = validate(&message_female, &profile);
        assert_eq!(issues_female.len(), 0);
        
        // Test with male patient (PID.8 = "M") but missing required PID.5.1
        let hl7_text_male_invalid = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR|||||M||||\r";
        let message_male_invalid = hl7v2_core::parse(hl7_text_male_invalid.as_bytes()).unwrap();
        let issues_male_invalid = validate(&message_male_invalid, &profile);
        assert_eq!(issues_male_invalid.len(), 1);
        assert_eq!(issues_male_invalid[0].code, "MISSING_REQUIRED_FIELD");
    }
}