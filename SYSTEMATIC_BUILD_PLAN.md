# Systematic Build-Out Plan: HL7v2-rs to Production

**Version**: 1.0
**Last Updated**: 2025-11-19
**Status**: Active Development

## Executive Summary

This document provides a comprehensive, systematic plan to build out the hl7v2-rs project from its current ~65% completion to a production-ready, enterprise-grade HL7 v2 processing platform. The plan incorporates modern software engineering practices including:

- **Test-Driven Development (TDD)** - Write tests before implementation
- **Behavior-Driven Development (BDD)** - Define behavior specifications
- **Schema-Driven Design** - Use schemas to validate structures
- **Infrastructure-as-Code (IaC)** - Automate deployment and configuration
- **Policy-as-Code (PaC)** - Codify compliance and security policies
- **Continuous Integration/Deployment** - Automated testing and delivery

---

## Current State Analysis

### ✅ What Works (65% Complete)

**Core Parsing (hl7v2-core)**:
- ✅ Event-based streaming parser with `Event` enum
- ✅ Basic message parsing with delimiter handling
- ✅ MLLP framing/unframing (wrap_mllp, parse_mllp)
- ✅ Batch processing (BHS/BTS/FHS/FTS)
- ✅ Escape sequence handling (basic)
- ✅ JSON serialization
- ✅ Field path access API

**Profile Validation (hl7v2-prof)**:
- ✅ Profile inheritance and merging
- ✅ Constraint validation (required, lengths, patterns)
- ✅ HL7 table validation
- ✅ Advanced validation rules (temporal, contextual, cross-field)
- ✅ Advanced data type validation

**Message Generation (hl7v2-gen)**:
- ✅ Template-based generation
- ✅ Deterministic seeding
- ✅ Realistic data generators
- ✅ Error injection
- ✅ Basic corpus management with golden hashes

**CLI (hl7v2-cli)**:
- ✅ parse, val, norm, ack, gen commands
- ✅ Interactive REPL mode
- ✅ JSON output support

**Infrastructure**:
- ✅ CI/CD with GitHub Actions
- ✅ Multi-platform testing (Linux, Windows, macOS)
- ✅ Code coverage tracking
- ✅ Clippy + rustfmt enforcement
- ✅ MSRV checking (Rust 1.89)

### ❌ What's Missing (35% Remaining)

**Critical Gaps (Blocking v1.2)**:
- ❌ Network module (completely stubbed)
- ❌ Zero-copy parsing optimizations
- ❌ Backpressure/bounded channels
- ❌ Memory bounds enforcement
- ❌ Resume parsing across boundaries
- ❌ Remote profile loading (HTTP, S3, GCS)
- ❌ Profile cycle detection
- ❌ Corpus manifest generation
- ❌ Server mode (HTTP/gRPC)
- ❌ CLI flag gaps (--report, --canonical-delims)

**Future Gaps (v1.3+)**:
- ❌ Language bindings (C, Python, JS, Java)
- ❌ Database connectors
- ❌ Message queue integration
- ❌ Cloud service integration
- ❌ Security/compliance features
- ❌ Advanced analytics

---

## Phase 1: Foundation & Infrastructure (Weeks 1-2)

### Objectives
- Establish modern development practices
- Set up schema-driven validation
- Create test infrastructure
- Define acceptance criteria framework

### 1.1 Schema-Driven Design Implementation

**Create JSON Schemas for All Data Structures**:

```bash
mkdir -p schemas/{profile,template,message,config}
```

**Deliverables**:
- `schemas/profile/profile-v1.schema.json` - Profile YAML validation
- `schemas/template/template-v1.schema.json` - Template YAML validation
- `schemas/message/message-v1.schema.json` - Parsed message JSON schema
- `schemas/config/hl7v2-config-v1.schema.json` - CLI/server configuration schema
- `schemas/error/error-v1.schema.json` - Error response schema
- `schemas/manifest/corpus-manifest-v1.schema.json` - Corpus manifest schema

**Implementation**:
```rust
// Add to hl7v2-prof
use schemars::{JsonSchema, schema_for};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Profile {
    pub message_structure: String,
    pub version: String,
    #[serde(default)]
    pub parent: Option<String>,
    pub constraints: Vec<Constraint>,
    // ...
}

// Generate schema at build time
fn main() {
    let schema = schema_for!(Profile);
    std::fs::write(
        "schemas/profile/profile-v1.schema.json",
        serde_json::to_string_pretty(&schema).unwrap()
    ).unwrap();
}
```

**Acceptance Criteria**:
- [ ] All schemas pass JSON Schema validation
- [ ] Schemas are versioned (v1, v2, etc.)
- [ ] Schemas include examples and descriptions
- [ ] CI validates all YAML files against schemas
- [ ] Documentation includes schema references

### 1.2 Behavior-Driven Development (BDD) Framework

**Set Up Cucumber for BDD**:

```toml
# Add to workspace Cargo.toml
[dev-dependencies]
cucumber = "0.20"
```

**Create Feature Files**:

```gherkin
# features/parsing/basic_parsing.feature
Feature: Basic HL7 Message Parsing
  As a healthcare system integrator
  I want to parse HL7 v2 messages
  So that I can extract and validate clinical data

  Scenario: Parse simple ADT message
    Given an HL7 ADT^A01 message with standard delimiters
    When I parse the message
    Then the parsing should succeed
    And the message should have 5 segments
    And the MSH segment should have the correct sending facility

  Scenario: Parse message with custom delimiters
    Given an HL7 message with pipe delimiter "|" and component delimiter "^"
    When I parse the message
    Then the parser should detect the delimiters correctly
    And field parsing should use the detected delimiters

  Scenario: Handle malformed message gracefully
    Given a truncated HL7 message missing the MSH segment
    When I attempt to parse the message
    Then parsing should fail with error code "P_MissingMSH"
    And the error should include byte offset information
```

**Implement Step Definitions**:

```rust
// tests/bdd/steps/parsing.rs
use cucumber::{given, when, then, World};
use hl7v2_core::{parse, Message};

#[derive(Debug, Default, World)]
struct ParsingWorld {
    input: Vec<u8>,
    result: Option<Result<Message, hl7v2_core::Error>>,
}

#[given(expr = "an HL7 ADT^A01 message with standard delimiters")]
async fn given_adt_message(world: &mut ParsingWorld) {
    world.input = include_bytes!("../../test_data/valid/adt_a01.hl7").to_vec();
}

#[when("I parse the message")]
async fn when_parse(world: &mut ParsingWorld) {
    world.result = Some(parse(&world.input));
}

#[then(expr = "the parsing should succeed")]
async fn then_success(world: &mut ParsingWorld) {
    assert!(world.result.as_ref().unwrap().is_ok());
}

#[then(expr = "the message should have {int} segments")]
async fn then_segment_count(world: &mut ParsingWorld, count: usize) {
    let msg = world.result.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(msg.segments.len(), count);
}
```

**Acceptance Criteria**:
- [ ] BDD framework integrated with `cargo test`
- [ ] Feature files cover all user stories from roadmap
- [ ] Step definitions are reusable across features
- [ ] BDD tests run in CI pipeline
- [ ] Living documentation generated from features

### 1.3 Property-Based Testing with Proptest

**Expand Proptest Coverage**:

```rust
// crates/hl7v2-core/src/tests/property_tests.rs
use proptest::prelude::*;
use crate::{parse, write};

proptest! {
    #[test]
    fn prop_parse_never_panics(input in ".*") {
        // Parser should never panic on arbitrary input
        let _ = parse(input.as_bytes());
    }

    #[test]
    fn prop_round_trip_preserves_semantics(
        field_sep in "[|^~\\\\&]",
        segments in prop::collection::vec(any::<String>(), 1..10)
    ) {
        // Build a message, serialize it, parse it, should be identical
        let original = construct_message(field_sep, &segments);
        let serialized = write(&original);
        let parsed = parse(&serialized).unwrap();

        assert_eq!(original.segments.len(), parsed.segments.len());
        assert_eq!(original.delims, parsed.delims);
    }

    #[test]
    fn prop_validation_is_deterministic(
        msg_bytes in prop::collection::vec(any::<u8>(), 100..1000),
        profile_str in ".*"
    ) {
        // Same message + profile should always produce same validation result
        let result1 = validate_with_profile(&msg_bytes, &profile_str);
        let result2 = validate_with_profile(&msg_bytes, &profile_str);
        assert_eq!(result1, result2);
    }
}
```

**Acceptance Criteria**:
- [ ] Property tests cover all core parsing functions
- [ ] Property tests validate invariants (round-trip, determinism)
- [ ] Property tests detect edge cases not covered by unit tests
- [ ] Property tests run as part of CI

### 1.4 Infrastructure-as-Code (IaC) Setup

**Create Deployment Configurations**:

```yaml
# infrastructure/docker-compose.yml
version: '3.8'
services:
  hl7v2-server:
    build:
      context: ..
      dockerfile: infrastructure/Dockerfile
    ports:
      - "8080:8080"   # HTTP
      - "8443:8443"   # HTTPS
      - "2575:2575"   # MLLP
    environment:
      - RUST_LOG=info
      - HL7V2_CONFIG=/etc/hl7v2/config.toml
    volumes:
      - ./config:/etc/hl7v2
      - ./profiles:/var/lib/hl7v2/profiles
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: hl7v2
      POSTGRES_USER: hl7v2
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres-data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data

volumes:
  postgres-data:
  redis-data:
```

```dockerfile
# infrastructure/Dockerfile
FROM rust:1.89-alpine AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p hl7v2-cli

FROM alpine:latest
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/hl7v2 /usr/local/bin/
USER nobody
ENTRYPOINT ["hl7v2"]
CMD ["server", "--config", "/etc/hl7v2/config.toml"]
```

**Kubernetes Deployment**:

```yaml
# infrastructure/k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: hl7v2-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: hl7v2-server
  template:
    metadata:
      labels:
        app: hl7v2-server
    spec:
      containers:
      - name: hl7v2
        image: hl7v2-rs:latest
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 2575
          name: mllp
        env:
        - name: RUST_LOG
          value: info
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

**Acceptance Criteria**:
- [ ] Docker Compose setup works locally
- [ ] Kubernetes manifests deploy successfully
- [ ] Health checks are responsive
- [ ] Metrics are exposed for Prometheus
- [ ] Logs are structured JSON

### 1.5 Policy-as-Code (PaC) Framework

**Open Policy Agent (OPA) Integration**:

```rego
# policies/validation.rego
package hl7v2.validation

# Deny messages without required PID segment
deny[msg] {
    not input.segments[_].id == "PID"
    msg := "Missing required PID segment"
}

# Deny messages with invalid message type
deny[msg] {
    msh := input.segments[_]
    msh.id == "MSH"
    not valid_message_type(msh.fields[8])
    msg := sprintf("Invalid message type: %v", [msh.fields[8]])
}

valid_message_type(msg_type) {
    allowed := ["ADT^A01", "ADT^A04", "ORU^R01", "ORM^O01"]
    msg_type.value == allowed[_]
}

# Enforce PHI redaction in logs
redact_phi[field_path] {
    input.config.log_phi == false
    phi_fields[field_path]
}

phi_fields := {
    "PID.3",  # Patient ID
    "PID.5",  # Patient Name
    "PID.7",  # Date of Birth
    "PID.13", # Phone Number
}
```

**Integration with Validation**:

```rust
// crates/hl7v2-prof/src/policy.rs
use std::process::Command;

pub fn validate_with_policy(message: &Message, policy_file: &str) -> Result<Vec<String>, Error> {
    let msg_json = serde_json::to_string(message)?;

    let output = Command::new("opa")
        .args(&["eval", "-d", policy_file, "-I", "-", "data.hl7v2.validation.deny"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?
        .stdin.unwrap()
        .write_all(msg_json.as_bytes())?;

    // Parse OPA output and return violations
    Ok(vec![])
}
```

**Acceptance Criteria**:
- [ ] OPA policies define all compliance rules
- [ ] Policies are version-controlled
- [ ] Policy tests cover all edge cases
- [ ] Policies run as part of validation pipeline
- [ ] Policy violations include remediation guidance

---

## Phase 2: Core Feature Implementation (Weeks 3-8)

### 2.1 Network Module Implementation (Weeks 3-4)

**Priority**: CRITICAL - Blocks server mode

#### 2.1.1 TCP MLLP Server (Week 3)

**Implementation Steps**:

1. **Add Tokio for async runtime**:
```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
tokio-util = { version = "0.7", features = ["codec"] }
```

2. **Implement MLLP codec**:
```rust
// crates/hl7v2-core/src/network/codec.rs
use tokio_util::codec::{Decoder, Encoder};
use bytes::{BytesMut, Buf};

pub struct MllpCodec;

const START_BLOCK: u8 = 0x0B;  // VT
const END_BLOCK: u8 = 0x1C;     // FS
const CARRIAGE_RETURN: u8 = 0x0D; // CR

impl Decoder for MllpCodec {
    type Item = Vec<u8>;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        // Look for START_BLOCK
        if src[0] != START_BLOCK {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Missing MLLP start block"
            ));
        }

        // Look for END_BLOCK + CR
        if let Some(pos) = src.windows(2).position(|w| w == &[END_BLOCK, CARRIAGE_RETURN]) {
            // Extract message (skip START_BLOCK, include up to END_BLOCK)
            let msg = src[1..pos].to_vec();
            src.advance(pos + 2); // Consume START + message + END + CR
            return Ok(Some(msg));
        }

        Ok(None) // Need more data
    }
}

impl Encoder<Vec<u8>> for MllpCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Vec<u8>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.len() + 3);
        dst.put_u8(START_BLOCK);
        dst.extend_from_slice(&item);
        dst.put_u8(END_BLOCK);
        dst.put_u8(CARRIAGE_RETURN);
        Ok(())
    }
}
```

3. **Implement server**:
```rust
// crates/hl7v2-core/src/network/server.rs
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use futures::{SinkExt, StreamExt};

pub struct MllpServer {
    config: MllpConfig,
}

impl MllpServer {
    pub fn new(config: MllpConfig) -> Self {
        Self { config }
    }

    pub async fn bind(&self, addr: &str) -> Result<(), Error> {
        let listener = TcpListener::bind(addr).await?;
        println!("MLLP server listening on {}", addr);

        loop {
            let (socket, peer_addr) = listener.accept().await?;
            println!("New connection from {}", peer_addr);

            let config = self.config.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, config).await {
                    eprintln!("Connection error: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(socket: TcpStream, config: MllpConfig) -> Result<(), Error> {
    let mut framed = Framed::new(socket, MllpCodec);

    while let Some(result) = framed.next().await {
        match result {
            Ok(msg_bytes) => {
                // Parse message
                let message = parse(&msg_bytes)?;

                // Validate (if profile configured)
                // Generate ACK
                let ack = generate_ack(&message, "AA")?;

                // Send ACK
                framed.send(write(&ack)).await?;
            }
            Err(e) => {
                eprintln!("Frame error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
```

**Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mllp_server_accepts_connection() {
        let server = MllpServer::new(MllpConfig::default());

        tokio::spawn(async move {
            server.bind("127.0.0.1:2575").await.unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect client
        let stream = TcpStream::connect("127.0.0.1:2575").await.unwrap();
        // ... test message exchange
    }
}
```

**Acceptance Criteria**:
- [ ] TCP server binds to configurable port
- [ ] Accepts multiple concurrent connections
- [ ] Decodes MLLP frames correctly
- [ ] Sends ACK responses
- [ ] Handles connection errors gracefully
- [ ] Passes integration tests with real HL7 messages

#### 2.1.2 TLS Support (Week 4)

**Implementation**:
```rust
use tokio_rustls::{TlsAcceptor, rustls};
use std::sync::Arc;

pub async fn create_tls_acceptor(
    cert_path: &str,
    key_path: &str
) -> Result<TlsAcceptor, Error> {
    let certs = rustls_pemfile::certs(&mut BufReader::new(File::open(cert_path)?))
        .collect::<Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut BufReader::new(File::open(key_path)?))
        .unwrap().unwrap();

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}
```

**Acceptance Criteria**:
- [ ] TLS 1.2+ enforcement
- [ ] Certificate validation
- [ ] mTLS support (optional)
- [ ] Integration test with real certificates

### 2.2 Backpressure & Memory Bounds (Week 5)

**Implement Bounded Channels**:

```rust
use tokio::sync::mpsc;

pub struct BoundedStreamParser<R> {
    reader: R,
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
    capacity: usize,
}

impl<R: AsyncRead + Unpin> BoundedStreamParser<R> {
    pub fn new(reader: R, capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(capacity);
        Self { reader, tx, rx, capacity }
    }

    pub async fn next_event(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    async fn parse_loop(&mut self) {
        // Parse and send events, backpressure when channel full
        while let Some(event) = self.parse_next_event().await {
            if self.tx.send(event).await.is_err() {
                break; // Receiver dropped
            }
        }
    }
}
```

**Memory Tracking**:

```rust
#[cfg(test)]
mod memory_tests {
    use std::alloc::{GlobalAlloc, System, Layout};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TrackingAllocator;

    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

    unsafe impl GlobalAlloc for TrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
            System.alloc(layout)
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
            System.dealloc(ptr, layout)
        }
    }

    #[global_allocator]
    static GLOBAL: TrackingAllocator = TrackingAllocator;

    #[test]
    fn test_memory_bounded() {
        ALLOCATED.store(0, Ordering::SeqCst);

        let large_corpus = vec![0u8; 10 * 1024 * 1024 * 1024]; // 10GB
        let mut parser = StreamParser::new(&large_corpus[..], Delimiters::default());

        while let Some(_) = parser.next_event().unwrap() {
            // Parse entire corpus
        }

        let peak = ALLOCATED.load(Ordering::SeqCst);
        assert!(peak < 64 * 1024 * 1024, "Peak memory {} exceeds 64MB", peak);
    }
}
```

**Acceptance Criteria**:
- [ ] Bounded channel prevents unbounded memory growth
- [ ] Backpressure works in server mode (429 responses when full)
- [ ] Memory tests pass with 10GB corpus < 64MB RSS
- [ ] Performance within acceptable bounds

### 2.3 Server Mode HTTP/gRPC (Weeks 6-7)

**HTTP Server with Axum**:

```rust
// crates/hl7v2-cli/src/server/mod.rs
use axum::{
    Router,
    routing::{post, get},
    extract::{State, Json},
    http::StatusCode,
};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    profiles: Arc<ProfileCache>,
}

pub async fn run_server(config: ServerConfig) -> Result<(), Error> {
    let state = AppState {
        profiles: Arc::new(ProfileCache::new()),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/hl7/parse", post(parse_handler))
        .route("/hl7/validate", post(validate_handler))
        .route("/hl7/ack", post(ack_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    println!("HTTP server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn parse_handler(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match parse(body.as_bytes()) {
        Ok(msg) => Ok(Json(to_json(&msg))),
        Err(e) => {
            eprintln!("Parse error: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn validate_handler(
    State(state): State<AppState>,
    Json(req): Json<ValidationRequest>,
) -> Result<Json<ValidationResponse>, StatusCode> {
    let msg = parse(req.message.as_bytes())
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let profile = state.profiles.get(&req.profile_name)
        .ok_or(StatusCode::NOT_FOUND)?;

    let issues = validate(&msg, &profile);

    Ok(Json(ValidationResponse { issues }))
}
```

**gRPC with Tonic**:

```protobuf
// proto/hl7v2.proto
syntax = "proto3";
package hl7v2;

service HL7Service {
  rpc Parse(ParseRequest) returns (ParseResponse);
  rpc Validate(ValidateRequest) returns (ValidateResponse);
  rpc GenerateAck(AckRequest) returns (AckResponse);
  rpc StreamParse(stream ParseRequest) returns (stream ParseResponse);
}

message ParseRequest {
  bytes message = 1;
  bool mllp_framed = 2;
}

message ParseResponse {
  string json = 1;
  repeated Error errors = 2;
}
```

```rust
// Build gRPC server
tonic::transport::Server::builder()
    .add_service(HL7ServiceServer::new(service))
    .serve(addr)
    .await?;
```

**Acceptance Criteria**:
- [ ] HTTP server handles parse/validate/ack endpoints
- [ ] gRPC server implements proto definitions
- [ ] Streaming requests/responses work
- [ ] Health/readiness endpoints respond correctly
- [ ] Authentication middleware validates tokens
- [ ] RBAC enforces authorization
- [ ] Stress test handles 1000+ concurrent requests

---

## Phase 3: Advanced Features (Weeks 9-12)

### 3.1 Remote Profile Loading (Week 9)

**HTTP/HTTPS Fetcher**:
```rust
use reqwest::Client;
use std::time::Duration;

pub async fn fetch_profile_http(url: &str) -> Result<Profile, Error> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client.get(url)
        .header("If-None-Match", get_cached_etag(url)?)
        .send()
        .await?;

    if response.status() == 304 {
        return load_from_cache(url);
    }

    let etag = response.headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let profile_yaml = response.text().await?;
    let profile: Profile = serde_yaml::from_str(&profile_yaml)?;

    cache_profile(url, &profile, etag)?;
    Ok(profile)
}
```

**Acceptance Criteria**:
- [ ] HTTP/HTTPS fetching with timeout
- [ ] ETag/If-None-Match caching
- [ ] S3/GCS support (AWS SDK, GCS SDK)
- [ ] LRU cache with size limit (100MB default)
- [ ] Cache manifest tracks metadata

### 3.2 Corpus Manifest (Week 10)

```rust
#[derive(Serialize, Deserialize)]
pub struct CorpusManifest {
    pub version: String,  // Tool version
    pub seed: u64,
    pub templates: Vec<TemplateInfo>,
    pub profiles: Vec<ProfileInfo>,
    pub messages: Vec<MessageInfo>,
    pub generated_at: DateTime<Utc>,
}

pub fn generate_manifest(
    templates: &[String],
    profiles: &[String],
    messages: &[PathBuf],
    seed: u64,
) -> Result<CorpusManifest, Error> {
    let version = env!("CARGO_PKG_VERSION").to_string();

    let templates = templates.iter().map(|t| TemplateInfo {
        path: t.clone(),
        sha256: hash_file(t).unwrap(),
    }).collect();

    let messages = messages.iter().map(|m| MessageInfo {
        path: m.clone(),
        sha256: hash_file(m).unwrap(),
        message_type: detect_message_type(m).unwrap(),
    }).collect();

    Ok(CorpusManifest {
        version,
        seed,
        templates,
        profiles,
        messages,
        generated_at: Utc::now(),
    })
}
```

**Acceptance Criteria**:
- [ ] Manifest includes all required metadata
- [ ] SHA-256 hashes for all files
- [ ] `gen --verify-manifest` validates corpus
- [ ] Tamper detection works

---

## Phase 4: Continuous Delivery & Observability (Weeks 13-14)

### 4.1 OpenTelemetry Integration

```rust
use opentelemetry::{global, sdk::Resource, KeyValue};
use opentelemetry_otlp::WithExportConfig;

pub fn init_telemetry(service_name: &str) -> Result<(), Error> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317")
        )
        .with_trace_config(
            opentelemetry::sdk::trace::config()
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", service_name),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ]))
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    global::set_tracer_provider(tracer);
    Ok(())
}

// Instrument functions
#[tracing::instrument]
async fn parse_handler(body: String) -> Result<Message, Error> {
    tracing::info!("Parsing message");
    let msg = parse(body.as_bytes())?;
    tracing::info!("Parsed {} segments", msg.segments.len());
    Ok(msg)
}
```

**Metrics**:
```rust
use opentelemetry::metrics::{Counter, Histogram};

lazy_static! {
    static ref MESSAGES_PARSED: Counter<u64> = global::meter("hl7v2")
        .u64_counter("hl7_messages_parsed_total")
        .with_description("Total messages parsed")
        .init();

    static ref PARSE_DURATION: Histogram<f64> = global::meter("hl7v2")
        .f64_histogram("hl7_parse_duration_ms")
        .with_description("Parse duration in milliseconds")
        .init();
}

pub fn record_parse_metrics(duration_ms: f64) {
    MESSAGES_PARSED.add(1, &[]);
    PARSE_DURATION.record(duration_ms, &[]);
}
```

**Acceptance Criteria**:
- [ ] Metrics exported to Prometheus
- [ ] Traces sent to Jaeger/Tempo
- [ ] Logs structured as JSON
- [ ] PHI redacted by default
- [ ] Grafana dashboard template included

### 4.2 Automated Release Pipeline

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build release
        run: cargo build --release -p hl7v2-cli
      - name: Create archive
        run: tar -czf hl7v2-${{ matrix.os }}.tar.gz target/release/hl7v2
      - name: Upload release
        uses: actions/upload-artifact@v3
        with:
          name: hl7v2-${{ matrix.os }}
          path: hl7v2-${{ matrix.os }}.tar.gz

  publish:
    needs: release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Publish to crates.io
        run: cargo publish --token ${{ secrets.CARGO_TOKEN }}
```

---

## Quality Gates & Definition of Done

### Code Quality
- [ ] All code passes `cargo clippy` with zero warnings
- [ ] All code formatted with `cargo fmt`
- [ ] No unsafe code in public APIs
- [ ] Documentation complete for all public APIs

### Testing
- [ ] Unit test coverage ≥ 90%
- [ ] Integration tests pass on Linux/Windows/macOS
- [ ] Property tests detect no violations
- [ ] BDD scenarios all pass
- [ ] Performance benchmarks meet targets

### Security
- [ ] No critical/high vulnerabilities (`cargo audit`)
- [ ] OPA policies enforce compliance rules
- [ ] PHI redaction tested
- [ ] TLS enforced in production mode

### Performance
- [ ] Parse ≥ 100k msgs/min (small messages)
- [ ] Memory < 64MB RSS for 10GB corpus
- [ ] Server handles 1000+ concurrent connections
- [ ] Sub-millisecond p95 latency

---

## Summary

This systematic build-out plan provides:

1. **Clear phases** with specific deliverables
2. **Modern practices** (TDD, BDD, IaC, PaC)
3. **Acceptance criteria** for every feature
4. **Quality gates** at each phase
5. **Production-ready** infrastructure

The plan takes the project from 65% complete to a fully production-ready, enterprise-grade HL7 v2 processing platform in approximately 14 weeks with a team of 3-4 engineers.

**Next Steps**:
1. Review and approve this plan
2. Create GitHub issues for each phase
3. Assign team members to phases
4. Start Phase 1: Foundation & Infrastructure

---

**Document Control**
Last Updated: 2025-11-19
Author: HL7v2-rs Team
Next Review: Weekly during active development
