//! BDD tests for hl7v2-gen using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use std::collections::HashMap;

use cucumber::{World, given, then, when};
use hl7v2_gen::{
    AckCode, Faker, Message, Template, ValueSource, ack, ack_with_error, generate,
};

/// Test world for generation BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct GenWorld {
    template: Option<Template>,
    seed: u64,
    messages_a: Vec<Message>,
    messages_b: Vec<Message>,
    ack_message: Option<Message>,
    original_message: Option<Message>,
    faker_result: Option<String>,
}

impl GenWorld {
    fn new() -> Self {
        Self {
            template: None,
            seed: 0,
            messages_a: Vec::new(),
            messages_b: Vec::new(),
            ack_message: None,
            original_message: None,
            faker_result: None,
        }
    }

    fn simple_adt_template() -> Template {
        Template {
            name: "adt_a01".to_string(),
            delims: r#"^~\&"#.to_string(),
            segments: vec![
                r#"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#
                    .to_string(),
                r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
            ],
            values: HashMap::new(),
        }
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given("a simple ADT template")]
fn given_simple_template(world: &mut GenWorld) {
    world.template = Some(GenWorld::simple_adt_template());
}

#[given("seed value 42")]
fn given_seed_42(world: &mut GenWorld) {
    world.seed = 42;
}

#[given("a template with dynamic values")]
fn given_dynamic_template(world: &mut GenWorld) {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::UuidV4]);
    world.template = Some(Template {
        name: "dynamic".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#
                .to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    });
}

#[given(regex = r#"a template with PID\.5 fixed to "([^"]+)""#)]
fn given_template_fixed_pid5(world: &mut GenWorld, value: String) {
    let mut values = HashMap::new();
    values.insert(
        "PID.5".to_string(),
        vec![ValueSource::Fixed(value)],
    );
    world.template = Some(Template {
        name: "fixed".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#
                .to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    });
}

#[given(regex = r#"a template with PID\.8 from list "([^"]+)""#)]
fn given_template_from_list(world: &mut GenWorld, list: String) {
    let items: Vec<String> = list.split(',').map(|s| s.to_string()).collect();
    let mut values = HashMap::new();
    values.insert("PID.8".to_string(), vec![ValueSource::From(items)]);
    world.template = Some(Template {
        name: "from_list".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#
                .to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John||19800101|M"#.to_string(),
        ],
        values,
    });
}

#[given("a template with PID.3 as UUID")]
fn given_template_uuid(world: &mut GenWorld) {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::UuidV4]);
    world.template = Some(Template {
        name: "uuid".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#
                .to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    });
}

#[given("a template with PID.3 as 6-digit numeric")]
fn given_template_numeric(world: &mut GenWorld) {
    let mut values = HashMap::new();
    values.insert(
        "PID.3".to_string(),
        vec![ValueSource::Numeric { digits: 6 }],
    );
    world.template = Some(Template {
        name: "numeric".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#
                .to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
        ],
        values,
    });
}

#[given(regex = r#"a template with PID\.7 as date between "(\d+)" and "(\d+)""#)]
fn given_template_date_range(world: &mut GenWorld, start: String, end: String) {
    let mut values = HashMap::new();
    values.insert(
        "PID.7".to_string(),
        vec![ValueSource::Date { start, end }],
    );
    world.template = Some(Template {
        name: "date".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"#
                .to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John|||M||||"#.to_string(),
        ],
        values,
    });
}

#[given("a template with OBX.5 as gaussian mean 100.0 stddev 10.0")]
fn given_template_gaussian(world: &mut GenWorld) {
    let mut values = HashMap::new();
    values.insert(
        "OBX.5".to_string(),
        vec![ValueSource::Gaussian {
            mean: 100.0,
            sd: 10.0,
            precision: 2,
        }],
    );
    world.template = Some(Template {
        name: "gaussian".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ORU^R01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
            r#"OBR|1|||1234^Test"#.to_string(),
            r#"OBX|1|NM|1234^Result||120|mg/dL"#.to_string(),
        ],
        values,
    });
}

#[given("a valid HL7 message to acknowledge")]
fn given_valid_message_for_ack(world: &mut GenWorld) {
    let msg = hl7v2_core::parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r",
    )
    .unwrap();
    world.original_message = Some(msg);
}

#[given("an ORU template with OBX segments")]
fn given_oru_template(world: &mut GenWorld) {
    world.template = Some(Template {
        name: "oru_r01".to_string(),
        delims: r#"^~\&"#.to_string(),
        segments: vec![
            r#"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ORU^R01|ABC123|P|2.5.1"#.to_string(),
            r#"PID|1||123456^^^HOSP^MR||Doe^John"#.to_string(),
            r#"OBR|1|||1234^Test"#.to_string(),
            r#"OBX|1|NM|1234^Result||120|mg/dL"#.to_string(),
        ],
        values: HashMap::new(),
    });
}

#[given("a faker with seed 42")]
fn given_faker(_world: &mut GenWorld) {
    // Faker will be created in the When step
}

// ============================================================================
// When Steps
// ============================================================================

#[when("I generate a message")]
fn when_generate_message(world: &mut GenWorld) {
    let template = world.template.as_ref().expect("No template set");
    world.messages_a = generate(template, world.seed, 1).expect("Generation failed");
}

#[when("I generate another message with the same seed")]
fn when_generate_same_seed(world: &mut GenWorld) {
    let template = world.template.as_ref().expect("No template set");
    world.messages_b = generate(template, world.seed, 1).expect("Generation failed");
}

#[when(regex = r"I generate a message with seed (\d+)")]
fn when_generate_with_seed(world: &mut GenWorld, seed: u64) {
    let template = world.template.as_ref().expect("No template set");
    let msgs = generate(template, seed, 1).expect("Generation failed");
    if world.messages_a.is_empty() {
        world.messages_a = msgs;
    } else {
        world.messages_b = msgs;
    }
}

#[when(regex = r"I generate (\d+) messages with seed (\d+)")]
fn when_generate_n_messages(world: &mut GenWorld, count: usize, seed: u64) {
    let template = world.template.as_ref().expect("No template set");
    let msgs = generate(template, seed, count).expect("Generation failed");
    if world.messages_a.is_empty() {
        world.messages_a = msgs;
    } else {
        world.messages_b = msgs;
    }
}

#[when(regex = r"I generate (\d+) messages again with seed (\d+)")]
fn when_generate_n_again(world: &mut GenWorld, count: usize, seed: u64) {
    let template = world.template.as_ref().expect("No template set");
    world.messages_b = generate(template, seed, count).expect("Generation failed");
}

#[when("I generate an ACK with code AA")]
fn when_generate_ack_aa(world: &mut GenWorld) {
    let msg = world.original_message.as_ref().expect("No original message");
    world.ack_message = Some(ack(msg, AckCode::AA).expect("ACK generation failed"));
}

#[when(regex = r#"I generate an ACK with error code AE and text "([^"]+)""#)]
fn when_generate_ack_error(world: &mut GenWorld, text: String) {
    let msg = world.original_message.as_ref().expect("No original message");
    world.ack_message =
        Some(ack_with_error(msg, AckCode::AE, Some(&text)).expect("ACK generation failed"));
}

#[when(regex = r#"I generate a patient name for gender "([^"]+)""#)]
fn when_faker_name(world: &mut GenWorld, gender: String) {
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    world.faker_result = Some(faker.name(Some(&gender)));
}

// ============================================================================
// Then Steps
// ============================================================================

#[then("both messages should be byte-for-byte identical")]
fn then_identical(world: &mut GenWorld) {
    assert!(!world.messages_a.is_empty());
    assert!(!world.messages_b.is_empty());
    let a = hl7v2_core::write(&world.messages_a[0]);
    let b = hl7v2_core::write(&world.messages_b[0]);
    assert_eq!(a, b, "Messages with same seed should be identical");
}

#[then("the messages should differ")]
fn then_differ(world: &mut GenWorld) {
    let a = hl7v2_core::write(&world.messages_a[0]);
    let b = hl7v2_core::write(&world.messages_b[0]);
    assert_ne!(a, b, "Messages with different seeds should differ");
}

#[then(regex = r"I should receive (\d+) messages")]
fn then_receive_n(world: &mut GenWorld, count: usize) {
    assert_eq!(world.messages_a.len(), count);
}

#[then("all messages should be valid HL7")]
fn then_all_valid(world: &mut GenWorld) {
    for msg in &world.messages_a {
        let bytes = hl7v2_core::write(msg);
        let parsed = hl7v2_core::parse(&bytes);
        assert!(parsed.is_ok(), "Message should be valid HL7");
    }
}

#[then("the generated message should be valid HL7")]
fn then_generated_valid(world: &mut GenWorld) {
    let msg = &world.messages_a[0];
    let bytes = hl7v2_core::write(msg);
    let parsed = hl7v2_core::parse(&bytes);
    assert!(parsed.is_ok(), "Generated message should be valid HL7");
}

#[then(regex = r#"all PID\.8 values should be from the list "([^"]+)""#)]
fn then_values_from_list(world: &mut GenWorld, list: String) {
    let allowed: Vec<&str> = list.split(',').collect();
    for msg in &world.messages_a {
        if let Some(val) = hl7v2_core::get(msg, "PID.8") {
            assert!(
                allowed.contains(&val),
                "PID.8 value '{}' not in allowed list {:?}",
                val,
                allowed
            );
        }
    }
}

#[then("the ACK should have MSH and MSA segments")]
fn then_ack_msh_msa(world: &mut GenWorld) {
    let ack = world.ack_message.as_ref().expect("No ACK generated");
    assert_eq!(ack.segments.len(), 2);
    assert_eq!(std::str::from_utf8(&ack.segments[0].id).unwrap(), "MSH");
    assert_eq!(std::str::from_utf8(&ack.segments[1].id).unwrap(), "MSA");
}

#[then("the ACK should have MSH, MSA, and ERR segments")]
fn then_ack_msh_msa_err(world: &mut GenWorld) {
    let ack = world.ack_message.as_ref().expect("No ACK generated");
    assert_eq!(ack.segments.len(), 3);
    assert_eq!(std::str::from_utf8(&ack.segments[0].id).unwrap(), "MSH");
    assert_eq!(std::str::from_utf8(&ack.segments[1].id).unwrap(), "MSA");
    assert_eq!(std::str::from_utf8(&ack.segments[2].id).unwrap(), "ERR");
}

#[then(regex = r#"MSA\.1 should be "([^"]+)""#)]
fn then_msa1(world: &mut GenWorld, expected: String) {
    let ack = world.ack_message.as_ref().expect("No ACK generated");
    let val = hl7v2_core::get(ack, "MSA.1").expect("MSA.1 not found");
    assert_eq!(val, expected);
}

#[then(regex = r#"the generated message should contain segment "([^"]+)""#)]
fn then_contains_segment(world: &mut GenWorld, segment_id: String) {
    let msg = &world.messages_a[0];
    let has_segment = msg
        .segments
        .iter()
        .any(|s| std::str::from_utf8(&s.id).unwrap() == segment_id);
    assert!(has_segment, "Message should contain {} segment", segment_id);
}

#[then("both corpora should be identical")]
fn then_corpora_identical(world: &mut GenWorld) {
    assert_eq!(world.messages_a.len(), world.messages_b.len());
    for (a, b) in world.messages_a.iter().zip(world.messages_b.iter()) {
        let wa = hl7v2_core::write(a);
        let wb = hl7v2_core::write(b);
        assert_eq!(wa, wb, "Corpus messages with same seed should be identical");
    }
}

#[then("the name should contain a component separator")]
fn then_name_has_component_sep(world: &mut GenWorld) {
    let name = world.faker_result.as_ref().expect("No faker result");
    assert!(name.contains('^'), "Name '{}' should contain '^'", name);
}

// Run the tests
#[tokio::main]
async fn main() {
    GenWorld::cucumber().run_and_exit("./features").await;
}
