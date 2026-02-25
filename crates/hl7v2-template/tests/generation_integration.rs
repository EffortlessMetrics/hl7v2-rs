use hl7v2_template::{generate, Template, ValueSource};
use std::collections::HashMap;

#[test]
fn integration_uses_externalized_value_source() {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::Numeric { digits: 6 }]);
    values.insert("PV1.4".to_string(), vec![ValueSource::Fixed("WARD01".to_string())]);

    let template = Template {
        name: "integration".to_string(),
        delims: "^~\\&".to_string(),
        segments: vec![
            "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
            "PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            "PV1|1||ER|".to_string(),
        ],
        values,
    };

    let first = generate(&template, 42, 1).expect("message generation should succeed");
    let second = generate(&template, 42, 1).expect("message generation should succeed");
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(first[0].segments.len(), second[0].segments.len());
}
