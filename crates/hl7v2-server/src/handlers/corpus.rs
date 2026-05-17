use std::collections::BTreeMap;

use crate::handlers::error::AppError;
use crate::hash::compute_sha256;
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
