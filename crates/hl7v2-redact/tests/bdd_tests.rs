//! BDD-style tests for hl7v2-redact using cucumber
//!
//! Run with: cargo test --test bdd_tests -p hl7v2-redact

use cucumber::{World, given, then, when};
use hl7v2_parser::parse;
use hl7v2_redact::{RedactionEngine, RedactionResult, RedactionRule, RedactionStrategy};

/// Test world for redact BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct RedactWorld {
    message: Option<String>,
    parsed_message: Option<hl7v2_model::Message>,
    redaction_result: Option<RedactionResult>,
    redaction_engine: Option<RedactionEngine>,
    rules: Vec<RedactionRule>,
    error: Option<String>,
}

impl RedactWorld {
    fn new() -> Self {
        Self {
            message: None,
            parsed_message: None,
            redaction_result: None,
            redaction_engine: None,
            rules: Vec::new(),
            error: None,
        }
    }
}

// Given Steps

#[given("a realistic HL7 message with PHI")]
fn given_realistic_hl7_with_phi(world: &mut RedactWorld) {
    let hl7 = "MSH|^~\\&|HOSPITAL_ADT|MAIN_HOSPITAL|LAB_SYSTEM|LAB|20250128120000||ADT^A01^ADT_A01|MSG00001|P|2.5\r\
EVN|A01|20250128120000|||USER123\r\
PID|1||123456789^^^HOSPITAL^MRN||Doe^John^Michael^Jr^^L||19800115|M|||123 Main Street^^Springfield^IL^62701^USA^^^H||555-123-4567|555-987-6543||S|C|123456789|123-45-6789\r\
NK1|1|Smith^Jane^Marie|SPOUSE|||555-456-7890||E\\C\r\
PV1|1|I|200^201^01|R|||DOC12345^Smith^John^M^^^MD|DOC67890^Jones^Mary^A^^^MD|||MED||||A|||1234567890|COMP_INS|||||\r\
OBX|1|ST|12345^Test^L||Positive||||||F\r";
    world.message = Some(hl7.to_string());
    world.parsed_message = parse(hl7.as_bytes()).ok();
}

#[given("a redaction engine with common PHI rules")]
fn given_common_phi_engine(world: &mut RedactWorld) {
    world.redaction_engine = Some(RedactionEngine::common_phi_rules());
}

#[given("a redaction engine with HIPAA Safe Harbor rules")]
fn given_hipaa_safe_harbor_engine(world: &mut RedactWorld) {
    world.redaction_engine = Some(RedactionEngine::hipaa_safe_harbor());
}

#[given(regex = r#"^a redaction rule for path "([^"]*)" with strategy "([^"]*)"$"#)]
fn given_redaction_rule(world: &mut RedactWorld, path: String, strategy: String) {
    let strategy = match strategy.as_str() {
        "hash" => RedactionStrategy::Hash,
        "mask" => RedactionStrategy::Mask,
        "remove" => RedactionStrategy::Remove,
        "replace" => RedactionStrategy::Replace("[REDACTED]".to_string()),
        _ => RedactionStrategy::Mask,
    };
    world.rules.push(RedactionRule::new(path, strategy));
}

#[given("an empty HL7 message")]
fn given_empty_message(world: &mut RedactWorld) {
    let hl7 = "MSH|^~\\&|HOSPITAL|FACILITY|20250128120000||ADT^A01|MSG00001|P|2.5\r";
    world.message = Some(hl7.to_string());
    world.parsed_message = parse(hl7.as_bytes()).ok();
}

#[given("an HL7 message with no PHI")]
fn given_no_phi_message(world: &mut RedactWorld) {
    let hl7 =
        "MSH|^~\\&|HOSPITAL|FACILITY|20250128120000||ACK^A01|MSG00001|P|2.5\rMSA|AA|MSG00001\r";
    world.message = Some(hl7.to_string());
    world.parsed_message = parse(hl7.as_bytes()).ok();
}

// When Steps

#[when("I apply redaction rules")]
fn when_apply_redaction(world: &mut RedactWorld) {
    if let Some(ref engine) = world.redaction_engine
        && let Some(ref message) = world.parsed_message
    {
        match engine.redact(message) {
            Ok(result) => world.redaction_result = Some(result),
            Err(e) => world.error = Some(e.to_string()),
        }
    }
}

#[when("I create a redaction engine with the configured rules")]
#[given("I create a redaction engine with the configured rules")]
fn when_create_engine(world: &mut RedactWorld) {
    world.redaction_engine = Some(RedactionEngine::new(world.rules.clone()));
}

#[when("I apply the redaction")]
fn when_apply_redaction_with_engine(world: &mut RedactWorld) {
    if let Some(ref engine) = world.redaction_engine
        && let Some(ref message) = world.parsed_message
    {
        match engine.redact(message) {
            Ok(result) => world.redaction_result = Some(result),
            Err(e) => world.error = Some(e.to_string()),
        }
    }
}

// Then Steps

#[then("the message should be successfully redacted")]
fn then_successfully_redacted(world: &mut RedactWorld) {
    assert!(
        world.redaction_result.is_some(),
        "Redaction should produce a result"
    );
    assert!(world.error.is_none(), "No error should occur");
}

#[then("the patient identifier should be hashed")]
fn then_patient_id_hashed(world: &mut RedactWorld) {
    if let Some(ref result) = world.redaction_result {
        let redacted = String::from_utf8_lossy(&result.message_bytes);
        // Should not contain raw patient identifiers
        assert!(
            !redacted.contains("123456789^^^HOSPITAL^MRN"),
            "Raw patient ID should not be present"
        );
    }
}

#[then("the patient name should be masked")]
fn then_patient_name_masked(world: &mut RedactWorld) {
    if let Some(ref result) = world.redaction_result {
        let redacted = String::from_utf8_lossy(&result.message_bytes);
        assert!(
            !redacted.contains("Doe^John^Michael"),
            "Patient name should be masked"
        );
    }
}

#[then("the address should be removed")]
fn then_address_removed(world: &mut RedactWorld) {
    if let Some(ref result) = world.redaction_result {
        let redacted = String::from_utf8_lossy(&result.message_bytes);
        assert!(
            !redacted.contains("123 Main Street"),
            "Address should be removed"
        );
    }
}

#[then("the phone number should be masked")]
fn then_phone_masked(world: &mut RedactWorld) {
    if let Some(ref result) = world.redaction_result {
        let redacted = String::from_utf8_lossy(&result.message_bytes);
        assert!(
            !redacted.contains("555-123-4567"),
            "Phone number should be masked"
        );
    }
}

#[then("an audit log should be generated")]
fn then_audit_log_generated(world: &mut RedactWorld) {
    if let Some(ref result) = world.redaction_result {
        assert!(
            !result.audit_log.is_empty(),
            "Audit log should not be empty"
        );
    }
}

#[then(regex = r"^the audit log should contain (\d+) entries$")]
fn then_audit_log_entries(world: &mut RedactWorld, count: usize) {
    if let Some(ref result) = world.redaction_result {
        assert_eq!(
            result.audit_log.len(),
            count,
            "Audit log should have {} entries",
            count
        );
    }
}

#[then("the redacted message should be valid HL7")]
fn then_valid_hl7(world: &mut RedactWorld) {
    if let Some(ref result) = world.redaction_result {
        let reparsed = parse(&result.message_bytes);
        assert!(reparsed.is_ok(), "Redacted message should be valid HL7");
    }
}

#[then("the message structure should be preserved")]
fn then_structure_preserved(world: &mut RedactWorld) {
    if let (Some(original), Some(result)) = (&world.parsed_message, &world.redaction_result) {
        let reparsed = parse(&result.message_bytes).unwrap();
        assert_eq!(
            original.segments.len(),
            reparsed.segments.len(),
            "Segment count should be preserved"
        );
    }
}

#[then("no error should occur")]
fn then_no_error(world: &mut RedactWorld) {
    assert!(
        world.error.is_none(),
        "No error should occur: {:?}",
        world.error
    );
}

#[then("the SSN should be hashed")]
fn then_ssn_hashed(world: &mut RedactWorld) {
    if let Some(ref result) = world.redaction_result {
        let redacted = String::from_utf8_lossy(&result.message_bytes);
        assert!(!redacted.contains("123-45-6789"), "SSN should be hashed");
    }
}

#[then("the date of birth should be replaced")]
fn then_dob_replaced(world: &mut RedactWorld) {
    if let Some(ref result) = world.redaction_result {
        let redacted = String::from_utf8_lossy(&result.message_bytes);
        assert!(!redacted.contains("19800115"), "DOB should be replaced");
    }
}

#[tokio::main]
async fn main() {
    RedactWorld::run("tests/features/redaction.feature").await;
}
