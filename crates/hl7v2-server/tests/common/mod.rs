//! Common test utilities and fixtures for integration tests.

use axum::Router;
use hl7v2_server::server::{AppState, Server, ServerConfig};
use std::sync::Arc;
use std::time::Instant;

/// Create a test server instance with default configuration
pub fn create_test_server() -> Server {
    let config = ServerConfig {
        bind_address: "127.0.0.1:0".to_string(), // Use random port for tests
        max_body_size: 1024 * 1024, // 1MB
        api_key: Some("test-key".to_string()),
    };
    Server::new(config)
}

/// Create a test router for integration testing
pub fn create_test_router() -> Router {
    let metrics_handle = hl7v2_server::metrics::init_metrics_recorder();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        metrics_handle: Arc::new(metrics_handle),
        api_key: Arc::new("test-key".to_string()),
    });
    hl7v2_server::routes::build_router(state)
}

/// Sample HL7v2 messages for testing
pub mod fixtures {
    /// Valid ADT^A01 message (Admit/Visit Notification)
    pub const ADT_A01_VALID: &str =
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG001|P|2.5\r\
         EVN|A01|20231119120000\r\
         PID|1||MRN123^^^Facility^MR||Doe^John^A||19800101|M||||||||||123456789\r\
         PV1|1|I|ICU^101^01||||DOC123^Smith^Jane|||MED||||||||V123456|||||||||||||||||||||||||20231119120000\r";

    /// Valid ADT^A04 message (Register Patient)
    pub const ADT_A04_VALID: &str =
        "MSH|^~\\&|RegSys|Hospital|ADT|Hospital|20231119130000||ADT^A04|MSG002|P|2.5\r\
         EVN|A04|20231119130000\r\
         PID|1||MRN456^^^Hospital^MR||Smith^Jane^M||19900215|F||||||||||987654321\r\
         PV1|1|O|CLINIC^201^01||||DOC456^Johnson^Robert|||||||||||V789012\r";

    /// Valid ORU^R01 message (Lab Results)
    pub const ORU_R01_VALID: &str =
        "MSH|^~\\&|LabSys|Lab|LIS|Hospital|20231119140000||ORU^R01|MSG003|P|2.5\r\
         PID|1||MRN789^^^Lab^MR||Patient^Test||19850610|M\r\
         OBR|1|ORD123|FIL456|CBC^Complete Blood Count|||20231119120000|||||||\
         |||DOC789^Doctor^Chief||||||||F|||||||\r\
         OBX|1|NM|WBC^White Blood Count||7.5|10^9/L|4.0-11.0|N|||F|||20231119130000\r\
         OBX|2|NM|RBC^Red Blood Count||4.8|10^12/L|4.5-5.5|N|||F|||20231119130000\r";

    /// Invalid message (malformed)
    pub const INVALID_MALFORMED: &str =
        "This is not a valid HL7 message";

    /// Invalid message (wrong encoding characters)
    pub const INVALID_ENCODING: &str =
        "MSH|Wrong encoding characters";

    /// Invalid message (missing required fields)
    pub const INVALID_MISSING_FIELDS: &str =
        "MSH|^~\\&||||||||||2.5\r";

    /// Minimal valid message (just MSH)
    pub const MINIMAL_VALID: &str =
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG999|P|2.5\r";
}

/// Sample conformance profiles for testing
pub mod profiles {
    /// Minimal profile (basic structure only)
    pub const MINIMAL_PROFILE: &str = r#"
message_structure: "MINIMAL"
version: "2.5"
description: "Minimal profile for testing"

segments:
  - id: "MSH"
    description: "Message Header"
    required: true
    max_uses: 1
"#;

    /// ADT_A01 profile excerpt (simplified for testing)
    pub const ADT_A01_PROFILE: &str = r#"
message_structure: "ADT_A01"
version: "2.5.1"
description: "ADT^A01 test profile"

segments:
  - id: "MSH"
    description: "Message Header"
    required: true
    max_uses: 1
  - id: "EVN"
    description: "Event Type"
    required: true
    max_uses: 1
  - id: "PID"
    description: "Patient Identification"
    required: true
    max_uses: 1
  - id: "PV1"
    description: "Patient Visit"
    required: true
    max_uses: 1

msh_constraints:
  - field: "MSH.9.1"
    required: true
    values: ["ADT"]
  - field: "MSH.9.2"
    required: true
    values: ["A01"]

field_constraints:
  - path: "PID.3"
    required: true
    description: "Patient ID required"
  - path: "PID.5"
    required: true
    description: "Patient name required"

hl7_tables:
  - id: "HL70001"
    name: "Administrative Sex"
    version: "2.5.1"
    codes:
      - code: "F"
        display: "Female"
      - code: "M"
        display: "Male"
      - code: "U"
        display: "Unknown"

valuesets:
  - path: "PID.8"
    name: "Administrative Sex"
    codes: []
    hl7_table: "HL70001"

expression_guardrails:
  max_depth: 10
  max_length: 1000
  allow_custom_scripts: false
"#;
}
