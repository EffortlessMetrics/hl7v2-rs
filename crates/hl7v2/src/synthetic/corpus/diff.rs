//! Corpus fingerprint diffing helpers.
//!
//! This module owns before/after comparisons so corpus scanning and
//! fingerprint extraction stay focused on one responsibility.

use super::{
    CorpusCount, CorpusCountDiff, CorpusDiffReport, CorpusError, CorpusFieldCardinality,
    CorpusFieldCardinalityDiff, CorpusFieldPresence, CorpusFieldPresenceDiff, CorpusFingerprint,
    CorpusTotalDiff, CorpusValueShapeStats, CorpusValueShapeStatsDiff, compare_field_paths,
    fingerprint_corpus_path,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Diff two file or directory corpora of HL7 v2 messages.
///
/// This fingerprints both inputs using [`fingerprint_corpus_path`] and returns
/// before/after counts plus signed deltas for stable corpus dimensions.
///
/// # Errors
///
/// Returns [`CorpusError::InvalidConfig`] if either input is neither a regular
/// file nor a directory. Returns [`CorpusError::IoError`] if traversal or file
/// reading fails.
pub fn diff_corpus_paths(
    before: impl AsRef<Path>,
    after: impl AsRef<Path>,
) -> Result<CorpusDiffReport, CorpusError> {
    let before_fingerprint = fingerprint_corpus_path(before)?;
    let after_fingerprint = fingerprint_corpus_path(after)?;
    Ok(diff_corpus_fingerprints(
        &before_fingerprint,
        &after_fingerprint,
    ))
}

/// Diff two already-computed corpus fingerprints.
pub fn diff_corpus_fingerprints(
    before: &CorpusFingerprint,
    after: &CorpusFingerprint,
) -> CorpusDiffReport {
    let message_type_counts = diff_counts(&before.message_type_counts, &after.message_type_counts);
    let segment_counts = diff_counts(&before.segment_counts, &after.segment_counts);

    CorpusDiffReport {
        diff_version: "1".to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        before_root: before.root.clone(),
        after_root: after.root.clone(),
        profile: before.profile.clone().or_else(|| after.profile.clone()),
        file_count: total_diff(before.file_count, after.file_count),
        message_count: total_diff(before.message_count, after.message_count),
        parse_error_count: total_diff(before.parse_error_count, after.parse_error_count),
        new_message_types: new_values(&message_type_counts),
        removed_message_types: removed_values(&message_type_counts),
        new_segments: new_values(&segment_counts),
        removed_segments: removed_values(&segment_counts),
        message_type_counts,
        segment_counts,
        field_presence: diff_field_presence(&before.field_presence, &after.field_presence),
        field_cardinality: diff_field_cardinality(
            &before.field_cardinality,
            &after.field_cardinality,
        ),
        value_shape_stats: diff_value_shape_stats(
            &before.value_shape_stats,
            &after.value_shape_stats,
        ),
        validation_issue_code_counts: diff_counts(
            &before.validation_issue_code_counts,
            &after.validation_issue_code_counts,
        ),
    }
}

fn total_diff(before: usize, after: usize) -> CorpusTotalDiff {
    CorpusTotalDiff {
        before,
        after,
        delta: signed_delta(before, after),
    }
}

fn diff_counts(before: &[CorpusCount], after: &[CorpusCount]) -> Vec<CorpusCountDiff> {
    let before_counts = count_map(before);
    let after_counts = count_map(after);
    let mut values = BTreeSet::new();

    for value in before_counts.keys() {
        values.insert(value.as_str());
    }
    for value in after_counts.keys() {
        values.insert(value.as_str());
    }

    values
        .into_iter()
        .map(|value| {
            let before = before_counts.get(value).copied().unwrap_or_default();
            let after = after_counts.get(value).copied().unwrap_or_default();
            CorpusCountDiff {
                value: value.to_string(),
                before,
                after,
                delta: signed_delta(before, after),
            }
        })
        .filter(|count| count.delta != 0)
        .collect()
}

fn count_map(counts: &[CorpusCount]) -> BTreeMap<String, usize> {
    counts
        .iter()
        .map(|count| (count.value.clone(), count.count))
        .collect()
}

fn new_values(counts: &[CorpusCountDiff]) -> Vec<String> {
    counts
        .iter()
        .filter(|count| count.before == 0 && count.after > 0)
        .map(|count| count.value.clone())
        .collect()
}

fn removed_values(counts: &[CorpusCountDiff]) -> Vec<String> {
    counts
        .iter()
        .filter(|count| count.before > 0 && count.after == 0)
        .map(|count| count.value.clone())
        .collect()
}

fn diff_field_presence(
    before: &[CorpusFieldPresence],
    after: &[CorpusFieldPresence],
) -> Vec<CorpusFieldPresenceDiff> {
    let before_fields = field_presence_map(before);
    let after_fields = field_presence_map(after);
    let mut paths = BTreeSet::new();

    for path in before_fields.keys() {
        paths.insert(path.as_str());
    }
    for path in after_fields.keys() {
        paths.insert(path.as_str());
    }

    let mut diffs: Vec<CorpusFieldPresenceDiff> = paths
        .into_iter()
        .map(|path| {
            let before = before_fields.get(path);
            let after = after_fields.get(path);
            let before_message_count = before.map_or(0, |field| field.message_count);
            let after_message_count = after.map_or(0, |field| field.message_count);
            let before_occurrence_count = before.map_or(0, |field| field.occurrence_count);
            let after_occurrence_count = after.map_or(0, |field| field.occurrence_count);

            CorpusFieldPresenceDiff {
                path: path.to_string(),
                before_message_count,
                after_message_count,
                message_count_delta: signed_delta(before_message_count, after_message_count),
                before_occurrence_count,
                after_occurrence_count,
                occurrence_count_delta: signed_delta(
                    before_occurrence_count,
                    after_occurrence_count,
                ),
            }
        })
        .filter(|field| field.message_count_delta != 0 || field.occurrence_count_delta != 0)
        .collect();
    diffs.sort_by(|left, right| compare_field_paths(&left.path, &right.path));
    diffs
}

fn field_presence_map(fields: &[CorpusFieldPresence]) -> BTreeMap<String, &CorpusFieldPresence> {
    fields
        .iter()
        .map(|field| (field.path.clone(), field))
        .collect()
}

fn diff_field_cardinality(
    before: &[CorpusFieldCardinality],
    after: &[CorpusFieldCardinality],
) -> Vec<CorpusFieldCardinalityDiff> {
    let before_fields = field_cardinality_map(before);
    let after_fields = field_cardinality_map(after);
    let mut paths = BTreeSet::new();

    for path in before_fields.keys() {
        paths.insert(path.as_str());
    }
    for path in after_fields.keys() {
        paths.insert(path.as_str());
    }

    let mut diffs: Vec<CorpusFieldCardinalityDiff> = paths
        .into_iter()
        .map(|path| {
            let before = before_fields.get(path);
            let after = after_fields.get(path);
            let before_min_per_message = before.map_or(0, |field| field.min_per_message);
            let after_min_per_message = after.map_or(0, |field| field.min_per_message);
            let before_max_per_message = before.map_or(0, |field| field.max_per_message);
            let after_max_per_message = after.map_or(0, |field| field.max_per_message);
            let before_total_occurrences = before.map_or(0, |field| field.total_occurrences);
            let after_total_occurrences = after.map_or(0, |field| field.total_occurrences);
            let before_message_count = before.map_or(0, |field| field.message_count);
            let after_message_count = after.map_or(0, |field| field.message_count);

            CorpusFieldCardinalityDiff {
                path: path.to_string(),
                before_min_per_message,
                after_min_per_message,
                min_per_message_delta: signed_delta(before_min_per_message, after_min_per_message),
                before_max_per_message,
                after_max_per_message,
                max_per_message_delta: signed_delta(before_max_per_message, after_max_per_message),
                before_total_occurrences,
                after_total_occurrences,
                total_occurrences_delta: signed_delta(
                    before_total_occurrences,
                    after_total_occurrences,
                ),
                before_message_count,
                after_message_count,
                message_count_delta: signed_delta(before_message_count, after_message_count),
            }
        })
        .filter(|field| {
            field.min_per_message_delta != 0
                || field.max_per_message_delta != 0
                || field.total_occurrences_delta != 0
                || field.message_count_delta != 0
        })
        .collect();
    diffs.sort_by(|left, right| compare_field_paths(&left.path, &right.path));
    diffs
}

fn field_cardinality_map(
    fields: &[CorpusFieldCardinality],
) -> BTreeMap<String, &CorpusFieldCardinality> {
    fields
        .iter()
        .map(|field| (field.path.clone(), field))
        .collect()
}

fn diff_value_shape_stats(
    before: &[CorpusValueShapeStats],
    after: &[CorpusValueShapeStats],
) -> Vec<CorpusValueShapeStatsDiff> {
    let before_shapes = value_shape_stats_map(before);
    let after_shapes = value_shape_stats_map(after);
    let mut paths = BTreeSet::new();

    for path in before_shapes.keys() {
        paths.insert(path.as_str());
    }
    for path in after_shapes.keys() {
        paths.insert(path.as_str());
    }

    let mut diffs: Vec<CorpusValueShapeStatsDiff> = paths
        .into_iter()
        .map(|path| {
            let before = before_shapes.get(path);
            let after = after_shapes.get(path);
            CorpusValueShapeStatsDiff {
                path: path.to_string(),
                coded_count: total_diff(
                    before.map_or(0, |shape| shape.coded_count),
                    after.map_or(0, |shape| shape.coded_count),
                ),
                timestamp_count: total_diff(
                    before.map_or(0, |shape| shape.timestamp_count),
                    after.map_or(0, |shape| shape.timestamp_count),
                ),
                numeric_count: total_diff(
                    before.map_or(0, |shape| shape.numeric_count),
                    after.map_or(0, |shape| shape.numeric_count),
                ),
                null_count: total_diff(
                    before.map_or(0, |shape| shape.null_count),
                    after.map_or(0, |shape| shape.null_count),
                ),
                text_count: total_diff(
                    before.map_or(0, |shape| shape.text_count),
                    after.map_or(0, |shape| shape.text_count),
                ),
            }
        })
        .filter(|shape| {
            shape.coded_count.delta != 0
                || shape.timestamp_count.delta != 0
                || shape.numeric_count.delta != 0
                || shape.null_count.delta != 0
                || shape.text_count.delta != 0
        })
        .collect();
    diffs.sort_by(|left, right| compare_field_paths(&left.path, &right.path));
    diffs
}

fn value_shape_stats_map(
    stats: &[CorpusValueShapeStats],
) -> BTreeMap<String, &CorpusValueShapeStats> {
    stats
        .iter()
        .map(|shape| (shape.path.clone(), shape))
        .collect()
}

fn signed_delta(before: usize, after: usize) -> i128 {
    let before = i128::try_from(before).unwrap_or(i128::MAX);
    let after = i128::try_from(after).unwrap_or(i128::MAX);
    after.saturating_sub(before)
}
