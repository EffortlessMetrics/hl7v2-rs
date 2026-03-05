use serde::{Deserialize, Serialize};
use hl7v2_validation::{RuleAction, RuleCondition};

/// Profile loading error types.
///
/// These errors provide detailed information about profile loading failures,
/// making it easier to diagnose configuration and parsing issues.
#[derive(Debug, thiserror::Error)]
pub enum ProfileLoadError {
    /// YAML syntax error during parsing.
    #[error("YAML parse error: {0}")]
    YamlParse(String),

    /// Required field is missing from the profile.
    #[error("Missing required field: {field}")]
    MissingField {
        /// The name of the missing field.
        field: String,
    },

    /// Invalid field value in the profile.
    #[error("Invalid value for field '{field}': {details}")]
    InvalidValue {
        /// The name of the field with an invalid value.
        field: String,
        /// Details about why the value is invalid.
        details: String,
    },

    /// IO error during profile file reading.
    #[error("IO error: {0}")]
    Io(String),

    /// Profile inheritance cycle detected.
    #[error("Profile inheritance cycle detected: {0}")]
    InheritanceCycle(String),

    /// Parent profile not found.
    #[error("Parent profile not found: {0}")]
    ParentNotFound(String),

    /// Network error during remote profile loading.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Profile not found in cache or local filesystem.
    #[error("Profile not found: {0}")]
    NotFound(String),

    /// Invalid URL scheme for remote loading.
    #[error("Invalid URL scheme: {0}. Only http and https are supported.")]
    InvalidScheme(String),

    /// Cache operation failed.
    #[error("Cache error: {0}")]
    Cache(String),

    /// Core HL7 library error.
    #[error("Core error: {0}")]
    Core(String),
}

impl From<serde_yaml::Error> for ProfileLoadError {
    fn from(err: serde_yaml::Error) -> Self {
        ProfileLoadError::YamlParse(err.to_string())
    }
}

impl From<std::io::Error> for ProfileLoadError {
    fn from(err: std::io::Error) -> Self {
        ProfileLoadError::Io(err.to_string())
    }
}

impl From<hl7v2_core::Error> for ProfileLoadError {
    fn from(err: hl7v2_core::Error) -> Self {
        ProfileLoadError::Core(err.to_string())
    }
}

/// A conformance profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub message_structure: String,
    pub version: String,
    #[serde(default)]
    pub message_type: Option<String>,
    #[serde(default)]
    pub parent: Option<String>, // Reference to parent profile by name
    pub segments: Vec<SegmentSpec>,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    #[serde(default)]
    pub lengths: Vec<LengthConstraint>,
    #[serde(default)]
    pub valuesets: Vec<ValueSet>,
    #[serde(default)]
    pub datatypes: Vec<DataTypeConstraint>,
    #[serde(default)]
    pub advanced_datatypes: Vec<AdvancedDataTypeConstraint>, // New field for advanced data type validation
    #[serde(default)]
    pub cross_field_rules: Vec<CrossFieldRule>,
    #[serde(default)]
    pub temporal_rules: Vec<TemporalRule>, // New field for temporal validation
    #[serde(default)]
    pub contextual_rules: Vec<ContextualRule>, // New field for contextual validation
    #[serde(default)]
    pub custom_rules: Vec<CustomRule>,
    #[serde(default)]
    pub hl7_tables: Vec<HL7Table>,
    /// Table precedence order - defines the order in which tables should be checked
    /// when multiple tables could apply to a field
    #[serde(default)]
    pub table_precedence: Vec<String>,
    /// Expression guardrails - rules that limit how expressions can be used in profiles
    #[serde(default)]
    pub expression_guardrails: ExpressionGuardrails,
}

/// Specification for a segment in a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentSpec {
    pub id: String,
}

/// Constraint on a field path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub path: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub components: Option<ComponentConstraint>,
    #[serde(default)]
    pub r#in: Option<Vec<String>>,
    #[serde(default)]
    pub when: Option<Condition>,
    #[serde(default)]
    pub pattern: Option<String>,
}

/// Component constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConstraint {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

/// Conditional constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    #[serde(default)]
    pub eq: Option<Vec<String>>,
    #[serde(default)]
    pub any: Option<Vec<Condition>>,
}

/// Length constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthConstraint {
    pub path: String,
    pub max: Option<usize>,
    pub policy: Option<String>, // "no-truncate" or "may-truncate"
}

/// Value set constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueSet {
    pub path: String,
    pub name: String,
    /// Codes can be defined inline OR reference an HL7 table by name
    #[serde(default)]
    pub codes: Vec<String>,
}

/// Data type constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTypeConstraint {
    pub path: String,
    pub r#type: String, // HL7 data type like "ST", "ID", "DT", etc.
}

/// Advanced data type constraint with complex validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedDataTypeConstraint {
    pub path: String,
    pub r#type: String, // HL7 data type like "ST", "ID", "DT", etc.
    #[serde(default)]
    pub pattern: Option<String>, // Regex pattern for additional validation
    #[serde(default)]
    pub min_length: Option<usize>, // Minimum length constraint
    #[serde(default)]
    pub max_length: Option<usize>, // Maximum length constraint
    #[serde(default)]
    pub format: Option<String>, // Format specification (e.g., "YYYY-MM-DD" for dates)
    #[serde(default)]
    pub checksum: Option<String>, // Checksum algorithm (e.g., "luhn" for credit cards)
}

/// Temporal validation rule for date/time relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalRule {
    pub id: String,
    pub description: String,
    pub before: String, // Path to field that should be before another
    pub after: String,  // Path to field that should be after another
    #[serde(default)]
    pub allow_equal: bool, // Whether equal times are allowed
    #[serde(default)]
    pub tolerance: Option<String>, // Tolerance for comparison (e.g., "1d" for 1 day)
}

/// Contextual validation rule based on message context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualRule {
    pub id: String,
    pub description: String,
    pub context_field: String,   // Field that determines the context
    pub context_value: String,   // Value that triggers this rule
    pub target_field: String,    // Field to validate
    pub validation_type: String, // Type of validation to apply
    #[serde(default)]
    pub parameters: std::collections::HashMap<String, String>, // Additional parameters
}

/// HL7 Table definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HL7Table {
    pub id: String,      // Table ID like "HL70001"
    pub name: String,    // Table name like "Administrative Sex"
    pub version: String, // HL7 version like "2.5.1"
    pub codes: Vec<HL7TableEntry>,
}

/// Entry in an HL7 table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HL7TableEntry {
    pub value: String,       // The code value
    pub description: String, // Description of the code
    #[serde(default)]
    pub status: String, // "A" (active), "D" (deprecated), "R" (restricted)
}

/// Cross-field validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossFieldRule {
    pub id: String,
    pub description: String,
    /// Validation mode: "conditional" (default) or "assert"
    /// - "conditional": If conditions are met, execute actions
    /// - "assert": Conditions must be true, fail otherwise
    #[serde(default = "default_validation_mode")]
    pub validation_mode: String,
    pub conditions: Vec<RuleCondition>,
    pub actions: Vec<RuleAction>,
}

pub(crate) fn default_validation_mode() -> String {
    "conditional".to_string()
}

/// Custom validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    pub id: String,
    pub description: String,
    pub script: String, // Could be a simple expression or reference to external logic
}

/// Expression guardrails - rules that limit how expressions can be used in profiles
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ExpressionGuardrails {
    /// Maximum depth of nested expressions
    #[serde(default)]
    pub max_depth: Option<usize>,
    /// Maximum length of expression strings
    #[serde(default)]
    pub max_length: Option<usize>,
    /// Whether to allow custom scripts
    #[serde(default)]
    pub allow_custom_scripts: bool,
}
