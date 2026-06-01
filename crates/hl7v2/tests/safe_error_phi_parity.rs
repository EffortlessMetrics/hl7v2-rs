use std::io;

#[derive(Debug, serde::Deserialize)]
struct SafeErrorPhiParityFixture {
    phi: PhiFixture,
    malformed_message: MalformedMessageFixture,
    invalid_profile: InvalidProfileFixture,
}

#[derive(Debug, serde::Deserialize)]
struct PhiFixture {
    #[cfg(feature = "redact")]
    message: String,
    #[cfg(feature = "redact")]
    policy: String,
    forbidden: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct MalformedMessageFixture {
    message: String,
    expected_error_substrings: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct InvalidProfileFixture {
    yaml: String,
    forbidden: Vec<String>,
}

fn fixture() -> Result<SafeErrorPhiParityFixture, serde_json::Error> {
    serde_json::from_str(include_str!(
        "../../../test_data/security/safe-error-phi-parity.json"
    ))
}

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn test_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    io::Error::other(message.into()).into()
}

fn ensure(condition: bool, message: impl Into<String>) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(test_error(message))
    }
}

fn ensure_no_forbidden(
    fixture: &SafeErrorPhiParityFixture,
    context: &str,
    content: &str,
) -> TestResult {
    for forbidden in fixture
        .phi
        .forbidden
        .iter()
        .chain(fixture.invalid_profile.forbidden.iter())
    {
        ensure(
            !content.contains(forbidden),
            format!("{context} leaked manifest-forbidden value: {forbidden}"),
        )?;
    }
    Ok(())
}

#[test]
fn parse_error_does_not_echo_manifest_phi_sentinels() -> TestResult {
    let fixture = fixture()?;
    let Err(err) = hl7v2::parse(fixture.malformed_message.message.as_bytes()) else {
        return Err(test_error("malformed message parsed successfully"));
    };
    let error_text = err.to_string();

    for expected in &fixture.malformed_message.expected_error_substrings {
        ensure(
            error_text.contains(expected),
            format!("parse error omitted expected safe context: {expected}; got: {error_text}"),
        )?;
    }
    ensure_no_forbidden(&fixture, "Rust parse error", &error_text)?;

    Ok(())
}

#[test]
fn profile_lint_error_does_not_echo_manifest_profile_sentinels() -> TestResult {
    let fixture = fixture()?;
    let report = hl7v2::lint_profile_yaml(&fixture.invalid_profile.yaml);

    ensure(!report.valid, "profile lint report was unexpectedly valid")?;
    ensure(
        report.issues.iter().any(|issue| {
            issue.code == "yaml_parse_error"
                && issue.message.contains("profile YAML could not be parsed")
        }),
        format!("profile lint report did not include sanitized YAML parse error: {report:?}"),
    )?;

    let report_text = serde_json::to_string(&report)?;
    ensure_no_forbidden(&fixture, "Rust profile lint report", &report_text)?;

    Ok(())
}

#[test]
#[cfg(feature = "redact")]
fn redaction_output_does_not_echo_manifest_phi_sentinels() -> TestResult {
    let fixture = fixture()?;
    let output =
        hl7v2::redact::redact_hl7_safe_analysis(&fixture.phi.message, &fixture.phi.policy)?;
    let output_text = serde_json::to_string(&output)?;

    ensure(output.receipt.phi_removed, "redaction did not remove PHI")?;
    ensure(
        output.redacted_hl7.contains("hash:sha256:"),
        "redacted HL7 omitted hash marker",
    )?;
    ensure_no_forbidden(&fixture, "Rust redaction output", &output_text)?;

    Ok(())
}
