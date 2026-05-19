use super::MessageMetadata;
use serde::{Deserialize, Serialize};

/// ACK generation request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckRequest {
    /// Raw HL7 message content
    pub message: String,
    /// ACK code to generate
    pub code: AckRequestCode,
    /// Optional error text for ERR segment generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Whether the input message is MLLP framed
    #[serde(default)]
    pub mllp_framed: bool,
    /// Whether to MLLP frame the ACK response
    #[serde(default)]
    pub mllp_frame: bool,
}

/// HTTP ACK codes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AckRequestCode {
    /// Application accept
    #[serde(rename = "AA")]
    Aa,
    /// Application error
    #[serde(rename = "AE")]
    Ae,
    /// Application reject
    #[serde(rename = "AR")]
    Ar,
    /// Commit accept
    #[serde(rename = "CA")]
    Ca,
    /// Commit error
    #[serde(rename = "CE")]
    Ce,
    /// Commit reject
    #[serde(rename = "CR")]
    Cr,
}

impl AckRequestCode {
    /// Return the HL7 code string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aa => "AA",
            Self::Ae => "AE",
            Self::Ar => "AR",
            Self::Ca => "CA",
            Self::Ce => "CE",
            Self::Cr => "CR",
        }
    }
}

/// ACK generation response body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckResponse {
    /// Generated ACK message, optionally MLLP framed
    pub ack_message: String,
    /// Generated ACK code
    pub ack_code: String,
    /// Metadata extracted from the generated ACK message
    pub metadata: MessageMetadata,
}

/// Configurable server ACK policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckPolicyConfig {
    /// ACK mode to use when choosing generated ACK/NAK codes.
    #[serde(default)]
    pub mode: AckPolicyMode,
    /// Condition that causes an accept ACK.
    #[serde(default)]
    pub accept_on: AckPolicyAcceptOn,
    /// Conditions that cause reject ACKs.
    #[serde(default = "default_ack_reject_on")]
    pub reject_on: Vec<AckPolicyRejectCondition>,
    /// Whether generated NAKs should include non-PHI error text in `ERR`.
    #[serde(default = "default_include_error_text")]
    pub include_error_text: bool,
}

impl Default for AckPolicyConfig {
    fn default() -> Self {
        Self {
            mode: AckPolicyMode::Original,
            accept_on: AckPolicyAcceptOn::Valid,
            reject_on: default_ack_reject_on(),
            include_error_text: true,
        }
    }
}

impl AckPolicyConfig {
    /// Return whether the policy rejects the supplied condition.
    pub fn rejects(&self, condition: AckPolicyRejectCondition) -> bool {
        self.reject_on.contains(&condition)
    }
}

fn default_ack_reject_on() -> Vec<AckPolicyRejectCondition> {
    vec![
        AckPolicyRejectCondition::ParseError,
        AckPolicyRejectCondition::ValidationError,
    ]
}

fn default_include_error_text() -> bool {
    true
}

/// ACK policy mode.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AckPolicyMode {
    /// Use original mode application ACK codes: `AA` and `AR`.
    #[default]
    Original,
    /// Use enhanced mode commit ACK codes: `CA` and `CR`.
    Enhanced,
}

/// ACK policy accept condition.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AckPolicyAcceptOn {
    /// Accept only after the message validates against the supplied profile.
    #[default]
    Valid,
}

/// ACK policy reject condition.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AckPolicyRejectCondition {
    /// Reject when the inbound message cannot be parsed enough to validate.
    ParseError,
    /// Reject when validation against the supplied profile fails.
    ValidationError,
}

/// Policy-driven ACK request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckPolicyRequest {
    /// Raw HL7 message content.
    pub message: String,
    /// Inline profile YAML content used for validation before deciding ACK/NAK.
    pub profile: String,
    /// Whether the input message is MLLP framed.
    #[serde(default)]
    pub mllp_framed: bool,
    /// Whether to MLLP frame the ACK response.
    #[serde(default)]
    pub mllp_frame: bool,
}

/// Policy-driven ACK response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckPolicyResponse {
    /// Generated ACK or NAK message, optionally MLLP framed.
    pub ack_message: String,
    /// Generated ACK code.
    pub ack_code: String,
    /// Decision details used to choose the ACK code.
    pub decision: AckPolicyDecision,
    /// Validation report used for the decision, when the message parsed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_report: Option<hl7v2::ValidationReport>,
    /// Metadata extracted from the generated ACK message.
    pub metadata: MessageMetadata,
}

/// ACK policy decision details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckPolicyDecision {
    /// Configured ACK mode.
    pub mode: AckPolicyMode,
    /// Decision outcome.
    pub outcome: AckPolicyOutcome,
    /// Reason for the decision.
    pub reason: AckPolicyReason,
    /// Generated ACK code.
    pub ack_code: String,
    /// Whether non-PHI error text was included in the ACK.
    pub include_error_text: bool,
    /// Error text included in the ACK, when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_text: Option<String>,
}

/// ACK policy decision outcome.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AckPolicyOutcome {
    /// Message was accepted.
    Accepted,
    /// Message was rejected.
    Rejected,
}

/// ACK policy decision reason.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AckPolicyReason {
    /// Message parsed and validated successfully.
    Valid,
    /// Message could not be parsed.
    ParseError,
    /// Message parsed but failed profile validation.
    ValidationError,
}

