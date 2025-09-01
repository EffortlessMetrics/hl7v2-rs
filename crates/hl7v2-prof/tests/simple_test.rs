use hl7v2_prof::{load_profile, Profile};

#[test]
fn test_simple() {
    let yaml = r#"
message_structure: "ADT_A01"
version: "2.5.1"
segments:
  - id: "MSH"
  - id: "PID"
"#;
    
    let profile = load_profile(yaml).unwrap();
    assert_eq!(profile.message_structure, "ADT_A01");
}