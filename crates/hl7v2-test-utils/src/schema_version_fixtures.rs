//! Deterministic schema-version parity fixtures.

use std::sync::OnceLock;

/// Shared manifest that ties schema-version parity tests together.
pub const SCHEMA_VERSION_PARITY_MANIFEST: &str =
    include_str!("../../../test_data/evidence/schema-version-parity.json");

static SCHEMA_VERSION_PARITY_FIXTURE: OnceLock<Result<SchemaVersionParityFixture, String>> =
    OnceLock::new();

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SchemaVersionParityFixture {
    pub schema_version: String,
    pub v2_report_schema_version: u8,
    pub expected_v2_schema_version: String,
    pub unsupported_report_schema_version: u8,
    pub unsupported_error_contains: String,
    pub tool_names: SchemaVersionToolNames,
    pub validation: SchemaVersionValidationFixture,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SchemaVersionToolNames {
    pub cli: String,
    pub rest: String,
    pub grpc: String,
    pub python: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SchemaVersionValidationFixture {
    pub message_type: String,
    pub profile_label: String,
    pub profile_version: String,
    pub required_issue_code: String,
    pub required_issue_path: String,
}

pub fn schema_version_parity_fixture() -> Result<&'static SchemaVersionParityFixture, String> {
    match SCHEMA_VERSION_PARITY_FIXTURE.get_or_init(|| {
        serde_json::from_str(SCHEMA_VERSION_PARITY_MANIFEST).map_err(|err| err.to_string())
    }) {
        Ok(fixture) => Ok(fixture),
        Err(err) => Err(err.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::schema_version_parity_fixture;

    #[test]
    fn test_schema_version_parity_fixture_records_expected_surfaces() -> Result<(), String> {
        let fixture = schema_version_parity_fixture()?;

        assert_eq!(fixture.schema_version, "1.0");
        assert_eq!(fixture.v2_report_schema_version, 2);
        assert_eq!(fixture.expected_v2_schema_version, "2");
        assert_eq!(fixture.unsupported_report_schema_version, 3);
        assert!(fixture.unsupported_error_contains.contains("1 or 2"));
        assert_eq!(fixture.tool_names.cli, "hl7v2-cli");
        assert_eq!(fixture.tool_names.rest, "hl7v2-server");
        assert_eq!(fixture.tool_names.grpc, "hl7v2-server-grpc");
        assert_eq!(fixture.tool_names.python, "hl7v2-python");
        assert_eq!(fixture.validation.message_type, "ADT^A01");
        assert_eq!(
            fixture.validation.required_issue_code,
            "missing_required_field"
        );

        Ok(())
    }
}
