//! Deterministic security-oriented test fixtures.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::OnceLock;

/// Shared manifest that ties PHI sentinel and safe-error parity tests together.
pub const SAFE_ERROR_PHI_PARITY_MANIFEST: &str =
    include_str!("../../../test_data/security/safe-error-phi-parity.json");

static SAFE_ERROR_PHI_PARITY_FIXTURE: OnceLock<Result<SafeErrorPhiParityFixture, String>> =
    OnceLock::new();

/// Synthetic HL7 message containing deterministic PHI leak sentinels.
///
/// The values are intentionally memorable and fake. Tests use this fixture to
/// prove redaction/evidence outputs do not echo PHI-bearing fields.
pub const PHI_LEAK_SENTINEL_MESSAGE: &str = "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||MRN-777-ALPHA^^^HOSP^MR||Signal^Patricia||19661224|M|||742 Evergreen Terrace||5558675309\rNK1|1|Watcher^Nora||900 Support Way|5550001234\rOBX|1|NM|718-7^Hemoglobin^LN||13.2|g/dL\r";

/// Safe-analysis policy that protects every PHI sentinel in
/// [`PHI_LEAK_SENTINEL_MESSAGE`] while retaining non-PHI analysis shape.
pub const PHI_LEAK_SENTINEL_POLICY: &str = r#"
[[rules]]
path = "PID.3"
action = "hash"
reason = "patient identifier"

[[rules]]
path = "PID.5"
action = "drop"
reason = "patient name"

[[rules]]
path = "PID.7"
action = "drop"
reason = "date of birth"

[[rules]]
path = "PID.11"
action = "drop"
reason = "patient address"

[[rules]]
path = "PID.13"
action = "drop"
reason = "patient phone"

[[rules]]
path = "NK1.2"
action = "drop"
reason = "next of kin name"

[[rules]]
path = "NK1.4"
action = "drop"
reason = "next of kin address"

[[rules]]
path = "NK1.5"
action = "drop"
reason = "next of kin phone"

[[rules]]
path = "MSH.9"
action = "retain"
reason = "message type is needed for analysis"

[[rules]]
path = "MSH.10"
action = "retain"
reason = "control id is needed for replay correlation"

[[rules]]
path = "OBX.3"
action = "retain"
reason = "observation identifier is needed for analysis"

[[rules]]
path = "OBX.5"
action = "retain"
reason = "non-PHI synthetic observation value shape is needed for analysis"
"#;

/// File name used to catch raw input path leakage in generated evidence.
pub const RAW_INPUT_FILE_SENTINEL: &str = "raw-phi-input-sentinel.hl7";

/// File name used to catch raw policy path leakage in generated evidence.
pub const RAW_POLICY_FILE_SENTINEL: &str = "raw-policy-sentinel.toml";

/// PHI-bearing values that must not appear in receipts, traces, reports, logs,
/// replay output, or metadata.
pub const PHI_LEAK_SENTINELS: &[(&str, &str)] = &[
    ("patient name", "Signal^Patricia"),
    ("MRN", "MRN-777-ALPHA^^^HOSP^MR"),
    ("date of birth", "19661224"),
    ("address", "742 Evergreen Terrace"),
    ("phone", "5558675309"),
    ("next of kin name", "Watcher^Nora"),
    ("next of kin address", "900 Support Way"),
    ("next of kin phone", "5550001234"),
];

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SafeErrorPhiParityFixture {
    pub schema_version: String,
    pub phi: PhiFixture,
    pub malformed_message: MalformedMessageFixture,
    pub invalid_profile: InvalidProfileFixture,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PhiFixture {
    pub message: String,
    pub policy: String,
    pub raw_input_file: String,
    pub raw_policy_file: String,
    pub forbidden: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MalformedMessageFixture {
    pub message: String,
    pub expected_error_substrings: Vec<String>,
    pub rest_code: String,
    pub rest_location: String,
    pub rest_safe_detail_contains: String,
    pub rest_action_contains: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InvalidProfileFixture {
    pub yaml: String,
    pub forbidden: Vec<String>,
    pub rest_code: String,
    pub rest_location: String,
    pub rest_safe_detail_contains: String,
    pub rest_action_contains: String,
}

impl SafeErrorPhiParityFixture {
    pub fn forbidden_values(&self) -> impl Iterator<Item = &str> {
        self.phi
            .forbidden
            .iter()
            .chain(self.invalid_profile.forbidden.iter())
            .map(String::as_str)
    }

    pub fn assert_no_forbidden(&self, context: &str, content: &str) {
        for value in self.forbidden_values() {
            assert!(
                !content.contains(value),
                "{context} leaked manifest-forbidden value: {value}"
            );
        }
    }
}

pub fn safe_error_phi_parity_fixture() -> Result<&'static SafeErrorPhiParityFixture, String> {
    match SAFE_ERROR_PHI_PARITY_FIXTURE.get_or_init(|| {
        serde_json::from_str(SAFE_ERROR_PHI_PARITY_MANIFEST).map_err(|err| err.to_string())
    }) {
        Ok(fixture) => Ok(fixture),
        Err(err) => Err(err.clone()),
    }
}

/// Assert that `content` contains none of the shared PHI leak sentinel values.
///
/// This is a regression tripwire for synthetic fixtures, not a general PHI
/// detector.
pub fn assert_no_phi_leak_sentinels(context: &str, content: &str) {
    for (label, value) in PHI_LEAK_SENTINELS {
        assert!(
            !content.contains(value),
            "{context} leaked {label}: {value}"
        );
    }
}

/// Assert that `content` contains neither PHI sentinels nor raw file paths.
pub fn assert_no_phi_leak_sentinels_or_paths(
    context: &str,
    content: &str,
    message_path: &Path,
    policy_path: &Path,
) {
    assert_no_phi_leak_sentinels(context, content);

    let message_path = message_path.to_string_lossy();
    assert!(
        !content.contains(message_path.as_ref()),
        "{context} leaked raw input file path"
    );
    assert!(
        !content.contains(RAW_INPUT_FILE_SENTINEL),
        "{context} leaked raw input file name"
    );
    let policy_path = policy_path.to_string_lossy();
    assert!(
        !content.contains(policy_path.as_ref()),
        "{context} leaked raw policy file path"
    );
    assert!(
        !content.contains(RAW_POLICY_FILE_SENTINEL),
        "{context} leaked raw policy file name"
    );
}

/// Returns a deterministic API key for tests.
///
/// This key is derived from the supplied `seed`, making it stable across test
/// runs without storing raw secrets in source control.
pub fn deterministic_api_key(seed: &str) -> String {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    format!("test-key-{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::{
        PHI_LEAK_SENTINEL_MESSAGE, PHI_LEAK_SENTINEL_POLICY, PHI_LEAK_SENTINELS,
        assert_no_phi_leak_sentinels, deterministic_api_key, safe_error_phi_parity_fixture,
    };

    #[test]
    fn test_deterministic_api_key_is_stable() {
        let first = deterministic_api_key("seed-1");
        let second = deterministic_api_key("seed-1");
        let third = deterministic_api_key("seed-2");

        assert_eq!(first, second);
        assert_ne!(first, third);
    }

    #[test]
    fn test_phi_sentinel_policy_mentions_all_sensitive_paths() {
        for path in [
            "PID.3", "PID.5", "PID.7", "PID.11", "PID.13", "NK1.2", "NK1.4", "NK1.5",
        ] {
            assert!(
                PHI_LEAK_SENTINEL_POLICY.contains(path),
                "sentinel policy is missing {path}"
            );
        }
    }

    #[test]
    fn test_phi_sentinel_assertion_allows_redacted_shape() {
        assert_no_phi_leak_sentinels(
            "redacted shape",
            "MSH|^~\\&|LAB|L|EHR|E|202605030101||ADT^A01|CTRL123|P|2.5\rPID|1||hash:sha256:abc||||M|||",
        );
        assert!(PHI_LEAK_SENTINEL_MESSAGE.contains("Signal^Patricia"));
    }

    #[test]
    fn test_safe_error_phi_parity_fixture_matches_shared_phi_values() -> Result<(), String> {
        let fixture = safe_error_phi_parity_fixture()?;

        assert_eq!(fixture.schema_version, "1.0");
        assert_eq!(fixture.phi.message, PHI_LEAK_SENTINEL_MESSAGE);
        assert_eq!(fixture.phi.policy.trim(), PHI_LEAK_SENTINEL_POLICY.trim());
        assert_eq!(fixture.phi.raw_input_file, super::RAW_INPUT_FILE_SENTINEL);
        assert_eq!(fixture.phi.raw_policy_file, super::RAW_POLICY_FILE_SENTINEL);
        for (_, value) in PHI_LEAK_SENTINELS {
            assert!(
                fixture.phi.forbidden.iter().any(|item| item == value),
                "manifest missing PHI sentinel value: {value}"
            );
        }

        Ok(())
    }
}
