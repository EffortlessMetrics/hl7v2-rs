//! BDD tests for hl7v2-escape using Cucumber
//!
//! Run with: cargo test --test bdd_tests -p hl7v2-escape

use cucumber::{World, given, then, when};
use hl7v2::Delims;
use hl7v2_escape::{escape_text, needs_escaping, needs_unescaping, unescape_text};

/// Test world for escape BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct EscapeWorld {
    /// The input text under test
    text: String,
    /// The delimiter configuration
    delims: Delims,
    /// The result of an escape or unescape operation
    result: Option<String>,
}

impl EscapeWorld {
    fn new() -> Self {
        Self {
            text: String::new(),
            delims: Delims::default(),
            result: None,
        }
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given(regex = r#"^the text "([^"]*)"$"#)]
fn given_text(world: &mut EscapeWorld, text: String) {
    world.text = text;
}

#[given("default delimiters")]
fn given_default_delimiters(world: &mut EscapeWorld) {
    world.delims = Delims::default();
}

#[given(regex = r#"^custom delimiters "([^"]+)"$"#)]
fn given_custom_delimiters(world: &mut EscapeWorld, delim_str: String) {
    let chars: Vec<char> = delim_str.chars().collect();
    assert_eq!(
        chars.len(),
        5,
        "Custom delimiters must be exactly 5 characters"
    );
    world.delims = Delims {
        field: chars[0],
        comp: chars[1],
        rep: chars[2],
        esc: chars[3],
        sub: chars[4],
    };
}

// ============================================================================
// When Steps
// ============================================================================

#[when("I escape the text")]
fn when_escape(world: &mut EscapeWorld) {
    world.result = Some(escape_text(&world.text, &world.delims));
}

#[when("I unescape the text")]
fn when_unescape(world: &mut EscapeWorld) {
    world.result =
        Some(unescape_text(&world.text, &world.delims).expect("unescape_text should not fail"));
}

#[when("I escape then unescape the text")]
fn when_roundtrip(world: &mut EscapeWorld) {
    let escaped = escape_text(&world.text, &world.delims);
    let unescaped = unescape_text(&escaped, &world.delims).expect("unescape_text should not fail");
    world.result = Some(unescaped);
}

// ============================================================================
// Then Steps
// ============================================================================

#[then(regex = r#"^the result should be "([^"]*)"$"#)]
fn then_result_should_be(world: &mut EscapeWorld, expected: String) {
    let actual = world.result.as_ref().expect("No result produced");
    assert_eq!(
        actual, &expected,
        "Expected '{}' but got '{}'",
        expected, actual
    );
}

#[then("needs_escaping should return true")]
fn then_needs_escaping_true(world: &mut EscapeWorld) {
    assert!(
        needs_escaping(&world.text, &world.delims),
        "Expected needs_escaping to return true for '{}'",
        world.text
    );
}

#[then("needs_escaping should return false")]
fn then_needs_escaping_false(world: &mut EscapeWorld) {
    assert!(
        !needs_escaping(&world.text, &world.delims),
        "Expected needs_escaping to return false for '{}'",
        world.text
    );
}

#[then("needs_unescaping should return true")]
fn then_needs_unescaping_true(world: &mut EscapeWorld) {
    assert!(
        needs_unescaping(&world.text, &world.delims),
        "Expected needs_unescaping to return true for '{}'",
        world.text
    );
}

#[then("needs_unescaping should return false")]
fn then_needs_unescaping_false(world: &mut EscapeWorld) {
    assert!(
        !needs_unescaping(&world.text, &world.delims),
        "Expected needs_unescaping to return false for '{}'",
        world.text
    );
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    EscapeWorld::run("features/escape.feature").await;
}
