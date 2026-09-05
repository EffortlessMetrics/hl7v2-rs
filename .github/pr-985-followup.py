from pathlib import Path
from textwrap import dedent


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one occurrence, found {count}")
    return text.replace(old, new, 1)


cli_path = Path("crates/hl7v2-cli/src/evidence_bundle.rs")
cli = cli_path.read_text()
cli = replace_once(
    cli,
    "use std::collections::{BTreeMap, BTreeSet};",
    "use std::collections::BTreeMap;",
    "CLI collection imports",
)
cli = replace_once(
    cli,
    dedent(
        '''\
        #[derive(serde::Deserialize)]
        struct SafeAnalysisPolicy {
            rules: Vec<SafeAnalysisPolicyRule>,
        }

        #[derive(serde::Deserialize)]
        struct SafeAnalysisPolicyRule {
            path: String,
            action: RedactionAction,
            #[serde(default)]
            reason: Option<String>,
            #[serde(default)]
            optional: bool,
        }

        '''
    ),
    "",
    "CLI duplicate policy types",
)
cli = replace_once(
    cli,
    dedent(
        '''\
            let redaction_policy = policy::load_safe_analysis_policy(&policy_text)?;
            let receipt = policy::apply_safe_analysis_policy(&mut message, &redaction_policy)?;
        '''
    ),
    dedent(
        '''\
            let receipt = policy::redact_message(&mut message, &policy_text)?;
        '''
    ),
    "CLI redact command adapter",
)
cli = replace_once(
    cli,
    dedent(
        '''\
            let redaction_policy = policy::load_safe_analysis_policy(&policy_text)?;

            let mut redacted_message = message.clone();
            let redaction_receipt =
                policy::apply_safe_analysis_policy(&mut redacted_message, &redaction_policy)?;
        '''
    ),
    dedent(
        '''\
            let mut redacted_message = message.clone();
            let redaction_receipt = policy::redact_message(&mut redacted_message, &policy_text)?;
        '''
    ),
    "CLI bundle command adapter",
)

module_marker = "\nmod policy {"
marker_index = cli.find(module_marker)
if marker_index < 0:
    raise SystemExit("CLI policy module marker not found")

cli_module = dedent(
    r'''
    mod policy {
        //! Canonical safe-analysis execution plus CLI-only field-path tracing.

        use super::*;

        pub(super) fn redact_message(
            message: &mut Message,
            policy_text: &str,
        ) -> Result<RedactionReceipt, Box<dyn std::error::Error>> {
            let receipt = hl7v2::redact::redact_message_safe_analysis(message, policy_text)?;

            Ok(RedactionReceipt {
                phi_removed: receipt.phi_removed,
                hash_algorithm: "sha256",
                actions: receipt
                    .actions
                    .into_iter()
                    .map(|receipt| {
                        let action = map_action(receipt.action);
                        let status = map_status(receipt.status);
                        RedactionActionReceipt {
                            path: receipt.path,
                            action,
                            reason: receipt.reason,
                            matched_count: receipt.matched_count,
                            optional: receipt.optional,
                            status,
                        }
                    })
                    .collect(),
            })
        }

        fn map_action(action: hl7v2::redact::RedactionAction) -> RedactionAction {
            match action {
                hl7v2::redact::RedactionAction::Hash => RedactionAction::Hash,
                hl7v2::redact::RedactionAction::Drop => RedactionAction::Drop,
                hl7v2::redact::RedactionAction::Retain => RedactionAction::Retain,
            }
        }

        fn map_status(status: hl7v2::redact::RedactionActionStatus) -> RedactionActionStatus {
            match status {
                hl7v2::redact::RedactionActionStatus::Applied => RedactionActionStatus::Applied,
                hl7v2::redact::RedactionActionStatus::Retained => RedactionActionStatus::Retained,
                hl7v2::redact::RedactionActionStatus::NotFound => RedactionActionStatus::NotFound,
            }
        }

        pub(super) fn message_field_text(
            message: &Message,
            segment_id: &str,
            field_index: usize,
        ) -> Option<String> {
            let field_index = modeled_field_index(segment_id, field_index)?;
            let field = message
                .segments
                .iter()
                .find(|segment| segment.id_str() == segment_id)?
                .fields
                .get(field_index)?;
            Some(field_to_text(field, &message.delims))
        }

        pub(super) fn build_field_path_trace(
            message: &Message,
            receipt: &RedactionReceipt,
        ) -> FieldPathTraceReport {
            let redaction_actions: Vec<(&str, RedactionAction)> = receipt
                .actions
                .iter()
                .map(|action| (action.path.as_str(), action.action))
                .collect();
            let mut fields = Vec::new();
            let mut segment_occurrences = BTreeMap::<String, usize>::new();

            for (segment_position, segment) in message.segments.iter().enumerate() {
                let segment_index = segment_position.saturating_add(1);
                let segment_occurrence = {
                    let count = segment_occurrences
                        .entry(segment.id_str().to_string())
                        .or_insert(0);
                    *count = count.saturating_add(1);
                    *count
                };
                for (modeled_index, field) in segment.fields.iter().enumerate() {
                    let field_index = hl7_field_index(segment.id_str(), modeled_index);
                    let canonical_path = format!("{}.{}", segment.id_str(), field_index);
                    let occurrence_path = format!(
                        "{}[{}].{}",
                        segment.id_str(),
                        segment_occurrence,
                        field_index
                    );
                    let field_text = field_to_text(field, &message.delims);
                    fields.push(FieldPathTrace {
                        path: occurrence_path.clone(),
                        canonical_path: canonical_path.clone(),
                        segment_index,
                        field_index,
                        present: !field_text.is_empty(),
                        value_shape: field_value_shape(&field_text),
                        redaction_action: redaction_action_for_field(
                            &redaction_actions,
                            &occurrence_path,
                            &canonical_path,
                        ),
                    });
                }
            }

            FieldPathTraceReport {
                message_type: message_field_text(message, "MSH", 9)
                    .unwrap_or_else(|| "unknown".into()),
                field_count: fields.len(),
                fields,
            }
        }

        fn redaction_action_for_field(
            actions: &[(&str, RedactionAction)],
            occurrence_path: &str,
            canonical_path: &str,
        ) -> Option<RedactionAction> {
            actions.iter().find_map(|(action_path, action)| {
                (path_targets_field(action_path, occurrence_path)
                    || path_targets_field(action_path, canonical_path))
                .then_some(*action)
            })
        }

        fn path_targets_field(action_path: &str, field_path: &str) -> bool {
            if action_path == field_path {
                return true;
            }

            action_path
                .strip_prefix(field_path)
                .is_some_and(|suffix| suffix.starts_with('.') || suffix.starts_with('['))
        }

        fn hl7_field_index(segment_id: &str, modeled_index: usize) -> usize {
            if segment_id == "MSH" {
                modeled_index.saturating_add(2)
            } else {
                modeled_index.saturating_add(1)
            }
        }

        fn field_value_shape(field_text: &str) -> FieldValueShape {
            if field_text.is_empty() {
                FieldValueShape::Empty
            } else if field_text.starts_with("hash:sha256:") {
                FieldValueShape::HashedSha256
            } else {
                FieldValueShape::Present
            }
        }

        fn modeled_field_index(segment_id: &str, field_index: usize) -> Option<usize> {
            if segment_id == "MSH" {
                field_index.checked_sub(2)
            } else {
                field_index.checked_sub(1)
            }
        }

        fn field_to_text(field: &Field, delims: &hl7v2::Delims) -> String {
            field
                .reps
                .iter()
                .map(|rep| rep_to_text(rep, delims))
                .collect::<Vec<_>>()
                .join(&delims.rep.to_string())
        }

        fn rep_to_text(rep: &Rep, delims: &hl7v2::Delims) -> String {
            rep.comps
                .iter()
                .map(|comp| comp_to_text(comp, delims))
                .collect::<Vec<_>>()
                .join(&delims.comp.to_string())
        }

        fn comp_to_text(comp: &Comp, delims: &hl7v2::Delims) -> String {
            comp.subs
                .iter()
                .map(atom_to_text)
                .collect::<Vec<_>>()
                .join(&delims.sub.to_string())
        }

        fn atom_to_text(atom: &Atom) -> &str {
            match atom {
                Atom::Text(text) => text.as_str(),
                Atom::Null => "\"\"",
            }
        }
    }
    '''
).lstrip()
cli_path.write_text(cli[:marker_index] + "\n" + cli_module)

server_path = Path("crates/hl7v2-server/src/redaction.rs")
server_path.write_text(
    dedent(
        r'''
        //! Safe-analysis redaction adapter for HTTP and gRPC evidence surfaces.
        //!
        //! Policy parsing, safety validation, target traversal, mutation, and receipt
        //! semantics belong to `hl7v2`. This module only converts the canonical receipt
        //! into the server's public evidence model.

        use crate::models::{
            RedactionAction, RedactionActionReceipt, RedactionActionStatus, RedactionReceipt,
        };
        use hl7v2::Message;

        /// Apply a safe-analysis policy to a message and return a server receipt.
        pub fn redact_message(
            message: &mut Message,
            policy_text: &str,
        ) -> Result<RedactionReceipt, String> {
            let receipt = hl7v2::redact::redact_message_safe_analysis(message, policy_text)
                .map_err(|error| error.to_string())?;

            Ok(RedactionReceipt {
                phi_removed: receipt.phi_removed,
                hash_algorithm: receipt.hash_algorithm,
                actions: receipt
                    .actions
                    .into_iter()
                    .map(|receipt| {
                        let action = map_action(receipt.action);
                        let status = map_status(receipt.status);
                        RedactionActionReceipt {
                            path: receipt.path,
                            action,
                            reason: receipt.reason,
                            matched_count: receipt.matched_count,
                            optional: receipt.optional,
                            status,
                        }
                    })
                    .collect(),
            })
        }

        fn map_action(action: hl7v2::redact::RedactionAction) -> RedactionAction {
            match action {
                hl7v2::redact::RedactionAction::Hash => RedactionAction::Hash,
                hl7v2::redact::RedactionAction::Drop => RedactionAction::Drop,
                hl7v2::redact::RedactionAction::Retain => RedactionAction::Retain,
            }
        }

        fn map_status(status: hl7v2::redact::RedactionActionStatus) -> RedactionActionStatus {
            match status {
                hl7v2::redact::RedactionActionStatus::Applied => RedactionActionStatus::Applied,
                hl7v2::redact::RedactionActionStatus::Retained => RedactionActionStatus::Retained,
                hl7v2::redact::RedactionActionStatus::NotFound => RedactionActionStatus::NotFound,
            }
        }

        #[cfg(test)]
        mod tests {
            use super::{RedactionActionStatus, redact_message};
            use std::io;

            type TestResult = Result<(), Box<dyn std::error::Error>>;

            fn require(condition: bool, message: &'static str) -> TestResult {
                if condition {
                    Ok(())
                } else {
                    Err(io::Error::other(message).into())
                }
            }

            fn policy(path: &str) -> String {
                format!(
                    r#"
        [[rules]]
        path = "{path}"
        action = "drop"
        reason = "remove observation component"
        "#
                )
            }

            #[test]
            fn omitted_field_repetition_uses_canonical_all_repetition_semantics() -> TestResult {
                let mut message = hl7v2::parse(
                    b"MSH|^~\\&|SEND|FAC|RECV|FAC|202601010000||ORU^R01|CTRL|P|2.5\rOBX|1|TX|CODE||first^left~second^right",
                )?;
                let receipt = redact_message(&mut message, &policy("OBX.5.1"))
                    .map_err(io::Error::other)?;
                let output = String::from_utf8(hl7v2::write(&message))?;

                require(
                    output.contains("OBX|1|TX|CODE||^left~^right"),
                    "server adapter did not redact the component in every field repetition",
                )?;
                require(
                    !output.contains("first") && !output.contains("second"),
                    "server adapter leaked a targeted repetition",
                )?;
                let action = receipt
                    .actions
                    .first()
                    .ok_or_else(|| io::Error::other("missing redaction receipt action"))?;
                require(
                    action.matched_count == 1,
                    "receipt count must remain segment-based",
                )?;
                require(
                    action.status == RedactionActionStatus::Applied,
                    "expected applied receipt status",
                )
            }

            #[test]
            fn explicit_field_repetition_remains_narrow() -> TestResult {
                let mut message = hl7v2::parse(
                    b"MSH|^~\\&|SEND|FAC|RECV|FAC|202601010000||ORU^R01|CTRL|P|2.5\rOBX|1|TX|CODE||first^left~second^right",
                )?;
                let receipt = redact_message(&mut message, &policy("OBX.5[2].1"))
                    .map_err(io::Error::other)?;
                let output = String::from_utf8(hl7v2::write(&message))?;

                require(
                    output.contains("OBX|1|TX|CODE||first^left~^right"),
                    "server adapter widened an explicit field-repetition selector",
                )?;
                let action = receipt
                    .actions
                    .first()
                    .ok_or_else(|| io::Error::other("missing redaction receipt action"))?;
                require(
                    action.matched_count == 1,
                    "explicit selector receipt count must remain segment-based",
                )
            }
        }
        '''
    ).lstrip()
)

contract_paths = [
    "api/proto/hl7v2/v1/hl7v2.proto",
    "crates/hl7v2-server/proto/hl7v2/v1/hl7v2.proto",
    "schemas/evidence/redaction-receipt-v1.schema.json",
    "schemas/evidence/redaction-receipt-v2.schema.json",
    "api/openapi/hl7v2-api-v1.yaml",
    "crates/hl7v2-server/openapi/hl7v2-api-v1.yaml",
]
contract_variants = {
    "// Number of matching values affected by this action": (
        "// Number of matching segments containing a selected target.\n"
        "  // Multiple field repetitions in one segment count once."
    ),
    '"description": "Number of matching values affected by the action."': (
        '"description": "Number of matching segments containing a selected target; '
        'multiple field repetitions in one segment count once."'
    ),
    "description: Number of matching values affected by this action": (
        "description: Number of matching segments containing a selected target; "
        "multiple field repetitions in one segment count once"
    ),
    "description: Number of matching values affected by the action": (
        "description: Number of matching segments containing a selected target; "
        "multiple field repetitions in one segment count once"
    ),
}
for path_text in contract_paths:
    path = Path(path_text)
    text = path.read_text()
    for old, new in contract_variants.items():
        text = text.replace(old, new)
    path.write_text(text)

stale = [path for path in contract_paths if "matching values affected" in Path(path).read_text()]
if stale:
    raise SystemExit("stale matched_count descriptions remain in: " + ", ".join(stale))

test_path = Path("crates/hl7v2-cli/tests/redaction_repetition_test.rs")
test_path.write_text(
    dedent(
        r'''
        //! Cross-surface regression coverage for field-repetition redaction.

        mod common;

        use serde_json::Value;
        use std::fs;
        use std::io;
        use std::path::{Path, PathBuf};
        use tempfile::TempDir;

        type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

        const MESSAGE: &str = "MSH|^~\\&|SEND|FAC|RECV|FAC|202601010000||ORU^R01|CTRL|P|2.5\rOBX|1|TX|CODE||first^left~second^right";
        const PROFILE: &str = r#"
        message_structure: "ORU_R01"
        version: "2.5.1"
        segments:
          - id: "MSH"
          - id: "OBX"
        "#;

        fn require(condition: bool, message: &'static str) -> TestResult {
            if condition {
                Ok(())
            } else {
                Err(io::Error::other(message).into())
            }
        }

        fn write_input(dir: &TempDir, path: &str) -> TestResult<(PathBuf, PathBuf, PathBuf)> {
            let message_path = dir.path().join("message.hl7");
            let policy_path = dir.path().join("policy.toml");
            let profile_path = dir.path().join("profile.yaml");
            fs::write(&message_path, MESSAGE)?;
            fs::write(
                &policy_path,
                format!(
                    r#"
        [[rules]]
        path = "{path}"
        action = "drop"
        reason = "remove observation component"
        "#
                ),
            )?;
            fs::write(&profile_path, PROFILE)?;
            Ok((message_path, policy_path, profile_path))
        }

        fn run_redact(message: &Path, policy: &Path) -> TestResult<(String, u64)> {
            let output = common::cli_command()
                .arg("redact")
                .arg(message)
                .arg("--policy")
                .arg(policy)
                .arg("--format")
                .arg("json")
                .output()?;
            if !output.status.success() {
                return Err(io::Error::other(format!(
                    "redact command failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
                .into());
            }
            let json: Value = serde_json::from_slice(&output.stdout)?;
            let redacted_hl7 = json
                .get("redacted_hl7")
                .and_then(Value::as_str)
                .ok_or_else(|| io::Error::other("redact output omitted redacted_hl7"))?
                .to_string();
            let matched_count = json
                .pointer("/receipt/actions/0/matched_count")
                .and_then(Value::as_u64)
                .ok_or_else(|| io::Error::other("redact output omitted matched_count"))?;
            Ok((redacted_hl7, matched_count))
        }

        fn run_bundle(
            message: &Path,
            policy: &Path,
            profile: &Path,
            out: &Path,
        ) -> TestResult<(String, u64)> {
            let output = common::cli_command()
                .arg("support-bundle")
                .arg(message)
                .arg("--profile")
                .arg(profile)
                .arg("--redact-policy")
                .arg(policy)
                .arg("--out")
                .arg(out)
                .output()?;
            if !output.status.success() {
                return Err(io::Error::other(format!(
                    "support-bundle command failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
                .into());
            }
            let redacted_hl7 = fs::read_to_string(out.join("message.redacted.hl7"))?;
            let receipt: Value =
                serde_json::from_slice(&fs::read(out.join("redaction-receipt.json"))?)?;
            let matched_count = receipt
                .pointer("/actions/0/matched_count")
                .and_then(Value::as_u64)
                .ok_or_else(|| io::Error::other("bundle receipt omitted matched_count"))?;
            Ok((redacted_hl7, matched_count))
        }

        fn assert_all_repetitions(output: &str, matched_count: u64) -> TestResult {
            require(
                output.contains("OBX|1|TX|CODE||^left~^right"),
                "omitted repetition selector did not redact every repetition",
            )?;
            require(
                !output.contains("first") && !output.contains("second"),
                "omitted selector leaked a targeted component",
            )?;
            require(
                matched_count == 1,
                "matched_count must count the containing segment once",
            )
        }

        fn assert_explicit_repetition(output: &str, matched_count: u64) -> TestResult {
            require(
                output.contains("OBX|1|TX|CODE||first^left~^right"),
                "explicit selector did not remain narrow",
            )?;
            require(output.contains("first"), "explicit selector changed repetition one")?;
            require(!output.contains("second"), "explicit selector leaked repetition two")?;
            require(
                matched_count == 1,
                "matched_count must count the containing segment once",
            )
        }

        #[test]
        fn redact_command_redacts_all_repetitions_when_selector_is_omitted() -> TestResult {
            let dir = TempDir::new()?;
            let (message, policy, _) = write_input(&dir, "OBX.5.1")?;
            let (output, matched_count) = run_redact(&message, &policy)?;
            assert_all_repetitions(&output, matched_count)
        }

        #[test]
        fn redact_command_keeps_explicit_repetition_narrow() -> TestResult {
            let dir = TempDir::new()?;
            let (message, policy, _) = write_input(&dir, "OBX.5[2].1")?;
            let (output, matched_count) = run_redact(&message, &policy)?;
            assert_explicit_repetition(&output, matched_count)
        }

        #[test]
        fn support_bundle_redacts_all_repetitions_when_selector_is_omitted() -> TestResult {
            let dir = TempDir::new()?;
            let (message, policy, profile) = write_input(&dir, "OBX.5.1")?;
            let out = dir.path().join("bundle");
            let (output, matched_count) = run_bundle(&message, &policy, &profile, &out)?;
            assert_all_repetitions(&output, matched_count)
        }

        #[test]
        fn support_bundle_keeps_explicit_repetition_narrow() -> TestResult {
            let dir = TempDir::new()?;
            let (message, policy, profile) = write_input(&dir, "OBX.5[2].1")?;
            let out = dir.path().join("bundle");
            let (output, matched_count) = run_bundle(&message, &policy, &profile, &out)?;
            assert_explicit_repetition(&output, matched_count)
        }
        '''
    ).lstrip()
)
