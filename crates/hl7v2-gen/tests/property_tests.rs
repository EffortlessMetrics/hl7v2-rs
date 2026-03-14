use hl7v2_gen::{Template, generate};
use std::collections::HashMap;

// Basic test to verify the file works
#[test]
fn test_template_creation() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|123|P|2.5"#.to_string(),
        ],
        values: HashMap::new(),
    };
    assert!(!template.segments.is_empty());
}

// Simple generation test
#[test]
fn test_generate_simple_message() {
    let template = Template {
        name: "test".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|123|P|2.5"#.to_string(),
            r#"PID|1||456|Doe^John"#.to_string(),
        ],
        values: HashMap::new(),
    };

    let messages = generate(&template, 42, 2).unwrap();
    assert_eq!(messages.len(), 2);

    for message in &messages {
        assert_eq!(message.segments.len(), 2);
        assert_eq!(std::str::from_utf8(&message.segments[0].id).unwrap(), "MSH");
        assert_eq!(std::str::from_utf8(&message.segments[1].id).unwrap(), "PID");
    }
}
