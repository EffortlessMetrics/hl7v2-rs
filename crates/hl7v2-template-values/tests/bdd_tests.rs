//! BDD tests for hl7v2-template-values

use cucumber::{World, given, then, when};
use hl7v2_template_values::{ValueSource, generate_value};
use rand::SeedableRng;
use rand::rngs::StdRng;

#[derive(Debug, World)]
#[world(init = Self::new)]
struct ValueSourceWorld {
    source: Option<ValueSource>,
    result: Option<String>,
    error: Option<String>,
}

impl ValueSourceWorld {
    fn new() -> Self {
        Self {
            source: None,
            result: None,
            error: None,
        }
    }
}

#[given(regex = r#"a fixed value source of "([^"]+)""#)]
fn given_fixed_value_source(world: &mut ValueSourceWorld, value: String) {
    world.source = Some(ValueSource::Fixed(value));
    world.result = None;
    world.error = None;
}

#[given(regex = r#"a from-value source with options "([^"]+)", "([^"]+)", "([^"]+)""#)]
fn given_from_value_source(world: &mut ValueSourceWorld, a: String, b: String, c: String) {
    world.source = Some(ValueSource::From(vec![a, b, c]));
    world.result = None;
    world.error = None;
}

#[given("an injected invalid segment id value source")]
fn given_invalid_segment_id(world: &mut ValueSourceWorld) {
    world.source = Some(ValueSource::InvalidSegmentId);
    world.result = None;
    world.error = None;
}

#[when("I generate the value")]
fn when_generate_value(world: &mut ValueSourceWorld) {
    generate_value_impl(world);
}

#[when("I attempt to generate the value")]
fn when_attempt_generate_value(world: &mut ValueSourceWorld) {
    generate_value_impl(world);
}

fn generate_value_impl(world: &mut ValueSourceWorld) {
    if let Some(source) = &world.source {
        let mut rng = StdRng::seed_from_u64(42);
        match generate_value(source, &mut rng) {
            Ok(result) => {
                world.result = Some(result);
                world.error = None;
            }
            Err(err) => {
                world.result = None;
                world.error = Some(err.to_string());
            }
        }
    }
}

#[then(regex = r#"the generated value should be "([^"]+)""#)]
fn then_result_matches(world: &mut ValueSourceWorld, expected: String) {
    let actual = world.result.as_deref().expect("Missing generated value");
    assert_eq!(actual, expected);
}

#[then(regex = r#"the generated value should be one of "([^"]+)", "([^"]+)", or "([^"]+)""#)]
fn then_result_one_of(world: &mut ValueSourceWorld, a: String, b: String, c: String) {
    let actual = world.result.as_deref().expect("Missing generated value");
    assert!(
        actual == a || actual == b || actual == c,
        "unexpected generated value: {}",
        actual
    );
}

#[then("the generation should fail")]
fn then_generation_fails(world: &mut ValueSourceWorld) {
    assert!(world.error.is_some());
}

#[tokio::main]
async fn main() {
    ValueSourceWorld::cucumber()
        .run_and_exit("./features")
        .await;
}
