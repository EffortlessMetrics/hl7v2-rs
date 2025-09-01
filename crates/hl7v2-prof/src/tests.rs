#[cfg(test)]
mod tests {
    use crate::{load_profile, validate, Profile};
    use hl7v2_core::parse;

    // Helper: build a tiny valid ADT A01 (PID.3 and PID.8 filled)
    fn adt_a01_msg() -> String {
        let mut s = String::new();
        s.push_str("MSH|^~\\&|SND|SF|RCV|RF|20250101000000||ADT^A01|MSG1|P|2.5.1\r");
        s.push_str("PID|1||123456^^^HOSP^MR||Doe^John||19800101|M||||||||||||||||\r");
        s
    }

    #[test]
    fn test_load_simple_profile() {
        let y = r#"
message_structure: "simple"
version: "2.5.1"
segments:
  - id: "PID"
constraints:
  - path: "PID.3"
    required: true
  - path: "PID.8"
    required: true
"#;
        let p: Profile = load_profile(y).unwrap();
        let msg = parse(&adt_a01_msg()).unwrap();
        let probs = validate(&msg, &p);
        assert!(probs.is_empty(), "unexpected problems: {probs:?}");
    }

    #[test]
    fn test_cross_field_equals() {
        let y = r#"
message_structure: "xfield"
version: "2.5.1"
segments:
  - id: "PID"
cross_field_rules:
  - id: "test-rule"
    description: "Sex must be M"
    conditions:
      - field: "PID.8"
        operator: "eq"
        value: "M"
    actions: []
"#;
        let p: Profile = load_profile(y).unwrap();
        let msg = parse(&adt_a01_msg()).unwrap();
        let probs = validate(&msg, &p);
        assert!(probs.is_empty(), "unexpected problems: {probs:?}");
    }
}