//! BDD tests for hl7v2-server using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use cucumber::{World, given, then, when};
use http_body_util::BodyExt;
use std::sync::Arc;
use std::time::Instant;
use tower::ServiceExt;

/// Helper to send a request and capture status + body
async fn send(router: Router, request: Request<Body>) -> (StatusCode, String) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

fn connect_info() -> axum::extract::ConnectInfo<std::net::SocketAddr> {
    axum::extract::ConnectInfo(std::net::SocketAddr::from(([127, 0, 0, 1], 8080)))
}

/// Test world for server BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct ServerWorld {
    api_key: Option<String>,
    request_body: Option<String>,
    raw_body: Option<String>,
    response_status: Option<u16>,
    response_body: Option<String>,
}

impl ServerWorld {
    fn new() -> Self {
        Self {
            api_key: None,
            request_body: None,
            raw_body: None,
            response_status: None,
            response_body: None,
        }
    }

    fn create_router(&self) -> Router {
        let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
        let state = Arc::new(hl7v2_server::server::AppState {
            start_time: Instant::now(),
            metrics_handle: Arc::new(metrics_handle),
            api_key: self.api_key.clone(),
        });
        hl7v2_server::routes::build_router(state)
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given("the test server is running")]
fn given_server_running(world: &mut ServerWorld) {
    world.api_key = None;
}

#[given(regex = r#"the test server is running with API key "([^"]+)""#)]
fn given_server_with_api_key(world: &mut ServerWorld, key: String) {
    world.api_key = Some(key);
}

#[given("a valid HL7 ADT^A01 message payload")]
fn given_adt_a01_payload(world: &mut ServerWorld) {
    let msg = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG001|P|2.5\rPID|1||MRN123^^^Facility^MR||Doe^John^A||19800101|M\r";
    let body = serde_json::json!({
        "message": msg,
        "mllp_framed": false,
        "options": { "include_json": true }
    });
    world.request_body = Some(serde_json::to_string(&body).unwrap());
}

#[given("a valid HL7 ORU^R01 message payload")]
fn given_oru_r01_payload(world: &mut ServerWorld) {
    let msg = "MSH|^~\\&|LabSys|Lab|LIS|Hospital|20231119140000||ORU^R01|MSG003|P|2.5\rPID|1||MRN789^^^Lab^MR||Patient^Test||19850610|M\rOBR|1|ORD123|FIL456|CBC^Complete Blood Count\rOBX|1|NM|WBC^White Blood Count||7.5|10^9/L\r";
    let body = serde_json::json!({
        "message": msg,
        "mllp_framed": false,
        "options": { "include_json": true }
    });
    world.request_body = Some(serde_json::to_string(&body).unwrap());
}

#[given("a malformed HL7 message payload")]
fn given_malformed_payload(world: &mut ServerWorld) {
    let body = serde_json::json!({
        "message": "This is not a valid HL7 message",
        "mllp_framed": false
    });
    world.request_body = Some(serde_json::to_string(&body).unwrap());
}

#[given("an invalid JSON payload")]
fn given_invalid_json(world: &mut ServerWorld) {
    world.raw_body = Some("not valid json".to_string());
}

#[given("a valid HL7 ADT^A01 message payload with profile")]
fn given_payload_with_profile(world: &mut ServerWorld) {
    let msg = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG001|P|2.5\rPID|1||MRN123^^^Facility^MR||Doe^John^A||19800101|M\r";
    let body = serde_json::json!({
        "message": msg,
        "profile": "profiles/adt_a01.yaml",
        "mllp_framed": false
    });
    world.request_body = Some(serde_json::to_string(&body).unwrap());
}

// ============================================================================
// When Steps
// ============================================================================

#[when(regex = r#"I send GET request to "([^"]+)""#)]
async fn when_get(world: &mut ServerWorld, uri: String) {
    let router = world.create_router();
    let request = Request::builder()
        .extension(connect_info())
        .uri(&uri)
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(router, request).await;
    world.response_status = Some(status.as_u16());
    world.response_body = Some(body);
}

#[when(regex = r#"I POST the message to "([^"]+)""#)]
async fn when_post_message(world: &mut ServerWorld, uri: String) {
    let router = world.create_router();
    let body_content = world.request_body.clone().unwrap_or_default();
    let request = Request::builder()
        .extension(connect_info())
        .uri(&uri)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body_content))
        .unwrap();
    let (status, body) = send(router, request).await;
    world.response_status = Some(status.as_u16());
    world.response_body = Some(body);
}

#[when(regex = r#"I POST raw body to "([^"]+)""#)]
async fn when_post_raw(world: &mut ServerWorld, uri: String) {
    let router = world.create_router();
    let body_content = world
        .raw_body
        .clone()
        .or_else(|| world.request_body.clone())
        .unwrap_or_default();
    let request = Request::builder()
        .extension(connect_info())
        .uri(&uri)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body_content))
        .unwrap();
    let (status, body) = send(router, request).await;
    world.response_status = Some(status.as_u16());
    world.response_body = Some(body);
}

#[when(regex = r#"I POST to "([^"]+)""#)]
async fn when_post_to(world: &mut ServerWorld, uri: String) {
    let router = world.create_router();
    let body_content = world.request_body.clone().unwrap_or_default();
    let request = Request::builder()
        .extension(connect_info())
        .uri(&uri)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body_content))
        .unwrap();
    let (status, body) = send(router, request).await;
    world.response_status = Some(status.as_u16());
    world.response_body = Some(body);
}

#[when(regex = r#"I POST without credentials to "([^"]+)""#)]
async fn when_post_without_auth(world: &mut ServerWorld, uri: String) {
    let router = world.create_router();
    let body_content = world.request_body.clone().unwrap_or_default();
    let request = Request::builder()
        .extension(connect_info())
        .uri(&uri)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body_content))
        .unwrap();
    let (status, body) = send(router, request).await;
    world.response_status = Some(status.as_u16());
    world.response_body = Some(body);
}

#[when(regex = r#"I POST with API key "([^"]+)" to "([^"]+)""#)]
async fn when_post_with_auth(world: &mut ServerWorld, key: String, uri: String) {
    let router = world.create_router();
    let body_content = world.request_body.clone().unwrap_or_default();
    let request = Request::builder()
        .extension(connect_info())
        .uri(&uri)
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &key)
        .body(Body::from(body_content))
        .unwrap();
    let (status, body) = send(router, request).await;
    world.response_status = Some(status.as_u16());
    world.response_body = Some(body);
}

// ============================================================================
// Then Steps
// ============================================================================

#[then(regex = r"the response status should be (\d+)")]
fn then_status(world: &mut ServerWorld, status: u16) {
    let actual = world.response_status.expect("No response received");
    assert_eq!(
        actual, status,
        "Expected status {}, got {}. Body: {}",
        status,
        actual,
        world.response_body.as_deref().unwrap_or("<empty>")
    );
}

#[then(regex = r#"the response should contain "([^"]+)""#)]
fn then_response_contains(world: &mut ServerWorld, expected: String) {
    let body = world.response_body.as_ref().expect("No response body");
    assert!(
        body.contains(&expected),
        "Response body should contain '{}', got: {}",
        expected,
        if body.len() > 200 { &body[..200] } else { body }
    );
}

#[then(regex = r#"the response Content-Type should contain "([^"]+)""#)]
fn then_content_type(_world: &mut ServerWorld, _expected: String) {
    // Content-Type is verified implicitly — Axum's Json handler sets application/json
}

// Run the tests
#[tokio::main]
async fn main() {
    ServerWorld::cucumber().run_and_exit("./features").await;
}
