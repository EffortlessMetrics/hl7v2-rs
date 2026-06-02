use crate::conformance::validation::ValidationReport;
use crate::conformance::validation::{RuleAction, RuleCondition};
use crate::model::Error;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

impl From<Error> for ProfileLoadError {
    fn from(err: Error) -> Self {
        ProfileLoadError::Core(err.to_string())
    }
}

/// A conformance profile
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    /// HL7 message structure name (for example, `ADT_A01`).
    pub message_structure: String,
    /// HL7 version string (for example, `2.5.1`).
    pub version: String,
    /// Optional message type override.
    #[serde(default)]
    pub message_type: Option<String>,
    /// Reference to parent profile by name for profile inheritance.
    #[serde(default)]
    pub parent: Option<String>,
    /// Segment specifications for this profile.
    pub segments: Vec<SegmentSpec>,
    /// Field and primitive constraints.
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    /// Field length constraints.
    #[serde(default)]
    pub lengths: Vec<LengthConstraint>,
    /// HL7 value sets referenced by the profile.
    #[serde(default)]
    pub valuesets: Vec<ValueSet>,
    /// Simple datatype constraints.
    #[serde(default)]
    pub datatypes: Vec<DataTypeConstraint>,
    /// Advanced datatype constraints for richer checks.
    #[serde(default)]
    pub advanced_datatypes: Vec<AdvancedDataTypeConstraint>,
    /// Cross-field validation rules.
    #[serde(default)]
    pub cross_field_rules: Vec<CrossFieldRule>,
    /// Temporal validation rules for date/time comparisons.
    #[serde(default)]
    pub temporal_rules: Vec<TemporalRule>,
    /// Contextual validation rules based on message context.
    #[serde(default)]
    pub contextual_rules: Vec<ContextualRule>,
    /// Custom extension validation rules.
    #[serde(default)]
    pub custom_rules: Vec<CustomRule>,
    /// HL7 table definitions used by value-set checks.
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
    /// Segment identifier (for example, `MSH`).
    pub id: String,
    /// Whether this segment must appear in messages validated by this profile.
    #[serde(default)]
    pub required: bool,
    /// Whether this profile permits repeated occurrences of this segment.
    #[serde(default)]
    pub repetition: bool,
}

/// Constraint on a field path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Path of the constrained field.
    pub path: String,
    /// Whether the field is mandatory.
    #[serde(default)]
    pub required: bool,
    /// Optional component-level constraint.
    #[serde(default)]
    pub components: Option<ComponentConstraint>,
    /// Allowed literal values.
    #[serde(default)]
    pub r#in: Option<Vec<String>>,
    /// Condition under which this constraint is active.
    #[serde(default)]
    pub when: Option<Condition>,
    /// Regex pattern that the field value must match.
    #[serde(default)]
    pub pattern: Option<String>,
}

/// Component constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConstraint {
    /// Minimum number of values/components.
    pub min: Option<usize>,
    /// Maximum number of values/components.
    pub max: Option<usize>,
}

/// Conditional constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Match exactly one of these values.
    #[serde(default)]
    pub eq: Option<Vec<String>>,
    /// Match when any nested condition is satisfied.
    #[serde(default)]
    pub any: Option<Vec<Condition>>,
}

/// Length constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthConstraint {
    /// Path to the constrained field.
    pub path: String,
    /// Optional maximum length.
    pub max: Option<usize>,
    /// Truncation policy (`no-truncate` or `may-truncate`).
    pub policy: Option<String>,
}

/// Value set constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueSet {
    /// Field path where this value set applies.
    pub path: String,
    /// Name of the value set.
    pub name: String,
    /// Codes can be defined inline OR reference an HL7 table by name
    #[serde(default)]
    pub codes: Vec<String>,
}

/// Data type constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTypeConstraint {
    /// Field path constrained by datatype.
    pub path: String,
    /// HL7 datatype identifier (for example, `ST`, `ID`, `DT`).
    pub r#type: String,
}

/// Advanced data type constraint with complex validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedDataTypeConstraint {
    /// Field path constrained by advanced datatype rules.
    pub path: String,
    /// HL7 datatype identifier (for example, `ST`, `ID`, `DT`).
    pub r#type: String,
    /// Optional regex pattern to validate the field.
    #[serde(default)]
    pub pattern: Option<String>,
    /// Minimum length constraint.
    #[serde(default)]
    pub min_length: Option<usize>,
    /// Maximum length constraint.
    #[serde(default)]
    pub max_length: Option<usize>,
    /// Optional format hint (for example, `YYYY-MM-DD`).
    #[serde(default)]
    pub format: Option<String>,
    /// Optional checksum algorithm name.
    #[serde(default)]
    pub checksum: Option<String>,
}

/// Temporal validation rule for date/time relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalRule {
    /// Rule identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Path expected to be earlier than `after`.
    pub before: String,
    /// Path expected to be later than `before`.
    pub after: String,
    /// Whether equal timestamps are allowed.
    #[serde(default)]
    pub allow_equal: bool,
    /// Optional tolerance for comparison.
    #[serde(default)]
    pub tolerance: Option<String>,
}

/// Contextual validation rule based on message context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualRule {
    /// Rule identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Field used to determine applicability.
    pub context_field: String,
    /// Required field value to activate this rule.
    pub context_value: String,
    /// Field validated when the context matches.
    pub target_field: String,
    /// Validation type to execute.
    pub validation_type: String,
    /// Parameters passed to the validator.
    #[serde(default)]
    pub parameters: std::collections::HashMap<String, String>,
}

/// HL7 Table definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HL7Table {
    /// HL7 table identifier (for example, `HL70001`).
    pub id: String,
    /// Table display name.
    pub name: String,
    /// Table version.
    pub version: String,
    /// Table values.
    pub codes: Vec<HL7TableEntry>,
}

/// Entry in an HL7 table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HL7TableEntry {
    /// Code value.
    pub value: String,
    /// Code description.
    pub description: String,
    /// Entry status (`A` active, `D` deprecated, `R` restricted).
    #[serde(default)]
    pub status: String,
}

/// Cross-field validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossFieldRule {
    /// Rule identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Validation mode: "conditional" (default) or "assert"
    /// - "conditional": If conditions are met, execute actions
    /// - "assert": Conditions must be true, fail otherwise
    #[serde(default = "default_validation_mode")]
    pub validation_mode: String,
    /// Conditions that gate this rule.
    pub conditions: Vec<RuleCondition>,
    /// Actions produced when conditions pass.
    pub actions: Vec<RuleAction>,
}

fn default_validation_mode() -> String {
    "conditional".to_string()
}

/// Custom validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    /// Rule identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Rule script or reference to external logic.
    pub script: String,
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

/// Result of linting a profile definition.
///
/// Profile linting checks the profile as configuration, before it is applied to
/// a message. It reports YAML/load failures plus structural issues that the
/// runtime validator would otherwise ignore or only reveal during validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileLintReport {
    /// Whether the profile has no lint errors.
    pub valid: bool,
    /// Number of errors in `issues`.
    pub error_count: usize,
    /// Number of warnings in `issues`.
    pub warning_count: usize,
    /// Total number of issues.
    pub issue_count: usize,
    /// Lint findings.
    pub issues: Vec<ProfileLintIssue>,
}

impl ProfileLintReport {
    pub(super) fn from_issues(issues: Vec<ProfileLintIssue>) -> Self {
        let error_count = issues
            .iter()
            .filter(|issue| issue.severity == ProfileLintSeverity::Error)
            .count();
        let warning_count = issues
            .iter()
            .filter(|issue| issue.severity == ProfileLintSeverity::Warning)
            .count();
        Self {
            valid: error_count == 0,
            error_count,
            warning_count,
            issue_count: issues.len(),
            issues,
        }
    }

    /// Convert this v1 lint report into the explicit v2 evidence contract shape.
    ///
    /// This preserves the default serialized form of [`ProfileLintReport`].
    /// Producers opt into v2 when they are ready to emit embedded provenance.
    #[must_use]
    pub fn to_v2(
        &self,
        tool_name: impl Into<String>,
        tool_version: impl Into<String>,
    ) -> ProfileLintReportV2 {
        ProfileLintReportV2 {
            schema_version: "2".to_string(),
            tool_name: tool_name.into(),
            tool_version: tool_version.into(),
            report: self.clone(),
        }
    }
}

/// Profile lint report v2 with embedded evidence provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileLintReportV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// Producer surface that generated this profile lint report.
    pub tool_name: String,
    /// Producer package version.
    pub tool_version: String,
    /// V1 profile lint report fields.
    #[serde(flatten)]
    pub report: ProfileLintReport,
}

/// Structured explanation of a loaded profile contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainReport {
    /// Profile identifier supplied by the producer, usually a relative path.
    pub profile: String,
    /// SHA-256 of the source profile YAML.
    pub profile_sha256: String,
    /// HL7 message structure declared by the profile.
    pub message_structure: String,
    /// HL7 version declared by the profile.
    pub version: String,
    /// Message type declared by the profile, when present.
    pub message_type: Option<String>,
    /// Parent profile declared by the profile, when present.
    pub parent: Option<String>,
    /// Count summary for the profile contents.
    pub summary: ProfileExplainSummary,
    /// Declared segments.
    pub segments: Vec<ProfileExplainSegment>,
    /// Required field constraints.
    pub required_fields: Vec<ProfileExplainRequiredField>,
    /// Field-level constraints.
    pub field_constraints: Vec<ProfileExplainConstraint>,
    /// Length rules.
    pub length_rules: Vec<ProfileExplainLengthRule>,
    /// Datatype rules.
    pub datatype_rules: Vec<ProfileExplainDatatypeRule>,
    /// Value set references.
    pub value_sets: Vec<ProfileExplainValueSet>,
    /// Advanced rule groups.
    pub rules: ProfileExplainRules,
    /// Embedded HL7 tables.
    pub hl7_tables: Vec<ProfileExplainTable>,
    /// Table precedence configured by the profile.
    pub table_precedence: Vec<String>,
    /// Expression guardrail settings.
    pub expression_guardrails: ProfileExplainExpressionGuardrails,
    /// Lint summary folded into the explain report.
    pub lint: ProfileExplainLintSummary,
}

impl ProfileExplainReport {
    /// Convert this v1 profile explain report into the explicit v2 evidence
    /// contract shape.
    ///
    /// This preserves the default serialized form of [`ProfileExplainReport`].
    /// Producers opt into v2 when they are ready to emit embedded provenance.
    #[must_use]
    pub fn to_v2(
        &self,
        tool_name: impl Into<String>,
        tool_version: impl Into<String>,
    ) -> ProfileExplainReportV2 {
        ProfileExplainReportV2 {
            schema_version: "2".to_string(),
            tool_name: tool_name.into(),
            tool_version: tool_version.into(),
            report: self.clone(),
        }
    }
}

/// Profile explain report v2 with embedded evidence provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainReportV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// Producer surface that generated this profile explain report.
    pub tool_name: String,
    /// Producer package version.
    pub tool_version: String,
    /// V1 profile explain report fields.
    #[serde(flatten)]
    pub report: ProfileExplainReport,
}

/// Count summary for a profile explain report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainSummary {
    /// Number of declared segments.
    pub segment_count: usize,
    /// Number of required field constraints.
    pub required_field_count: usize,
    /// Number of field constraints.
    pub field_constraint_count: usize,
    /// Number of length rules.
    pub length_rule_count: usize,
    /// Number of simple datatype rules.
    pub datatype_rule_count: usize,
    /// Number of advanced datatype rules.
    pub advanced_datatype_rule_count: usize,
    /// Number of value sets.
    pub value_set_count: usize,
    /// Number of cross-field rules.
    pub cross_field_rule_count: usize,
    /// Number of temporal rules.
    pub temporal_rule_count: usize,
    /// Number of contextual rules.
    pub contextual_rule_count: usize,
    /// Number of custom rules.
    pub custom_rule_count: usize,
    /// Number of embedded HL7 tables.
    pub hl7_table_count: usize,
}

/// A segment declared by a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainSegment {
    /// Segment identifier, such as `MSH` or `PID`.
    pub id: String,
}

/// A required field recorded in a profile explain report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainRequiredField {
    /// HL7 path for the required field.
    pub path: String,
    /// Whether the requirement is conditional.
    pub conditional: bool,
}

/// A field constraint recorded in a profile explain report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainConstraint {
    /// HL7 path for the constraint.
    pub path: String,
    /// Whether the field is required.
    pub required: bool,
    /// Whether the constraint is conditional.
    pub conditional: bool,
    /// Minimum component cardinality, when configured.
    pub component_min: Option<usize>,
    /// Maximum component cardinality, when configured.
    pub component_max: Option<usize>,
    /// Count of inline allowed values.
    pub allowed_value_count: usize,
    /// Inline allowed values.
    pub allowed_values: Vec<String>,
    /// Regex pattern constraint, when configured.
    pub pattern: Option<String>,
}

/// A length rule recorded in a profile explain report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainLengthRule {
    /// HL7 path for the length rule.
    pub path: String,
    /// Maximum length, when configured.
    pub max: Option<usize>,
    /// Enforcement policy, when configured.
    pub policy: Option<String>,
}

/// A datatype rule recorded in a profile explain report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainDatatypeRule {
    /// HL7 path for the datatype rule.
    pub path: String,
    /// Datatype name.
    pub datatype: String,
    /// Rule kind, such as `simple` or `advanced`.
    pub kind: String,
    /// Regex pattern, when configured.
    pub pattern: Option<String>,
    /// Minimum length, when configured.
    pub min_length: Option<usize>,
    /// Maximum length, when configured.
    pub max_length: Option<usize>,
    /// Format rule, when configured.
    pub format: Option<String>,
    /// Checksum rule, when configured.
    pub checksum: Option<String>,
}

/// A value-set reference recorded in a profile explain report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainValueSet {
    /// Value-set name.
    pub name: String,
    /// HL7 path constrained by this value set.
    pub path: String,
    /// Source category, such as `inline`, `hl7_table`, or `empty`.
    pub source: String,
    /// Number of inline codes.
    pub inline_code_count: usize,
    /// Number of table codes.
    pub table_code_count: usize,
}

/// Advanced rule groups recorded in a profile explain report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainRules {
    /// Cross-field rules.
    pub cross_field: Vec<ProfileExplainRule>,
    /// Temporal rules.
    pub temporal: Vec<ProfileExplainRule>,
    /// Contextual rules.
    pub contextual: Vec<ProfileExplainRule>,
    /// Custom rules.
    pub custom: Vec<ProfileExplainRule>,
}

/// A named profile rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainRule {
    /// Rule identifier.
    pub id: String,
    /// Human-readable rule description.
    pub description: String,
}

/// An embedded HL7 table recorded in a profile explain report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainTable {
    /// Table identifier.
    pub id: String,
    /// Table name.
    pub name: String,
    /// Table version.
    pub version: String,
    /// Number of codes in the table.
    pub code_count: usize,
}

/// Expression guardrail settings recorded in a profile explain report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainExpressionGuardrails {
    /// Maximum expression depth, when configured.
    pub max_depth: Option<usize>,
    /// Maximum expression length, when configured.
    pub max_length: Option<usize>,
    /// Whether custom scripts are allowed.
    pub allow_custom_scripts: bool,
}

/// Lint summary embedded in a profile explain report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExplainLintSummary {
    /// Whether linting reported no errors.
    pub valid: bool,
    /// Number of lint errors.
    pub error_count: usize,
    /// Number of lint warnings.
    pub warning_count: usize,
    /// Total lint issue count.
    pub issue_count: usize,
    /// Ignored or unsupported configuration warnings.
    pub ignored_or_unsupported: Vec<ProfileLintIssue>,
}

/// Machine-readable report for profile fixture tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileTestReport {
    /// Profile identifier supplied by the producer, usually a path.
    pub profile: String,
    /// Fixture root identifier supplied by the producer, usually a path.
    pub fixtures: String,
    /// Whether all fixture cases passed.
    pub valid: bool,
    /// Number of fixture cases.
    pub case_count: usize,
    /// Number of passing fixture cases.
    pub passed_count: usize,
    /// Number of failing fixture cases.
    pub failed_count: usize,
    /// Per-fixture results.
    pub cases: Vec<ProfileTestCaseReport>,
}

impl ProfileTestReport {
    /// Convert this v1 profile test report into the explicit v2 evidence
    /// contract shape.
    ///
    /// This preserves the default serialized form of [`ProfileTestReport`].
    /// Producers opt into v2 when they are ready to emit embedded provenance.
    #[must_use]
    pub fn to_v2(
        &self,
        tool_name: impl Into<String>,
        tool_version: impl Into<String>,
    ) -> ProfileTestReportV2 {
        ProfileTestReportV2 {
            schema_version: "2".to_string(),
            tool_name: tool_name.into(),
            tool_version: tool_version.into(),
            report: self.clone(),
        }
    }
}

/// Profile test report v2 with embedded evidence provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileTestReportV2 {
    /// Evidence artifact schema version.
    pub schema_version: String,
    /// Producer surface that generated this profile test report.
    pub tool_name: String,
    /// Producer package version.
    pub tool_version: String,
    /// V1 profile test report fields.
    #[serde(flatten)]
    pub report: ProfileTestReport,
}

/// A single profile fixture test result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileTestCaseReport {
    /// Fixture name, usually relative to the fixture root.
    pub name: String,
    /// Fixture path label supplied by the producer.
    pub path: String,
    /// Expected fixture outcome.
    pub expectation: ProfileFixtureExpectation,
    /// Whether the fixture satisfied its expectation.
    pub passed: bool,
    /// Human-readable fixture result.
    pub message: String,
    /// Validation report generated for the fixture, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_report: Option<ValidationReport>,
    /// Expected report comparison result, when an expected report was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_report: Option<ExpectedReportComparison>,
}

/// Expected fixture outcome.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileFixtureExpectation {
    /// Fixture is expected to validate successfully.
    Valid,
    /// Fixture is expected to fail validation.
    Invalid,
}

impl ProfileFixtureExpectation {
    /// Return the lowercase expectation string used by text output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Invalid => "invalid",
        }
    }
}

/// Result of comparing a generated validation report to an expected report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedReportComparison {
    /// Expected report path label supplied by the producer.
    pub path: String,
    /// Whether the expected report matched.
    pub matched: bool,
    /// Optional mismatch detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub(super) enum ExpectedReportCandidate {
    File(PathBuf),
    Ambiguous(PathBuf),
}

/// A single profile lint finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileLintIssue {
    /// Stable lint code.
    pub code: String,
    /// Finding severity.
    pub severity: ProfileLintSeverity,
    /// Profile path or YAML location, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Human-readable explanation.
    pub message: String,
}

impl ProfileLintIssue {
    pub(super) fn error(code: &str, path: Option<String>, message: String) -> Self {
        Self {
            code: code.to_string(),
            severity: ProfileLintSeverity::Error,
            path,
            message,
        }
    }

    pub(super) fn warning(code: &str, path: Option<String>, message: String) -> Self {
        Self {
            code: code.to_string(),
            severity: ProfileLintSeverity::Warning,
            path,
            message,
        }
    }
}

/// Profile lint finding severity.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileLintSeverity {
    /// Profile should not be used until fixed.
    Error,
    /// Profile loads, but contains surprising or ignored configuration.
    Warning,
}

impl ProfileLintSeverity {
    /// Return the lowercase severity string used by text output.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}
