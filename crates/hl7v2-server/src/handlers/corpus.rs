use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::handlers::error::AppError;
use crate::models::CorpusMessageInput;

pub(super) fn validated_corpus_message_ids(
    messages: &[CorpusMessageInput],
    field_name: &str,
    default_prefix: &str,
) -> Result<Vec<String>, AppError> {
    if messages.is_empty() {
        return Err(AppError::Validation(format!(
            "{field_name} must contain at least one message"
        )));
    }

    messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            let label = message
                .id
                .clone()
                .unwrap_or_else(|| format!("{default_prefix}-{}", index.saturating_add(1)));
            validate_corpus_message_id(&label)?;
            Ok(label)
        })
        .collect()
}

pub(super) fn validate_corpus_message_id(label: &str) -> Result<(), AppError> {
    if label.is_empty() || label == "." || label == ".." || label.len() > 128 {
        return Err(AppError::Validation(
            "corpus message id must be 1-128 characters and cannot be '.' or '..'".to_string(),
        ));
    }

    if !label
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(AppError::Validation(
            "corpus message id must use only ASCII letters, numbers, '.', '_' or '-'".to_string(),
        ));
    }

    Ok(())
}

pub(super) fn corpus_message_refs<'a>(
    messages: &'a [CorpusMessageInput],
    ids: &'a [String],
) -> Vec<hl7v2::synthetic::corpus::CorpusMessageRef<'a>> {
    messages
        .iter()
        .zip(ids.iter())
        .map(|(message, id)| {
            hl7v2::synthetic::corpus::CorpusMessageRef::new(id.as_str(), message.message.as_bytes())
        })
        .collect()
}

pub(super) fn attach_profile_to_fingerprint(
    fingerprint: &mut hl7v2::synthetic::corpus::CorpusFingerprint,
    profile_yaml: &str,
    messages: &[CorpusMessageInput],
) -> Result<hl7v2::synthetic::corpus::CorpusFingerprintProfile, AppError> {
    let profile = hl7v2::load_profile_checked(profile_yaml).map_err(AppError::from)?;
    let metadata = hl7v2::synthetic::corpus::CorpusFingerprintProfile {
        path: "<inline-profile>".to_string(),
        sha256: compute_sha256(profile_yaml),
        version: profile.version.clone(),
        message_structure: profile.message_structure.clone(),
    };
    fingerprint.profile = Some(metadata.clone());
    fingerprint.validation_issue_code_counts =
        validation_issue_counts_for_loaded_profile(messages, &profile);
    Ok(metadata)
}

pub(super) fn validation_issue_counts_for_messages(
    messages: &[CorpusMessageInput],
    profile_yaml: &str,
) -> Result<Vec<hl7v2::synthetic::corpus::CorpusCount>, AppError> {
    let profile = hl7v2::load_profile_checked(profile_yaml).map_err(AppError::from)?;
    Ok(validation_issue_counts_for_loaded_profile(
        messages, &profile,
    ))
}

pub(super) fn validation_issue_counts_for_loaded_profile(
    messages: &[CorpusMessageInput],
    profile: &hl7v2::Profile,
) -> Vec<hl7v2::synthetic::corpus::CorpusCount> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();

    for message in messages {
        let parsed = if hl7v2::is_mllp_framed(message.message.as_bytes()) {
            hl7v2::parse_mllp(message.message.as_bytes())
        } else {
            hl7v2::parse(message.message.as_bytes())
        };
        let Ok(parsed) = parsed else {
            continue;
        };

        let issues = hl7v2::validate(&parsed, profile);
        let report = hl7v2::ValidationReport::from_issues(
            &parsed,
            Some(profile.message_structure.clone()),
            issues,
        );
        for issue in report.issues {
            let count = counts.entry(issue.code).or_insert(0);
            *count = count.saturating_add(1);
        }
    }

    counts
        .into_iter()
        .map(|(value, count)| hl7v2::synthetic::corpus::CorpusCount { value, count })
        .collect()
}

pub(super) fn validation_report_v2_for_server(
    report: &hl7v2::ValidationReport,
    profile_yaml: &str,
    profile: &hl7v2::Profile,
) -> hl7v2::ValidationReportV2 {
    report.to_v2(
        "hl7v2-server",
        env!("CARGO_PKG_VERSION"),
        Some(hl7v2::ValidationReportProfileIdentity {
            label: profile.message_structure.clone(),
            message_structure: Some(profile.message_structure.clone()),
            version: Some(profile.version.clone()),
            sha256: Some(compute_sha256(profile_yaml)),
        }),
    )
}

pub(super) fn compute_sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message_input(id: Option<&str>, body: &str) -> CorpusMessageInput {
        CorpusMessageInput {
            id: id.map(str::to_string),
            message: body.to_string(),
        }
    }

    #[test]
    fn validate_corpus_message_id_accepts_simple_identifier() {
        validate_corpus_message_id("alpha-001.txt").expect("alpha-001.txt is allowed");
        validate_corpus_message_id("a").expect("single char allowed");
        validate_corpus_message_id("UPPER_lower-1.2").expect("mixed allowed chars");
    }

    #[test]
    fn validate_corpus_message_id_accepts_max_length_128() {
        let label = "a".repeat(128);
        validate_corpus_message_id(&label).expect("128 chars allowed");
    }

    #[test]
    fn validate_corpus_message_id_rejects_129_characters() {
        let label = "a".repeat(129);
        let err = validate_corpus_message_id(&label).expect_err("129 chars must fail");
        assert!(
            matches!(&err, AppError::Validation(m) if m.contains("1-128")),
            "expected Validation mentioning 1-128, got {err:?}"
        );
    }

    #[test]
    fn validate_corpus_message_id_rejects_empty() {
        let err = validate_corpus_message_id("").expect_err("empty id must fail");
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn validate_corpus_message_id_rejects_dot_and_dot_dot() {
        assert!(matches!(
            validate_corpus_message_id("."),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_corpus_message_id(".."),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn validate_corpus_message_id_rejects_leading_or_trailing_whitespace() {
        // Whitespace is outside the allowed `[A-Za-z0-9._-]` set, so the
        // validator must reject both leading and trailing whitespace.
        assert!(matches!(
            validate_corpus_message_id(" leading"),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_corpus_message_id("trailing "),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_corpus_message_id("middle space"),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            validate_corpus_message_id("tab\tchar"),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn validate_corpus_message_id_rejects_disallowed_punctuation() {
        for bad in [
            "msg/1", "msg\\1", "msg:1", "msg?1", "msg*1", "msg+1", "msg=1",
        ] {
            let err =
                validate_corpus_message_id(bad).expect_err("disallowed punctuation must fail");
            assert!(
                matches!(&err, AppError::Validation(m) if m.contains("ASCII")),
                "expected Validation mentioning ASCII for {bad}, got {err:?}"
            );
        }
    }

    #[test]
    fn validate_corpus_message_id_rejects_non_ascii_letters() {
        let err = validate_corpus_message_id("messäge").expect_err("non-ASCII must fail");
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn validated_corpus_message_ids_rejects_empty_list() {
        let err =
            validated_corpus_message_ids(&[], "messages", "msg").expect_err("empty list must fail");
        assert!(
            matches!(&err, AppError::Validation(m) if m.contains("messages") && m.contains("at least one")),
            "expected Validation mentioning field name and 'at least one', got {err:?}"
        );
    }

    #[test]
    fn validated_corpus_message_ids_uses_default_prefix_for_missing_ids() {
        let inputs = vec![
            message_input(None, "msg body 1"),
            message_input(None, "msg body 2"),
        ];
        let ids = validated_corpus_message_ids(&inputs, "messages", "auto")
            .expect("default ids must validate");
        assert_eq!(ids, vec!["auto-1".to_string(), "auto-2".to_string()]);
    }

    #[test]
    fn validated_corpus_message_ids_keeps_user_supplied_ids() {
        let inputs = vec![
            message_input(Some("first.id"), "body"),
            message_input(None, "body"),
            message_input(Some("third-id"), "body"),
        ];
        let ids =
            validated_corpus_message_ids(&inputs, "messages", "auto").expect("ids must validate");
        assert_eq!(
            ids,
            vec![
                "first.id".to_string(),
                "auto-2".to_string(),
                "third-id".to_string(),
            ]
        );
    }

    #[test]
    fn validated_corpus_message_ids_rejects_invalid_supplied_id() {
        let inputs = vec![message_input(Some("bad/id"), "body")];
        let err = validated_corpus_message_ids(&inputs, "messages", "auto")
            .expect_err("invalid id must fail");
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn validated_corpus_message_ids_does_not_deduplicate() {
        // The helper validates each id individually. Duplicates are not rejected
        // here; pin the behavior so future changes are intentional.
        let inputs = vec![
            message_input(Some("same"), "a"),
            message_input(Some("same"), "b"),
        ];
        let ids = validated_corpus_message_ids(&inputs, "messages", "auto")
            .expect("duplicate ids are not rejected by this helper");
        assert_eq!(ids, vec!["same".to_string(), "same".to_string()]);
    }

    #[test]
    fn compute_sha256_produces_stable_hex_digest() {
        // Empty input is a well-known SHA-256 fixture.
        assert_eq!(
            compute_sha256(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        );
        // Same input must produce the same digest each call.
        assert_eq!(compute_sha256("hello"), compute_sha256("hello"));
        // Different inputs must differ.
        assert_ne!(compute_sha256("a"), compute_sha256("b"));
    }
}
