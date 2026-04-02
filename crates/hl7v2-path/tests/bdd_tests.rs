//! BDD tests for hl7v2-path using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use cucumber::{World, given, then, when};
use hl7v2_path::{Path, PathError, parse_path};

/// Test world for Path BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct PathWorld {
    /// Path string being parsed
    path_string: String,
    /// Parsed path result
    path_result: Option<Result<Path, PathError>>,
    /// Parsed path (if successful)
    path: Option<Path>,
    /// Error (if parsing failed)
    error: Option<PathError>,
    /// Formatted path string
    formatted_path: String,
}

impl PathWorld {
    fn new() -> Self {
        Self {
            path_string: String::new(),
            path_result: None,
            path: None,
            error: None,
            formatted_path: String::new(),
        }
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given(regex = r#"^the path string "([^"]+)"$"#)]
fn given_path_string(world: &mut PathWorld, path: String) {
    world.path_string = path;
}

#[given("a parsed path with segment \"PID\" field 5 component 1")]
fn given_parsed_path_pid_5_1(world: &mut PathWorld) {
    world.path = Some(Path::new("PID", 5).with_component(1));
}

#[given("a parsed path with segment \"PID\" field 5 repetition 2 component 1")]
fn given_parsed_path_pid_5_2_1(world: &mut PathWorld) {
    world.path = Some(Path::new("PID", 5).with_repetition(2).with_component(1));
}

#[given("a parsed path with segment \"PID\" field 5 component 1 subcomponent 1")]
fn given_parsed_path_pid_5_1_1(world: &mut PathWorld) {
    world.path = Some(Path::new("PID", 5).with_component(1).with_subcomponent(1));
}

// ============================================================================
// When Steps
// ============================================================================

#[when("I parse the path")]
fn when_parse_path(world: &mut PathWorld) {
    world.path_result = Some(parse_path(&world.path_string));
    match world.path_result.as_ref().unwrap() {
        Ok(p) => world.path = Some(p.clone()),
        Err(e) => world.error = Some(e.clone()),
    }
}

#[when("I attempt to parse the path")]
fn when_attempt_parse_path(world: &mut PathWorld) {
    when_parse_path(world);
}

#[when("I format the path to string")]
fn when_format_path(world: &mut PathWorld) {
    let path = world.path.as_ref().expect("No path");
    world.formatted_path = path.to_path_string();
}

// ============================================================================
// Then Steps
// ============================================================================

#[then(regex = r#"^the segment should be "([^"]+)"$"#)]
fn then_segment(world: &mut PathWorld, segment: String) {
    let path = world.path.as_ref().expect("No path");
    assert_eq!(path.segment, segment);
}

#[then(regex = r#"^the field should be (\d+)$"#)]
fn then_field(world: &mut PathWorld, field: usize) {
    let path = world.path.as_ref().expect("No path");
    assert_eq!(path.field, field);
}

#[then(regex = r#"^the repetition should be (\d+)$"#)]
fn then_repetition(world: &mut PathWorld, rep: usize) {
    let path = world.path.as_ref().expect("No path");
    assert_eq!(path.repetition, Some(rep));
}

#[then("the repetition should be None")]
fn then_repetition_none(world: &mut PathWorld) {
    let path = world.path.as_ref().expect("No path");
    assert_eq!(path.repetition, None);
}

#[then(regex = r#"^the component should be (\d+)$"#)]
fn then_component(world: &mut PathWorld, comp: usize) {
    let path = world.path.as_ref().expect("No path");
    assert_eq!(path.component, Some(comp));
}

#[then("the component should be None")]
fn then_component_none(world: &mut PathWorld) {
    let path = world.path.as_ref().expect("No path");
    assert_eq!(path.component, None);
}

#[then(regex = r#"^the subcomponent should be (\d+)$"#)]
fn then_subcomponent(world: &mut PathWorld, sub: usize) {
    let path = world.path.as_ref().expect("No path");
    assert_eq!(path.subcomponent, Some(sub));
}

#[then("the subcomponent should be None")]
fn then_subcomponent_none(world: &mut PathWorld) {
    let path = world.path.as_ref().expect("No path");
    assert_eq!(path.subcomponent, None);
}

#[then("parsing should fail")]
fn then_parsing_fail(world: &mut PathWorld) {
    assert!(world.path_result.as_ref().unwrap().is_err());
}

#[then("the error should indicate invalid format")]
fn then_error_invalid_format(world: &mut PathWorld) {
    match &world.error {
        Some(PathError::InvalidFormat(_)) => (),
        _ => panic!("Expected InvalidFormat error"),
    }
}

#[then("the error should indicate invalid segment ID")]
fn then_error_invalid_segment(world: &mut PathWorld) {
    match &world.error {
        Some(PathError::InvalidSegmentId(_)) => (),
        _ => panic!("Expected InvalidSegmentId error"),
    }
}

#[then("the error should indicate invalid field number")]
fn then_error_invalid_field(world: &mut PathWorld) {
    match &world.error {
        Some(PathError::InvalidFieldNumber(_)) => (),
        _ => panic!("Expected InvalidFieldNumber error"),
    }
}

#[then("the error should indicate invalid component number")]
fn then_error_invalid_component(world: &mut PathWorld) {
    match &world.error {
        Some(PathError::InvalidComponentNumber(_)) => (),
        _ => panic!("Expected InvalidComponentNumber error"),
    }
}

#[then("the error should indicate invalid repetition index")]
fn then_error_invalid_repetition(world: &mut PathWorld) {
    match &world.error {
        Some(PathError::InvalidRepetitionIndex(_)) => (),
        _ => panic!("Expected InvalidRepetitionIndex error"),
    }
}

#[then(regex = r#"^the result should be "([^"]+)"$"#)]
fn then_result_string(world: &mut PathWorld, expected: String) {
    assert_eq!(world.formatted_path, expected);
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    PathWorld::run("features/path.feature").await;
}
